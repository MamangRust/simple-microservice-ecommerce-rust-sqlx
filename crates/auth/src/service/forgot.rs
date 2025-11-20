use crate::{
    abstract_trait::{
        auth::PasswordServiceTrait,
        grpc_client::user::DynUserGrpcClient,
        reset_token::{DynResetTokenCommandRepository, DynResetTokenQueryRepository},
    },
    domain::{
        requests::{
            email::EmailRequest,
            reset_token::{CreateResetPasswordRequest, CreateResetTokenRequest},
            user::{UpdateUserPasswordRequest, UpdateUserVerifiedRequest},
        },
        response::api::ApiResponse,
    },
};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynKafka,
    cache::CacheStore,
    errors::{AppErrorGrpc, ServiceError},
    utils::{
        EmailTemplateData, MetadataInjector, Method, Metrics, Status as StatusUtils,
        TracingContext, generate_random_string,
    },
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, Status};
use tracing::{error, info, warn};

pub struct PasswordResetServiceDeps {
    pub reset_token_query: DynResetTokenQueryRepository,
    pub reset_token_command: DynResetTokenCommandRepository,
    pub user_client: DynUserGrpcClient,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub cache_store: Arc<CacheStore>,
}

#[derive(Clone)]
pub struct PasswordResetService {
    reset_token_query: DynResetTokenQueryRepository,
    reset_token_command: DynResetTokenCommandRepository,
    user_client: DynUserGrpcClient,
    kafka: DynKafka,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl PasswordResetService {
    pub async fn new(deps: PasswordResetServiceDeps) -> Self {
        let PasswordResetServiceDeps {
            reset_token_query,
            reset_token_command,
            user_client,
            kafka,
            metrics,
            registry,
            cache_store,
        } = deps;

        registry.lock().await.register(
            "password_reset_service_request_counter",
            "Total number of requests to the PasswordResetService",
            metrics.lock().await.request_counter.clone(),
        );

        registry.lock().await.register(
            "password_reset_service_request_duration",
            "Histogram of request durations for the PasswordResetService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            reset_token_query,
            reset_token_command,
            user_client,
            kafka,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("password-reset-service")
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
impl PasswordServiceTrait for PasswordResetService {
    async fn forgot(&self, email: &str) -> Result<ApiResponse<bool>, ServiceError> {
        let log_msg = format!("🔐 Forgot password requested | Email: {email}");
        info!("{log_msg}");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "ForgotPassword",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", email.to_string()),
            ],
        );

        let mut request = Request::new(email);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("auth:reset_request:{email}");

        if self
            .cache_store
            .get_from_cache::<bool>(&cache_key)
            .is_some()
        {
            let msg = "Reset request already sent recently (cached)";
            info!("✅ Cache hit: {msg}");
            self.complete_tracing_success(&tracing_ctx, method, msg)
                .await;
            return Ok(ApiResponse {
                status: "success".to_string(),
                message: msg.to_string(),
                data: true,
            });
        }

        let user_response = self
            .user_client
            .find_by_email(email.to_string())
            .await
            .map_err(|err| {
                let error_msg = err.to_string();
                error!("❌ Failed to find user by email: {}", error_msg);
                Self::grpc_status_to_service_error(err.into())
            })?;

        let random_token = match generate_random_string(10) {
            Ok(token) => token,
            Err(e) => {
                error!("❌ Failed to generate reset token: {:?}", e);
                self.complete_tracing_error(&tracing_ctx, method, "Failed to generate reset token")
                    .await;
                return Err(ServiceError::Internal(
                    "Failed to generate token".to_string(),
                ));
            }
        };

        let expires_at = (Utc::now() + Duration::hours(24)).naive_utc();

        let request_token = CreateResetTokenRequest {
            user_id: user_response.data.id,
            reset_token: random_token.clone(),
            expired_at: expires_at.to_string(),
        };

        if let Err(e) = self
            .reset_token_command
            .create_reset_token(&request_token)
            .await
        {
            error!("❌ Failed to save reset token: {:?}", e);
            self.complete_tracing_error(&tracing_ctx, method, "Failed to save reset token")
                .await;
            return Err(ServiceError::Repo(e));
        }

        self.cache_store
            .set_to_cache(&cache_key, &true, Duration::minutes(1));

        let template = EmailTemplateData {
            title: "Reset Your Password".to_string(),
            message: "Click the button below to reset your password.".to_string(),
            button: "Reset Password".to_string(),
            link: format!("https://sanedge.example.com/reset-password?token={random_token}"),
        };

        let email_req = EmailRequest {
            to: user_response.data.email.clone(),
            subject: "Password Reset Request".into(),
            data: template,
        };

        let payload_bytes = serde_json::to_vec(&email_req)
            .map_err(|e| ServiceError::Custom(format!("Failed to serialize email: {e}")))?;

        if let Err(e) = self
            .kafka
            .publish(
                "email-service-topic-auth-forgot-password",
                &user_response.data.id.to_string(),
                &payload_bytes,
            )
            .await
        {
            self.complete_tracing_error(
                &tracing_ctx,
                method,
                "Failed to publish forgot password event",
            )
            .await;

            error!("❌ Failed to publish forgot-password event: {:?}", e);
            return Err(ServiceError::Custom(
                "Failed to send email notification".to_string(),
            ));
        }

        self.complete_tracing_success(&tracing_ctx, method, "Reset email sent")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Password reset email sent".to_string(),
            data: true,
        })
    }

    async fn reset_password(
        &self,
        data: &CreateResetPasswordRequest,
    ) -> Result<ApiResponse<bool>, ServiceError> {
        let log_msg = format!("🔁 Resetting password | Token: {}", data.reset_token);
        info!("{log_msg}");

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "ResetPassword",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("token", data.reset_token.clone()),
            ],
        );

        let cache_key = format!("auth:reset:{}", data.reset_token);

        if self
            .cache_store
            .get_from_cache::<bool>(&cache_key)
            .is_some()
        {
            let msg = "Token already used (cached)";
            warn!("❌ {msg}");
            self.complete_tracing_error(&tracing_ctx, method, msg).await;
            return Err(ServiceError::Custom("Token already used".to_string()));
        }

        let reset_token_model = match self
            .reset_token_query
            .find_by_token(&data.reset_token)
            .await
        {
            Ok(Some(token)) => token,
            Ok(None) => {
                error!("❌ Reset token not found: {}", data.reset_token);
                self.complete_tracing_error(&tracing_ctx, method, "Reset token not found")
                    .await;
                return Err(ServiceError::Custom("Invalid or expired token".to_string()));
            }
            Err(e) => {
                error!("❌ DB error querying reset token: {:?}", e);
                self.complete_tracing_error(&tracing_ctx, method, "Failed to find reset token")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        if data.password != data.confirm_password {
            let msg = "Password and confirm password do not match";
            error!("❌ {msg}");
            self.complete_tracing_error(&tracing_ctx, method, msg).await;
            return Err(ServiceError::Custom(msg.to_string()));
        }

        self.user_client
            .update_user_password(UpdateUserPasswordRequest {
                user_id: reset_token_model.user_id as i32,
                password: data.password.clone(),
            })
            .await
            .map_err(|err| {
                error!("❌ Failed to update password: {}", err);
                Self::grpc_status_to_service_error(err.into())
            })?;

        self.cache_store
            .set_to_cache(&cache_key, &true, Duration::hours(24));

        self.complete_tracing_success(&tracing_ctx, method, "Password reset completed")
            .await;

        info!(
            "✅ Password reset successful for user ID: {}",
            reset_token_model.user_id
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Password reset successfully".to_string(),
            data: true,
        })
    }

    async fn verify_code(&self, code: &str) -> Result<ApiResponse<bool>, ServiceError> {
        let log_msg = format!("🔍 Verifying code | Code: {code}");
        info!("{log_msg}");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "VerifyCode",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("verify_code", code.to_string()),
            ],
        );

        let cache_key = format!("auth:verify:{code}");

        if self
            .cache_store
            .get_from_cache::<bool>(&cache_key)
            .is_some()
        {
            let msg = "Code already verified (cached)";
            info!("✅ {msg}");
            self.complete_tracing_success(&tracing_ctx, method, msg)
                .await;

            return Ok(ApiResponse {
                status: "success".into(),
                message: msg.into(),
                data: true,
            });
        }

        let user_response = self
            .user_client
            .find_verification_code(code.to_string())
            .await
            .map_err(|err| {
                error!("❌ Failed to find verification code: {}", err);
                Self::grpc_status_to_service_error(err.into())
            })?;

        let update_req = UpdateUserVerifiedRequest {
            user_id: user_response.data.id,
            is_verified: true,
        };

        self.user_client
            .update_user_is_verified(update_req)
            .await
            .map_err(|err| {
                error!("❌ Failed to update user verification: {}", err);
                Self::grpc_status_to_service_error(err.into())
            })?;

        self.cache_store
            .set_to_cache(&cache_key, &true, Duration::minutes(5));

        self.complete_tracing_success(&tracing_ctx, method, "User marked as verified")
            .await;

        Ok(ApiResponse {
            status: "success".into(),
            message: "Verification successful".into(),
            data: true,
        })
    }
}
