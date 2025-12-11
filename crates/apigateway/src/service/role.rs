use crate::{
    abstract_trait::role::RoleGrpcClientTrait,
    domain::{
        requests::role::{
            CreateRoleRequest as DomainCreateRoleRequest, FindAllRole as DomainFindAllRoles,
            UpdateRoleRequest as DomainUpdateRoleRequest,
        },
        response::{
            api::{ApiResponse, ApiResponsePagination},
            role::{RoleResponse, RoleResponseDeleteAt},
        },
    },
};
use anyhow::Result;
use async_trait::async_trait;
use genproto::role::{
    CreateRoleRequest, FindAllRoleRequest, FindByIdRoleRequest, FindByIdUserRoleRequest,
    UpdateRoleRequest, role_command_service_client::RoleCommandServiceClient,
    role_query_service_client::RoleQueryServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    errors::{AppErrorGrpc, HttpError},
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use tokio::time::Instant;
use tonic::{Request, transport::Channel};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct RoleGrpcClientService {
    query_client: RoleQueryServiceClient<Channel>,
    command_client: RoleCommandServiceClient<Channel>,
    metrics: Metrics,
}

impl RoleGrpcClientService {
    pub fn new(
        query_client: RoleQueryServiceClient<Channel>,
        command_client: RoleCommandServiceClient<Channel>,
        registry: &mut Registry,
    ) -> Result<Self> {
        let metrics = Metrics::new();

        registry.register(
            "role_service_client_request_counter",
            "Total number of requests to the RoleGrpcClientService",
            metrics.request_counter.clone(),
        );
        registry.register(
            "role_service_client_duration",
            "Histogram of request durations for the RoleGrpcClientService",
            metrics.request_duration.clone(),
        );

        Ok(Self {
            query_client,
            command_client,
            metrics,
        })
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("role-client-service")
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
impl RoleGrpcClientTrait for RoleGrpcClientService {
    async fn find_all(
        &self,
        req: &DomainFindAllRoles,
    ) -> Result<ApiResponsePagination<Vec<RoleResponse>>, HttpError> {
        info!(
            "Retrieving all role (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllRoleRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.clone().find_all_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched roles")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to fetch roles")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let roles: Vec<RoleResponse> = inner.data.into_iter().map(Into::into).collect();

        let role_len = roles.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: roles,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {role_len} Roles");
        Ok(reply)
    }

    async fn find_active(
        &self,
        req: &DomainFindAllRoles,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving all active role (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindActiveRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllRoleRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.clone().find_by_active(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched roles")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to fetch roles")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let roles: Vec<RoleResponseDeleteAt> = inner.data.into_iter().map(Into::into).collect();

        let roles_len = roles.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: roles,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {roles_len} active Roles");
        Ok(reply)
    }

    async fn find_trashed(
        &self,
        req: &DomainFindAllRoles,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving all trashed role (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindTrashedRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllRoleRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.clone().find_by_trashed(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched roles")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to fetch roles")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let roles: Vec<RoleResponseDeleteAt> = inner.data.into_iter().map(Into::into).collect();

        let roles_len = roles.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: roles,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {roles_len} trashed Roles");
        Ok(reply)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<RoleResponse>, HttpError> {
        info!("Retrieving Role: {}", id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindByIdRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("role.id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdRoleRequest { role_id: id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.clone().find_by_id_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched Role")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to fetch Role")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let role_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Role data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_role: RoleResponse = role_data.into();

        let role_name = domain_role.clone().role_name;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_role.clone(),
        };

        info!("Successfully fetched Role: {role_name}");
        Ok(reply)
    }

    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Vec<RoleResponse>>, HttpError> {
        info!("Fetching Roles by user_id: {}", user_id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindByIdUserRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_by_user_id"),
                KeyValue::new("user.id", user_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdUserRoleRequest { user_id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.clone().find_by_user_id(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched roles")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to fetch roles")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let roles: Vec<RoleResponse> = inner.data.into_iter().map(RoleResponse::from).collect();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: roles.clone(),
        };

        info!(
            "Successfully fetched {} roles for user_id {}",
            roles.len(),
            user_id
        );
        Ok(reply)
    }

    async fn create_role(
        &self,
        req: &DomainCreateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, HttpError> {
        info!("Creating new Role: {}", req.name.clone());

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "create"),
                KeyValue::new("role.name", req.name.clone()),
            ],
        );

        let mut request = Request::new(CreateRoleRequest {
            name: req.name.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().create_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully created Role")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to create Role")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let role_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Role data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_role: RoleResponse = role_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_role,
        };

        info!("Role {} created successfully", req.name);
        Ok(reply)
    }

    async fn update_role(
        &self,
        req: &DomainUpdateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, HttpError> {
        info!("Updating Role: {:?}", req.id);

        let role_id = req
            .id
            .ok_or_else(|| HttpError::BadRequest("role_id is required".into()))?;

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "update"),
                KeyValue::new("role.id", role_id.to_string()),
                KeyValue::new("role.name", req.name.clone()),
            ],
        );

        let mut request = Request::new(UpdateRoleRequest {
            id: role_id,
            name: req.name.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().update_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully updated Role")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to update Role")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let role_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Role data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_role: RoleResponse = role_data.into();

        let role_name = domain_role.clone().role_name;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_role,
        };

        info!("Role {role_name} updated successfully");
        Ok(reply)
    }

    async fn trash_role(&self, id: i32) -> Result<ApiResponse<RoleResponseDeleteAt>, HttpError> {
        info!("Soft deleting Role: {id}");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "TrashRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("role_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdRoleRequest { role_id: id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().trashed_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully soft deleted Role",
                )
                .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to soft delete Role")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let role_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Role data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_role: RoleResponseDeleteAt = role_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_role,
        };

        info!("Role {} soft deleted successfully", id);
        Ok(reply)
    }

    async fn restore_role(&self, id: i32) -> Result<ApiResponse<RoleResponseDeleteAt>, HttpError> {
        info!("Restoring Role: {}", id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("role_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdRoleRequest { role_id: id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().restore_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully restored Role")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to restore Role")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let role_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Role data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_role: RoleResponseDeleteAt = role_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_role,
        };

        info!("Role {} restored successfully", id);
        Ok(reply)
    }

    async fn delete_ole(&self, id: i32) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting Role: {}", id);

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("role_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdRoleRequest { role_id: id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .clone()
            .delete_role_permanent(request)
            .await
        {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully deleted Role")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to delete Role")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("Role {} permanently deleted", id);
        Ok(reply)
    }

    async fn restore_all_role(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Restoring all trashed Roles");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreAllRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "restore"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().restore_all_role(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "All Roles restored")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to restore all Roles")
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All Roles restored successfully");
        Ok(reply)
    }

    async fn delete_all_role(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting all trashed Roles");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteAllRole",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "delete"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .clone()
            .delete_all_role_permanent(request)
            .await
        {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "All trashed Roles deleted")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    "Failed to delete all trashed Roles",
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All trashed Roles permanently deleted");
        Ok(reply)
    }
}
