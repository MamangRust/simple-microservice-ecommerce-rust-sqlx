use crate::{
    abstract_trait::{
        DynHashing, DynKafka, DynRoleQueryRepository, DynUserCommandRepository,
        DynUserQueryRepository, DynUserRoleCommandRepository, RegisterServiceTrait,
    },
    cache::CacheStore,
    domain::{
        requests::{CreateUserRequest, CreateUserRoleRequest, EmailRequest, RegisterRequest},
        responses::{ApiResponse, UserResponse},
    },
    errors::ServiceError,
    utils::{
        EmailTemplateData, MetadataInjector, Method, Metrics, Status as StatusUtils,
        TracingContext, generate_random_string,
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
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::Request;
use tracing::{error, info};

pub struct RegisterService {
    query: DynUserQueryRepository,
    command: DynUserCommandRepository,
    role: DynRoleQueryRepository,
    user_role: DynUserRoleCommandRepository,
    hash: DynHashing,
    kafka: DynKafka,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

pub struct RegisterServiceDeps {
    pub query: DynUserQueryRepository,
    pub command: DynUserCommandRepository,
    pub role: DynRoleQueryRepository,
    pub user_role: DynUserRoleCommandRepository,
    pub hash: DynHashing,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub cache_store: Arc<CacheStore>,
}

impl RegisterService {
    pub async fn new(deps: RegisterServiceDeps) -> Self {
        let RegisterServiceDeps {
            query,
            command,
            role,
            user_role,
            hash,
            kafka,
            metrics,
            registry,
            cache_store,
        } = deps;

        registry.lock().await.register(
            "register_service_request_counter",
            "Total number of requests to the RegisterService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "register_service_request_duration",
            "Histogram of request durations for the RegisterService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            query,
            command,
            role,
            user_role,
            hash,
            kafka,
            metrics,
            cache_store,
        }
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

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl RegisterServiceTrait for RegisterService {
    async fn register(
        &self,
        req: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        let log_msg = format!(
            "📝 [REGISTER] Starting user registration | Email: {}",
            req.email
        );
        info!("{log_msg}");

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

        if let Some(cached_user) = self.cache_store.get_from_cache::<UserResponse>(&cache_key) {
            let log_msg = format!(
                "✅ [REGISTER] Cache hit! User already registered | Email: {}",
                req.email
            );
            info!("{log_msg}");

            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User already registered (from cache)",
            )
            .await;

            return Ok(ApiResponse {
                status: "success".to_string(),
                message: "User already registered (from cache)".to_string(),
                data: cached_user,
            });
        }

        let existing_user = match self.query.find_by_email(&req.email).await {
            Ok(user) => user,
            Err(e) => {
                error!("❌ Failed to check email in DB: {:?}", e);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        if existing_user.is_some() {
            let msg = "Email already exists";
            error!("❌ [REGISTER] Email already taken | Email: {}", req.email);
            self.complete_tracing_error(&tracing_ctx, method, msg).await;
            return Err(ServiceError::Custom("Email already registered".to_string()));
        }

        const DEFAULT_ROLE_NAME: &str = "ROLE_ADMIN";
        let role = match self.role.find_by_name(DEFAULT_ROLE_NAME).await {
            Ok(Some(role)) => role,
            Ok(None) => {
                error!("❌ Role not found: {}", DEFAULT_ROLE_NAME);
                return Err(ServiceError::Custom("Default role not found".to_string()));
            }
            Err(e) => {
                error!("❌ Failed to query role: {:?}", e);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Role query failed")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let verified_code = match generate_random_string(10) {
            Ok(code) => code,
            Err(e) => {
                error!("❌ Failed to generate verification code: {:?}", e);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate code",
                )
                .await;
                return Err(ServiceError::Internal(
                    "Failed to generate verification code".into(),
                ));
            }
        };

        let new_request = CreateUserRequest {
            firstname: req.firstname.clone(),
            lastname: req.lastname.clone(),
            password: req.password.clone(),
            email: req.email.clone(),
            confirm_password: req.confirm_password.clone(),
            verification_code: verified_code.clone(),
            is_verified: false,
        };

        let new_user = match self.command.create_user(&new_request).await {
            Ok(user) => user,
            Err(e) => {
                error!("❌ Failed to create user: {:?}", e);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to create user")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let assign_role_request = CreateUserRoleRequest {
            user_id: new_user.user_id,
            role_id: role.role_id,
        };

        if let Err(e) = self
            .user_role
            .assign_role_to_user(&assign_role_request)
            .await
        {
            error!(
                "❌ Failed to assign role {} to user {}: {:?}",
                role.role_id, new_user.user_id, e
            );
            self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to assign role")
                .await;
            return Err(ServiceError::Repo(e));
        }

        let template = EmailTemplateData {
            title: "Welcome to SanEdge".to_string(),
            message: "Your account has been successfully created.".to_string(),
            button: "Login Now".to_string(),
            link: format!("https://sanedge.example.com/login?verify_code={verified_code}"),
        };

        let email_request = EmailRequest {
            to: new_user.email.clone(),
            subject: "Welcome to SanEdge".into(),
            data: template,
        };

        let payload_bytes = serde_json::to_vec(&email_request)
            .map_err(|e| ServiceError::Custom(format!("Failed to serialize email: {e}")))?;

        if let Err(e) = self
            .kafka
            .publish(
                "email-service-topic-auth-register",
                &new_user.user_id.to_string(),
                &payload_bytes,
            )
            .await
        {
            error!("❌ Failed to publish registration event: {:?}", e);
            self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to send email")
                .await;
            return Err(ServiceError::Custom(
                "Failed to send welcome email".to_string(),
            ));
        }

        self.cache_store.set_to_cache(
            &cache_key,
            &UserResponse::from(new_user.clone()),
            Duration::hours(24),
        );

        let user_response = UserResponse::from(new_user);

        info!(
            "✅ User registered successfully: {} {} ({})",
            user_response.firstname, user_response.lastname, user_response.email
        );

        self.complete_tracing_success(&tracing_ctx, method, "User registered successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "User registered successfully".to_string(),
            data: user_response,
        })
    }
}
