use crate::domain::requests::auth::RegisterRequest;
use crate::{
    abstract_trait::{auth::RegisterServiceTrait, grpc_client::user::DynUserGrpcClient},
    domain::{
        requests::{email::EmailRequest, user::CreateUserRequest},
        response::{api::ApiResponse, user::UserResponse},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::errors::grpc_status_to_service_error;
use shared::{
    abstract_trait::DynKafka,
    cache::CacheStore,
    errors::ServiceError,
    utils::{
        EmailTemplateData, MetadataInjector, Method, Metrics, Status as StatusUtils,
        TracingContext, generate_random_string,
    },
};
use std::sync::Arc;
use tokio::time::Instant;
use tonic::Request;
use tracing::{error, info};

pub struct RegisterServiceDeps {
    pub user_client: DynUserGrpcClient,
    pub kafka: DynKafka,
    pub cache_store: Arc<CacheStore>,
}

#[derive(Clone)]
pub struct RegisterService {
    user_client: DynUserGrpcClient,
    kafka: DynKafka,
    metrics: Metrics,
    cache_store: Arc<CacheStore>,
}

impl RegisterService {
    pub fn new(deps: RegisterServiceDeps, registry: &mut Registry) -> Result<Self> {
        let metrics = Metrics::new();

        let RegisterServiceDeps {
            user_client,
            kafka,
            cache_store,
        } = deps;

        registry.register(
            "register_service_request_counter",
            "Total number of requests to the RegisterService",
            metrics.request_counter.clone(),
        );
        registry.register(
            "register_service_request_duration",
            "Histogram of request durations for the RegisterService",
            metrics.request_duration.clone(),
        );

        Ok(Self {
            user_client,
            kafka,
            metrics,
            cache_store,
        })
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("register-service")
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

        self.metrics.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl RegisterServiceTrait for RegisterService {
    async fn register(
        &self,
        req: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!(
            "📝 [REGISTER] Starting user registration | Email: {}",
            req.email
        );

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RegisterUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", req.email.clone()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("auth:register:{}", req.email);

        if let Some(cached) = self.cache_store.get_from_cache::<UserResponse>(&cache_key) {
            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User already registered (from cache)",
            )
            .await;

            return Ok(ApiResponse {
                status: "success".into(),
                message: "User already registered (from cache)".into(),
                data: cached,
            });
        }

        let email_check = self.user_client.find_by_email(req.email.clone()).await;

        match email_check {
            Ok(_) => {
                self.complete_tracing_error(&tracing_ctx, method, "Email already exists")
                    .await;
                return Err(ServiceError::Custom("Email already registered".into()));
            }
            Err(e) => {
                if e.to_string().contains("User not found") {
                } else {
                    error!("gRPC error checking email: {:?}", e);
                    self.complete_tracing_error(&tracing_ctx, method, "User query failed")
                        .await;
                    return Err(ServiceError::Internal("Database error".into()));
                }
            }
        }

        let verification_code = generate_random_string(10)
            .map_err(|_| ServiceError::Internal("Failed to generate verification code".into()))?;

        let new_user = match self
            .user_client
            .create_user(CreateUserRequest {
                first_name: req.first_name.clone(),
                last_name: req.last_name.clone(),
                email: req.email.clone(),
                password: req.password.clone(),
                confirm_password: req.confirm_password.clone(),
                verified_code: verification_code.clone(),
                is_verified: false,
            })
            .await
        {
            Ok(resp) => resp.data,
            Err(e) => return Err(grpc_status_to_service_error(e.into())),
        };

        let template = EmailTemplateData {
            title: "Welcome to SanEdge".to_string(),
            message: "Your account has been created. Verify using the link below.".to_string(),
            button: "Verify Email".to_string(),
            link: format!("https://sanedge.example.com/login?verify_code={verification_code}"),
        };

        let email_request = EmailRequest {
            to: new_user.email.clone(),
            subject: "Welcome to SanEdge".into(),
            data: template,
        };

        let payload = serde_json::to_vec(&email_request)
            .map_err(|_| ServiceError::Custom("Failed to serialize email".into()))?;

        self.kafka
            .publish(
                "email-service-topic-auth-register",
                &new_user.id.to_string(),
                &payload,
            )
            .await
            .map_err(|_| ServiceError::Custom("Failed to send registration email".into()))?;

        self.cache_store
            .set_to_cache(&cache_key, &new_user, Duration::hours(24));

        self.complete_tracing_success(&tracing_ctx, method, "User registered successfully")
            .await;

        Ok(ApiResponse {
            status: "success".into(),
            message: "User registered successfully".into(),
            data: new_user,
        })
    }
}
