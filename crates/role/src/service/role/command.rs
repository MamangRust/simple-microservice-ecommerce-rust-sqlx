use crate::{
    abstract_trait::role::{
        repository::DynRoleCommandRepository, service::RoleCommandServiceTrait,
    },
    domain::{
        requests::role::{CreateRoleRequest, UpdateRoleRequest},
        response::{
            api::ApiResponse,
            role::{RoleResponse, RoleResponseDeleteAt},
        },
    },
};
use shared::{
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};

use async_trait::async_trait;
use genproto::order::FindByIdOrderRequest;
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

#[derive(Clone)]
pub struct RoleCommandService {
    pub command: DynRoleCommandRepository,
    pub metrics: Arc<Mutex<Metrics>>,
}

impl RoleCommandService {
    pub async fn new(
        command: DynRoleCommandRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        registry.lock().await.register(
            "role_command_service_request_counter",
            "Total number of requests to the RoleCommandService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "role_command_service_request_duration",
            "Histogram of request durations for the RoleCommandService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { command, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("role-command-service")
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
impl RoleCommandServiceTrait for RoleCommandService {
    async fn create_role(
        &self,
        role: &CreateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, ServiceError> {
        info!("🏗️ Creating new role: {}", role.name);

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "create_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "create"),
                KeyValue::new("role.name", role.name.clone()),
            ],
        );

        let mut request = Request::new(role.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let role_model = match self.command.create_role(role).await {
            Ok(model) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role created successfully",
                )
                .await;
                model
            }
            Err(err) => {
                error!("❌ Failed to create role '{}': {err:?}", role.name);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to create role")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = RoleResponse::from(role_model);

        info!(
            "✅ Role created successfully: {} (ID: {})",
            response.role_name, response.role_id,
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Role created successfully".to_string(),
            data: response,
        })
    }

    async fn update_role(
        &self,
        role: &UpdateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, ServiceError> {
        info!("✏️ Updating role with ID: {}", role.id);

        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "update_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "update"),
                KeyValue::new("role.id", role.id.to_string()),
                KeyValue::new("role.name", role.name.clone()),
            ],
        );

        let mut request = Request::new(role.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let role_model = match self.command.update_role(role).await {
            Ok(model) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role updated successfully",
                )
                .await;
                model
            }
            Err(err) => {
                error!("❌ Failed to update role ID {}: {err:?}", role.id);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to update role")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = RoleResponse::from(role_model);

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Role updated successfully".to_string(),
            data: response,
        })
    }

    async fn trash_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, ServiceError> {
        info!("🗑️ Soft deleting role with ID: {role_id}");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "trash_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("role.id", role_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id: role_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let role_model = match self.command.trash_role(role_id).await {
            Ok(model) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role moved to trash successfully",
                )
                .await;
                model
            }
            Err(err) => {
                error!("❌ Failed to soft delete role ID {role_id}: {err:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to trash role")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = RoleResponseDeleteAt::from(role_model);

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Role moved to trash successfully".to_string(),
            data: response,
        })
    }

    async fn restore_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, ServiceError> {
        info!("🔄 Restoring role with ID: {role_id}");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "restore_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("role.id", role_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id: role_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let role_model = match self.command.restore_role(role_id).await {
            Ok(model) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role restored successfully",
                )
                .await;
                model
            }
            Err(err) => {
                error!("❌ Failed to restore role ID {role_id}: {err:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to restore role")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = RoleResponseDeleteAt::from(role_model);

        info!("✅ Role restored: {} (ID: {role_id})", response.role_name);

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Role restored successfully".to_string(),
            data: response,
        })
    }

    async fn delete_ole(&self, role_id: i32) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting role with ID: {role_id}");

        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "delete_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("role.id", role_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id: role_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.command.delete_role(role_id).await {
            Ok(()) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role deleted permanently",
                )
                .await;
            }
            Err(err) => {
                error!("❌ Failed to permanently delete role ID {role_id}: {err:?}",);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to delete role")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        }

        info!("✅ Role permanently deleted: {role_id}");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Role deleted permanently".to_string(),
            data: (),
        })
    }

    async fn restore_all_role(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("🔄 Restoring all soft-deleted roles");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "restore_all_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "restore_all"),
            ],
        );

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.command.restore_all_role().await {
            Ok(()) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "All roles restored successfully",
                )
                .await;
            }
            Err(err) => {
                error!("❌ Failed to restore all roles: {err:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to restore all roles",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        }

        info!("✅ All roles restored successfully");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "All roles restored successfully".to_string(),
            data: (),
        })
    }

    async fn delete_all_role(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting all roles");

        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "delete_all_role",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "delete_all"),
            ],
        );

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.command.delete_all_role().await {
            Ok(()) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "All roles deleted permanently",
                )
                .await;
            }
            Err(err) => {
                error!("❌ Failed to delete all roles: {err:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to delete all roles",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        }

        info!("✅ All roles deleted permanently");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "All roles deleted permanently".to_string(),
            data: (),
        })
    }
}
