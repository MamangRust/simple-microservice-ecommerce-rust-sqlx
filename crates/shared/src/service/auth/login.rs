use crate::{
    abstract_trait::{DynHashing, DynTokenService, DynUserQueryRepository, LoginServiceTrait},
    cache::CacheStore,
    domain::{
        requests::LoginRequest,
        responses::{ApiResponse, TokenResponse},
    },
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
use chrono::Duration;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::Request;
use tracing::{error, info, warn};

pub struct LoginService {
    hash: DynHashing,
    token_service: DynTokenService,
    query: DynUserQueryRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

pub struct LoginServiceDeps {
    pub hash: DynHashing,
    pub token_service: DynTokenService,
    pub query: DynUserQueryRepository,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub cache_store: Arc<CacheStore>,
}

impl LoginService {
    pub async fn new(deps: LoginServiceDeps) -> Self {
        let LoginServiceDeps {
            hash,
            token_service,
            query,
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
            query,
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
}

#[async_trait]
impl LoginServiceTrait for LoginService {
    async fn login(
        &self,
        request: &LoginRequest,
    ) -> Result<ApiResponse<TokenResponse>, ServiceError> {
        let email = &request.email;
        let password = &request.password;

        let log_msg = format!("🔐 Attempting login for email: {email}");
        info!("{log_msg}");

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

        let failed_attempts_key = format!("auth:login_attempts:{email}");
        let current_attempts = self
            .cache_store
            .get_from_cache::<i32>(&failed_attempts_key)
            .unwrap_or(0);

        if current_attempts >= 5 {
            let msg = "Too many failed login attempts (rate limited)";
            warn!("❌ {}: {}", msg, email);
            self.complete_tracing_error(&tracing_ctx, method, msg).await;
            return Err(ServiceError::Custom(
                "Too many failed attempts. Try again later.".to_string(),
            ));
        }

        let user = match self.query.find_by_email_and_verify(email).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                error!("❌ User not found or not verified: {email}");

                let new_attempts = current_attempts + 1;
                self.cache_store.set_to_cache(
                    &failed_attempts_key,
                    &new_attempts,
                    Duration::minutes(15),
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "User not found or not verified",
                )
                .await;
                return Err(ServiceError::Custom("user not verified".to_string()));
            }
            Err(err) => {
                error!("❌ Failed to query user: {}", err);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        if self
            .hash
            .compare_password(&user.password, password)
            .await
            .is_err()
        {
            error!("❌ Invalid password for user: {email}");

            let new_attempts = current_attempts + 1;
            self.cache_store.set_to_cache(
                &failed_attempts_key,
                &new_attempts,
                Duration::minutes(15),
            );

            self.complete_tracing_error(&tracing_ctx, method.clone(), "Invalid password")
                .await;
            return Err(ServiceError::InvalidCredentials);
        }

        self.cache_store.delete_from_cache(&failed_attempts_key);

        let access_token = match self
            .token_service
            .create_access_token(user.user_id as i32)
            .await
        {
            Ok(token) => token,
            Err(e) => {
                error!("❌ Failed to generate access token: {:?}", e);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate access token",
                )
                .await;
                return Err(e);
            }
        };

        let refresh_token = match self
            .token_service
            .create_refresh_token(user.user_id as i32)
            .await
        {
            Ok(token) => token,
            Err(e) => {
                error!("❌ Failed to generate refresh token: {:?}", e);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate refresh token",
                )
                .await;
                return Err(e);
            }
        };

        let token = TokenResponse {
            access_token,
            refresh_token,
        };

        info!("✅ Login successful for email: {email}");

        self.complete_tracing_success(&tracing_ctx, method, "Login successful")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Login successful".to_string(),
            data: token,
        })
    }
}
