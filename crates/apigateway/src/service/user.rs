use crate::{
    abstract_trait::user::UserGrpcClientTrait,
    domain::{
        requests::user::{
            FindAllUsers as DomainFindAllUSers, UpdateUserRequest as DomainUpdateUserRequest,
        },
        response::{
            api::{ApiResponse, ApiResponsePagination},
            user::{UserResponse, UserResponseDeleteAt},
        },
    },
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use genproto::user::{
    FindAllUserRequest, FindByIdUserRequest, UpdateUserRequest,
    user_command_service_client::UserCommandServiceClient,
    user_query_service_client::UserQueryServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use shared::cache::CacheStore;
use shared::{
    errors::{AppErrorGrpc, HttpError},
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::time::Instant;
use tonic::{Request, transport::Channel};
use tracing::{error, info};

#[derive(Clone)]
pub struct UserGrpcClientService {
    query_client: UserQueryServiceClient<Channel>,
    command_client: UserCommandServiceClient<Channel>,
    metrics: Metrics,
    cache_store: Arc<CacheStore>,
}

impl UserGrpcClientService {
    pub fn new(
        query_client: UserQueryServiceClient<Channel>,
        command_client: UserCommandServiceClient<Channel>,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let metrics = Metrics::new(global::meter("user-client-service"));

        Ok(Self {
            query_client,
            command_client,
            metrics,
            cache_store,
        })
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-client-service")
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
impl UserGrpcClientTrait for UserGrpcClientService {
    async fn find_all(
        &self,
        req: &DomainFindAllUSers,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all user (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllOrders",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllUserRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "user:find_all:page:{page}:size:{page_size}:search:{}",
            req.search.clone()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponse>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} users in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_all(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched users")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let users: Vec<UserResponse> = inner.data.into_iter().map(Into::into).collect();

        let user_len = users.len();

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: users,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(10))
            .await;

        info!("Successfully fetched {user_len} users");
        Ok(api_response)
    }

    async fn find_active(
        &self,
        req: &DomainFindAllUSers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all active user (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindActiveUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllUserRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "user:find_active:page:{page}:size:{page_size}:search:{}",
            req.search.clone()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} users in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_active(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched active users",
                )
                .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let users: Vec<UserResponseDeleteAt> = inner.data.into_iter().map(Into::into).collect();

        let users_len = users.len();

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: users,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(10))
            .await;

        info!("Successfully fetched {users_len} active users");
        Ok(api_response)
    }

    async fn find_trashed(
        &self,
        req: &DomainFindAllUSers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all trashed user (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindTrashedUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllUserRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "user:find_trashed:page:{page}:size:{page_size}:search:{}",
            req.search.clone()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} users in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_trashed(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched trashed users",
                )
                .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let users: Vec<UserResponseDeleteAt> = inner.data.into_iter().map(Into::into).collect();

        let users_len = users.len();

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: users,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(10))
            .await;

        info!("Successfully fetched {users_len} trashed users");
        Ok(api_response)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, HttpError> {
        info!("Fetching user by ID: {}", id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindByIdUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdUserRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("user:find_by_id:{id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<UserResponse>>(&cache_key)
            .await
        {
            info!("✅ Found user with ID {id} in cache");
            self.complete_tracing_success(&tracing_ctx, method, "User retrieved from cache")
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_id(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched user")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("User data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_user: UserResponse = user_data.into();

        let user_email = domain_user.clone().email;

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user.clone(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(10))
            .await;

        info!("Successfully fetched user: {user_email}");
        Ok(api_response)
    }

    async fn update_user(
        &self,
        req: &DomainUpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, HttpError> {
        info!("Updating user: {:?}", req.user_id);

        let user_id = req
            .user_id
            .ok_or_else(|| HttpError::BadRequest("user_id is required".into()))?;

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "update"),
                KeyValue::new("user.id", user_id.to_string()),
                KeyValue::new("user.email", req.email.clone()),
                KeyValue::new("user.firstname", req.first_name.clone()),
            ],
        );

        let mut request = Request::new(UpdateUserRequest {
            id: user_id,
            firstname: req.first_name.clone(),
            lastname: req.last_name.clone(),
            email: req.email.clone(),
            password: req.password.clone(),
            confirm_password: req.confirm_password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().update_user(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "User updated successfully")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("User data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_user: UserResponse = user_data.into();

        let user_email = domain_user.clone().email;

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        };

        info!("User {user_email} updated successfully");
        Ok(api_response)
    }

    async fn trash_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, HttpError> {
        info!("Soft deleting user: {id}");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "TrashUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdUserRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().trashed_user(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "User soft deleted successfully",
                )
                .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("User data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_user: UserResponseDeleteAt = user_data.into();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        };

        info!("User {} soft deleted successfully", id);
        Ok(api_response)
    }

    async fn restore_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, HttpError> {
        info!("Restoring user: {}", id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdUserRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().restore_user(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "User restored successfully")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("User data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_user: UserResponseDeleteAt = user_data.into();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        };

        info!("User {} restored successfully", id);
        Ok(api_response)
    }

    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting user: {}", id);

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdUserRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .clone()
            .delete_user_permanent(request)
            .await
        {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "User permanently deleted")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("User {} permanently deleted", id);
        Ok(api_response)
    }

    async fn restore_all_user(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Restoring all trashed users");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreAllUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "restore"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().restore_all_user(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "All users restored")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All users restored successfully");
        Ok(api_response)
    }

    async fn delete_all_user(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting all trashed users");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteAllUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "delete"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .clone()
            .delete_all_user_permanent(request)
            .await
        {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "All trashed users deleted")
                    .await;
                response
            }
            Err(status) => {
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All trashed users permanently deleted");
        Ok(api_response)
    }
}
