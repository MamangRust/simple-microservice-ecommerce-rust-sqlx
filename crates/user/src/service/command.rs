use crate::{
    abstract_trait::{
        grpc_client::{role::DynRoleGrpcClient, user_role::DynUserRoleGrpcClient},
        user::{
            repository::{DynUserCommandRepository, DynUserQueryRepository},
            service::UserCommandServiceTrait,
        },
    },
    domain::{
        requests::{
            user::{
                CreateUserRequest, UpdateUserPasswordRequest, UpdateUserRequest,
                UpdateUserVerifiedRequest,
            },
            user_role::UserRoleRequest,
        },
        response::{
            api::ApiResponse,
            user::{UserResponse, UserResponseDeleteAt},
        },
    },
};
use anyhow::Result;
use async_trait::async_trait;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use shared::{
    abstract_trait::DynHashing,
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use tokio::time::Instant;
use tonic::Request;
use tracing::{error, info};

#[derive(Clone)]
pub struct UserCommandService {
    pub role_client: DynRoleGrpcClient,
    pub hash: DynHashing,
    pub user_role_client: DynUserRoleGrpcClient,
    pub query: DynUserQueryRepository,
    pub command: DynUserCommandRepository,
    pub metrics: Metrics,
}

pub struct UserCommandServiceDeps {
    pub role_client: DynRoleGrpcClient,
    pub user_role_client: DynUserRoleGrpcClient,
    pub hash: DynHashing,
    pub query: DynUserQueryRepository,
    pub command: DynUserCommandRepository,
}

impl UserCommandService {
    pub fn new(deps: UserCommandServiceDeps) -> Result<Self> {
        let metrics = Metrics::new(global::meter("user-command-service"));

        let UserCommandServiceDeps {
            role_client,
            user_role_client,
            query,
            command,
            hash,
        } = deps;

        Ok(Self {
            role_client,
            user_role_client,
            query,
            command,
            metrics,
            hash,
        })
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

        self.metrics.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl UserCommandServiceTrait for UserCommandService {
    async fn create_user(
        &self,
        req: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("🏗️ Creating new user: {}", req.email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "create"),
                KeyValue::new("user.email", req.email.clone()),
                KeyValue::new("user.firstname", req.first_name.clone()),
            ],
        );

        if req.password != req.confirm_password {
            return Err(ServiceError::Custom(
                "Password and confirm password do not match".into(),
            ));
        }

        let hashed_password = match self.hash.hash_password(&req.password).await {
            Ok(hash) => hash,
            Err(e) => {
                error!("❌ Failed to hash password: {:?}", e);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to hash password",
                )
                .await;
                return Err(ServiceError::Internal("Failed to hash password".into()));
            }
        };

        const DEFAULT_ROLE_NAME: &str = "ROLE_ADMIN";

        let role = match self.role_client.find_by_name(DEFAULT_ROLE_NAME).await {
            Ok(role) => {
                info!("✅ Role found: name={}", DEFAULT_ROLE_NAME);
                role
            }
            Err(e) => {
                error!("❌ Failed to fetch role: {:?}", e);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Role query failed")
                    .await;

                return Err(ServiceError::Internal("Role service unavailable".into()));
            }
        };

        let create_req = CreateUserRequest {
            first_name: req.first_name.clone(),
            last_name: req.last_name.clone(),
            email: req.email.clone(),
            password: hashed_password,
            confirm_password: req.confirm_password.clone(),
            is_verified: req.is_verified,
            verified_code: req.verified_code.clone(),
        };

        let mut req = Request::new(create_req.clone());

        self.inject_trace_context(&tracing_ctx.cx, &mut req);

        let new_user = match self.command.create_user(&create_req).await {
            Ok(user) => {
                info!("✅ User created successfully: {}", user.email);
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "User created successfully",
                )
                .await;
                user
            }
            Err(e) => {
                error!("❌ Failed to create user '{}': {e:?}", create_req.email);
                self.complete_tracing_error(&tracing_ctx, method.clone(), &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let role_req = UserRoleRequest {
            user_id: new_user.user_id,
            role_id: role.data.id,
        };

        if let Err(status) = self.user_role_client.assign_role(role_req).await {
            error!(
                "❌ gRPC error assigning role: user_id={} status={:?}",
                new_user.user_id, status
            );

            self.complete_tracing_error(&tracing_ctx, method.clone(), "Role service unavailable")
                .await;

            return Err(ServiceError::Internal("Role service unavailable".into()));
        }

        let user_response = UserResponse::from(new_user);

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
        info!("✏️ Updating user {:?}", req.user_id);

        let user_id = match req.user_id {
            Some(id) => id,
            None => {
                error!("❌ Update user failed: user_id is missing in the request");
                return Err(ServiceError::Custom(
                    "User ID is required for update".into(),
                ));
            }
        };

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "UpdateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "update"),
                KeyValue::new("user_id", user_id.to_string()),
            ],
        );

        let existing_user = match self.query.find_by_id(user_id).await {
            Ok(Some(user)) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Fetched existing user",
                )
                .await;
                user
            }
            Ok(None) => {
                self.complete_tracing_error(&tracing_ctx, method.clone(), "User not found")
                    .await;

                return Err(ServiceError::Custom("User not found".into()));
            }
            Err(e) => {
                error!("❌ Failed find user ID={} | {:?}", user_id, e);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "User query failed")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let mut new_email = existing_user.email.clone();

        if !req.email.is_empty() && req.email != existing_user.email {
            info!(
                "🔄 Email is changed: {} → {}",
                existing_user.email, req.email
            );

            let email_check = self.query.find_by_email(req.email.clone()).await;

            if let Ok(Some(_)) = email_check {
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Email already exists")
                    .await;

                return Err(ServiceError::Custom("Email already registered".into()));
            } else if let Err(e) = email_check {
                error!("❌ Error checking email: {:?}", e);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Email check error")
                    .await;

                return Err(ServiceError::Repo(e));
            }

            new_email = req.email.clone();
        }

        let mut new_password = existing_user.password.clone();

        if !req.password.is_empty() {
            if req.password != req.confirm_password {
                return Err(ServiceError::Custom(
                    "Password and confirmation do not match".into(),
                ));
            }

            new_password = match self.hash.hash_password(&req.password).await {
                Ok(hash) => hash,
                Err(e) => {
                    error!("❌ Failed hashing password: {:?}", e);
                    return Err(ServiceError::Internal("Failed to hash password".into()));
                }
            };
        }

        let update_req = UpdateUserRequest {
            user_id: req.user_id,
            first_name: if req.first_name.is_empty() {
                existing_user.firstname
            } else {
                req.first_name.clone()
            },
            last_name: if req.last_name.is_empty() {
                existing_user.lastname
            } else {
                req.last_name.clone()
            },
            email: new_email,
            password: new_password,
            confirm_password: req.confirm_password.clone(),
        };

        let updated_user = match self.command.update_user(&update_req).await {
            Ok(user) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "User updated successfully",
                )
                .await;
                user
            }
            Err(e) => {
                error!("❌ Failed update user ID={} | {:?}", user_id, e);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to update user")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let res = ApiResponse {
            status: "success".to_string(),
            message: "User updated successfully".into(),
            data: UserResponse::from(updated_user),
        };

        info!("✅ User updated: {}", res.data.email);

        Ok(res)
    }

    async fn update_user_is_verified(
        &self,
        req: &UpdateUserVerifiedRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!(
            "✔️ Updating user verification status: user_id={}, is_verified={}",
            req.user_id, req.is_verified
        );

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "UpdateUserIsVerified",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "update_is_verified"),
                KeyValue::new("user.id", req.user_id.to_string()),
                KeyValue::new("user.is_verified", req.is_verified.to_string()),
            ],
        );

        let user_model = match self.command.update_isverifed(req).await {
            Ok(user) => {
                info!("✅ User verification updated: {}", user.email);
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "User verification updated",
                )
                .await;
                user
            }
            Err(e) => {
                error!(
                    "❌ Failed to update verification for user {}: {e:?}",
                    req.user_id
                );
                self.complete_tracing_error(&tracing_ctx, method.clone(), &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponse::from(user_model);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User verification updated successfully".to_string(),
            data: user_response,
        };

        info!(
            "✔️ User verification updated: {} (ID: {})",
            response.data.email, response.data.id
        );

        Ok(response)
    }

    async fn update_user_password(
        &self,
        req: &UpdateUserPasswordRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("🔑 Updating password for user ID: {}", req.user_id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "UpdateUserPassword",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "update_password"),
                KeyValue::new("user.id", req.user_id.to_string()),
            ],
        );

        let user_model = match self.command.update_password(req).await {
            Ok(user) => {
                info!("✅ Password updated for user: {}", user.email);
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "User password updated",
                )
                .await;
                user
            }
            Err(e) => {
                error!(
                    "❌ Failed to update password for user {}: {e:?}",
                    req.user_id
                );
                self.complete_tracing_error(&tracing_ctx, method.clone(), &e.to_string())
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponse::from(user_model);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Password updated successfully".to_string(),
            data: user_response,
        };

        info!(
            "🔑 Password updated for user: {} (ID: {})",
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

    async fn restore_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<UserResponseDeleteAt>, ServiceError> {
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

        let user_response = UserResponseDeleteAt::from(user_model);

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
