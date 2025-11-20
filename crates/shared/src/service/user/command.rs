use crate::{
    abstract_trait::{DynUserCommandRepository, UserCommandServiceTrait},
    domain::{
        requests::{CreateUserRequest, RegisterRequest, UpdateUserRequest},
        responses::{ApiResponse, UserResponse, UserResponseDeleteAt},
    },
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
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

pub struct UserCommandService {
    pub command: DynUserCommandRepository,
    pub metrics: Arc<Mutex<Metrics>>,
}

impl UserCommandService {
    pub async fn new(
        command: DynUserCommandRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        registry.lock().await.register(
            "user_command_service_request_counter",
            "Total number of requests to the UserCommandService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "user_command_service_request_duration",
            "Histogram of request durations for the UserCommandService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { command, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-command-service")
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
            info!("Operation completed successfully: {message}");
        } else {
            error!("Operation failed: {message}");
        }

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl UserCommandServiceTrait for UserCommandService {
    async fn create_user(
        &self,
        req: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("🏗️ Creating new user: {}", req.email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "create"),
                KeyValue::new("user.email", req.email.clone()),
                KeyValue::new("user.firstname", req.firstname.clone()),
            ],
        );

        let create_req = CreateUserRequest {
            firstname: req.firstname.clone(),
            lastname: req.lastname.clone(),
            email: req.email.clone(),
            password: req.password.clone(),
            confirm_password: req.confirm_password.clone(),
            is_verified: true,
            verification_code: "".to_string(),
        };

        let mut req = Request::new(create_req.clone());

        self.inject_trace_context(&tracing_ctx.cx, &mut req);

        let user_model = match self.command.create_user(&create_req).await {
            Ok(user) => {
                info!("✅ User created successfully: {}", user.email);
                self.complete_tracing_success(&tracing_ctx, method, "User created successfully")
                    .await;
                user
            }
            Err(e) => {
                error!("❌ Failed to create user '{}': {e:?}", create_req.email);
                self.complete_tracing_error(&tracing_ctx, method, &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponse::from(user_model);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User created successfully".to_string(),
            data: user_response,
        };

        info!(
            "✅ User created successfully: {} (ID: {})",
            response.data.email, response.data.id
        );

        Ok(response)
    }

    async fn update_user(
        &self,
        req: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("✏️ Updating user: {}", req.email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "UpdateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "update"),
                KeyValue::new("user.email", req.email.clone()),
                KeyValue::new("user.firstname", req.firstname.clone()),
            ],
        );

        let user_model = match self.command.update_user(req).await {
            Ok(user) => {
                info!("✅ User updated successfully: {}", user.email);
                self.complete_tracing_success(&tracing_ctx, method, "User updated successfully")
                    .await;
                user
            }
            Err(e) => {
                error!("❌ Failed to update user '{}': {e:?}", req.email);
                self.complete_tracing_error(&tracing_ctx, method, &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponse::from(user_model);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User updated successfully".to_string(),
            data: user_response,
        };

        info!(
            "✅ User updated: {} (ID: {})",
            response.data.email, response.data.id
        );

        Ok(response)
    }

    async fn trash_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, ServiceError> {
        info!("🗑️ Soft deleting user with ID: {}", id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "TrashUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let user_model = match self.command.trash_user(id).await {
            Ok(user) => {
                info!("✅ User moved to trash: {}", user.email);
                self.complete_tracing_success(&tracing_ctx, method.clone(), "User moved to trash")
                    .await;
                user
            }
            Err(e) => {
                error!("❌ Failed to move user ID {id} to trash: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponseDeleteAt::from(user_model);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User moved to trash successfully".to_string(),
            data: user_response,
        };

        info!("✅ User soft deleted: {} (ID: {id})", response.data.email);

        Ok(response)
    }

    async fn restore_user(&self, id: i32) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("🔄 Restoring user with ID: {}", id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let user_model = match self.command.restore_user(id).await {
            Ok(user) => {
                self.complete_tracing_success(&tracing_ctx, method, "User restored")
                    .await;
                user
            }
            Err(e) => {
                error!("❌ Failed to restore user ID {id}: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method, &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponse::from(user_model);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User restored successfully".to_string(),
            data: user_response,
        };

        info!("✅ User restored: {} (ID: {id})", response.data.email);

        Ok(response)
    }

    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting user with ID: {id}");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "DeleteUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        match self.command.delete_user(id).await {
            Ok(_) => {
                info!("✅ User permanently deleted: {id}");
                self.complete_tracing_success(&tracing_ctx, method, "User deleted permanently")
                    .await;
            }
            Err(e) => {
                error!("❌ Failed to delete user ID {id}: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method, &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };
        let response = ApiResponse {
            status: "success".to_string(),
            message: "User deleted permanently".to_string(),
            data: (),
        };

        info!("✅ User permanently deleted: {id}");

        Ok(response)
    }

    async fn restore_all_user(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("🔄 Restoring all soft-deleted users");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreAllUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "restore"),
            ],
        );

        match self.command.restore_all_user().await {
            Ok(_) => {
                self.complete_tracing_success(&tracing_ctx, method, "All users restored")
                    .await;
            }
            Err(e) => {
                error!("❌ Failed to restore all users: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method, &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".to_string(),
            message: "All users restored successfully".to_string(),
            data: (),
        };

        info!("✅ All users restored successfully");

        Ok(response)
    }

    async fn delete_all_user(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting all users");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "DeleteAllUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "delete"),
            ],
        );

        match self.command.delete_all_user().await {
            Ok(_) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "All users deleted permanently",
                )
                .await;
            }
            Err(e) => {
                error!("❌ Failed to delete all users: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method, &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".to_string(),
            message: "All users deleted permanently".to_string(),
            data: (),
        };

        info!("✅ All users deleted permanently");

        Ok(response)
    }
}
