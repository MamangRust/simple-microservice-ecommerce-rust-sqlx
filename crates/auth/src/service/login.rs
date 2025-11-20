use crate::{
    abstract_trait::{
        auth::{LoginServiceTrait, token::DynTokenService},
        grpc_client::user::DynUserGrpcClient,
    },
    domain::{
        requests::auth::AuthRequest,
        response::{api::ApiResponse, token::TokenResponse},
    },
};
use async_trait::async_trait;
use chrono::Duration;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{abstract_trait::DynHashing, errors::AppErrorGrpc};
use shared::{
    cache::CacheStore,
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, Status};
use tracing::{error, info};

pub struct LoginServiceDeps {
    pub hash: DynHashing,
    pub token_service: DynTokenService,
    pub user_client: DynUserGrpcClient,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub cache_store: Arc<CacheStore>,
}

#[derive(Clone)]
pub struct LoginService {
    hash: DynHashing,
    token_service: DynTokenService,
    user_client: DynUserGrpcClient,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl LoginService {
    pub async fn new(deps: LoginServiceDeps) -> Self {
        let LoginServiceDeps {
            hash,
            token_service,
            user_client,
            metrics,
            registry,
            cache_store,
        } = deps;

        registry.lock().await.register(
            "logn_service_request_counter",
            "Total number of requests to the LoginService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "login_service_request_duration",
            "Histogram of request durations for the LoginService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            hash,
            token_service,
            user_client,
            metrics,
            cache_store,
        }
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("login-service")
    }

    fn inject_trace_context<T>(&self, cx: &Context, request: &mut Request<T>) {
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(cx, &mut MetadataInjector(request.metadata_mut()))
        });
    }

    fn start_tracing(&self, operation_name: &str, attributes: Vec<KeyValue>) -> TracingContext {
        let start_time = Instant::now();
        let tracer = self.get_tracer();
        let mut span = tracer
            .span_builder(operation_name.to_string())
            .with_kind(SpanKind::Server)
            .with_attributes(attributes)
            .start(&tracer);

        info!("Starting operation: {operation_name}");

        span.add_event(
            "Operation started",
            vec![
                KeyValue::new("operation", operation_name.to_string()),
                KeyValue::new("timestamp", start_time.elapsed().as_secs_f64().to_string()),
            ],
        );

        let cx = Context::current_with_span(span);
        TracingContext { cx, start_time }
    }

    async fn complete_tracing_success(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, true, message)
            .await;
    }

    async fn complete_tracing_error(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        error_message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, false, error_message)
            .await;
    }

    async fn complete_tracing_internal(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        is_success: bool,
        message: &str,
    ) {
        let status_str = if is_success { "SUCCESS" } else { "ERROR" };
        let status = if is_success {
            StatusUtils::Success
        } else {
            StatusUtils::Error
        };
        let elapsed = tracing_ctx.start_time.elapsed().as_secs_f64();

        tracing_ctx.cx.span().add_event(
            "Operation completed",
            vec![
                KeyValue::new("status", status_str),
                KeyValue::new("duration_secs", elapsed.to_string()),
                KeyValue::new("message", message.to_string()),
            ],
        );

        if is_success {
            info!("✅ Operation completed successfully: {message}");
        } else {
            error!("❌ Operation failed: {message}");
        }

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }

    fn grpc_status_to_service_error(status: Status) -> ServiceError {
        let app_error = AppErrorGrpc::from(status);
        match app_error {
            AppErrorGrpc::Service(service_err) => service_err,
            AppErrorGrpc::Unhandled(msg) => ServiceError::Internal(msg),
        }
    }
}

#[async_trait]
impl LoginServiceTrait for LoginService {
    async fn login(
        &self,
        request: &AuthRequest,
    ) -> Result<ApiResponse<TokenResponse>, ServiceError> {
        let email = &request.email;
        let password = &request.password;

        info!("🔐 Attempting login for email: {email}");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "Login",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", email.to_string()),
            ],
        );

        let mut request = Request::new(request.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let attempts_key = format!("auth:login_attempts:{email}");
        let attempts = self
            .cache_store
            .get_from_cache::<i32>(&attempts_key)
            .unwrap_or(0);

        if attempts >= 5 {
            let msg = "Too many failed login attempts (rate limited)";
            self.complete_tracing_error(&tracing_ctx, method, msg).await;

            return Err(ServiceError::Custom(
                "Too many failed attempts. Try again later.".into(),
            ));
        }

        let user_response = self
            .user_client
            .find_by_email_and_verify(email.clone())
            .await
            .map_err(|err| Self::grpc_status_to_service_error(err.into()))?;

        let user = user_response.data;

        if self
            .hash
            .compare_password(&user.password, password)
            .await
            .is_err()
        {
            let new_attempts = attempts + 1;

            self.cache_store
                .set_to_cache(&attempts_key, &new_attempts, Duration::minutes(15));

            self.complete_tracing_error(&tracing_ctx, method.clone(), "Invalid password")
                .await;

            return Err(ServiceError::InvalidCredentials);
        }

        self.cache_store.delete_from_cache(&attempts_key);

        let uid = user.id as i32;

        let access_token = match self.token_service.create_access_token(uid).await {
            Ok(t) => t,
            Err(e) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate access token",
                )
                .await;
                return Err(e);
            }
        };

        let refresh_token = match self.token_service.create_refresh_token(uid).await {
            Ok(t) => t,
            Err(e) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate refresh token",
                )
                .await;
                return Err(e);
            }
        };

        let tokens = TokenResponse {
            access_token,
            refresh_token,
        };

        self.complete_tracing_success(&tracing_ctx, method, "Login successful")
            .await;

        Ok(ApiResponse {
            status: "success".into(),
            message: "Login successful".into(),
            data: tokens,
        })
    }
}
