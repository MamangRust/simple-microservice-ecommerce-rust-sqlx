use crate::{
    abstract_trait::{DynRoleQueryRepository, RoleQueryServiceTrait},
    cache::CacheStore,
    domain::{
        requests::FindAllRole,
        responses::{
            ApiResponse, ApiResponsePagination, Pagination, RoleResponse, RoleResponseDeleteAt,
        },
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
use tracing::{error, info};

pub struct RoleQueryService {
    pub query: DynRoleQueryRepository,
    pub metrics: Arc<Mutex<Metrics>>,
    pub cache_store: Arc<CacheStore>,
}

impl RoleQueryService {
    pub async fn new(
        query: DynRoleQueryRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.lock().await.register(
            "role_query_service_request_counter",
            "Total number of requests to the RoleQueryService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "role_query_service_request_duration",
            "Histogram of request durations for the RoleQueryService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            query,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("role-query-service")
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
impl RoleQueryServiceTrait for RoleQueryService {
    async fn find_all(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponse>>, ServiceError> {
        info!(
            "🔍 Finding all roles | Page: {}, Size: {}, Search: '{}'",
            req.page, req.page_size, req.search
        );

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "role_find_all",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "role:find_all:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<RoleResponse>>>(&cache_key)
        {
            info!("✅ Found {} roles in cache", cached.data.len());
            self.complete_tracing_success(&tracing_ctx, method, "Roles retrieved from cache")
                .await;
            return Ok(cached);
        }

        let (roles, total) = match self.query.find_all(req).await {
            Ok(res) => {
                info!("✅ Retrieved {} roles from database", res.0.len());
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Roles retrieved from DB",
                )
                .await;
                res
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch all roles: {e:?}");
                error!("{}", msg);
                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<RoleResponse> = roles.into_iter().map(RoleResponse::from).collect();
        let total_pages = ((total - 1) / page_size as i64) + 1;

        let pagination = Pagination {
            page,
            page_size,
            total_items: total,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Roles retrieved successfully".to_string(),
            data,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));

        info!(
            "✅ Roles retrieved: {} (total: {total})",
            response.data.len()
        );

        Ok(response)
    }

    async fn find_active(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, ServiceError> {
        info!(
            "🟢 Finding active roles | Page: {}, Size: {}, Search: '{}'",
            req.page, req.page_size, req.search
        );

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "role_find_active",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "role:find_active:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<RoleResponseDeleteAt>>>(&cache_key)
        {
            info!("✅ Found {} active roles in cache", cached.data.len());
            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "Active roles retrieved from cache",
            )
            .await;
            return Ok(cached);
        }

        let (roles, total) = match self.query.find_active(req).await {
            Ok(res) => {
                info!("✅ Retrieved {} active roles from DB", res.0.len());
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Active roles retrieved from DB",
                )
                .await;
                res
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch active roles: {e:?}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<RoleResponseDeleteAt> =
            roles.into_iter().map(RoleResponseDeleteAt::from).collect();
        let total_pages = ((total - 1) / page_size as i64) + 1;

        let pagination = Pagination {
            page,
            page_size,
            total_items: total,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Active roles retrieved successfully".to_string(),
            data,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));

        info!(
            "✅ Found {} active roles (total: {})",
            response.data.len(),
            total
        );

        Ok(response)
    }

    async fn find_trashed(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, ServiceError> {
        info!(
            "🗑️ Finding trashed roles | Page: {}, Size: {}, Search: '{}'",
            req.page, req.page_size, req.search
        );

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "role_find_trashed",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "role:find_trashed:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<RoleResponseDeleteAt>>>(&cache_key)
        {
            info!("✅ Found {} trashed roles in cache", cached.data.len());
            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "Trashed roles retrieved from cache",
            )
            .await;
            return Ok(cached);
        }

        let (roles, total) = match self.query.find_trashed(req).await {
            Ok(res) => {
                info!("✅ Retrieved {} trashed roles from DB", res.0.len());
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Trashed roles retrieved from DB",
                )
                .await;
                res
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch trashed roles: {e:?}");
                error!("{}", msg);
                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<RoleResponseDeleteAt> =
            roles.into_iter().map(RoleResponseDeleteAt::from).collect();
        let total_pages = ((total - 1) / page_size as i64) + 1;

        let pagination = Pagination {
            page,
            page_size,
            total_items: total,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Trashed roles retrieved successfully".to_string(),
            data,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));

        info!(
            "✅ Found {} trashed roles (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<RoleResponse>, ServiceError> {
        info!("🆔 Finding role by ID: {}", id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "role_find_by_id",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("role.id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("role:find_by_id:id:{id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<RoleResponse>>(&cache_key)
        {
            info!("✅ Found role ID {id} in cache");
            self.complete_tracing_success(&tracing_ctx, method, "Role retrieved from cache")
                .await;
            return Ok(cached);
        }

        let role = match self.query.find_by_id(id).await {
            Ok(Some(role)) => {
                info!("✅ Found role in DB: {} (ID: {id})", role.role_name);
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role retrieved from DB",
                )
                .await;
                role
            }
            Ok(None) => {
                let msg = format!("❌ Role not found with ID: {id}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Role not found")
                    .await;
                return Err(ServiceError::Custom("Role not found".to_string()));
            }
            Err(e) => {
                let msg = format!("❌ Database error while finding role ID {id}: {e:?}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Role retrieved successfully".to_string(),
            data: RoleResponse::from(role),
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));

        info!(
            "✅ Role retrieved: '{}' (ID: {})",
            response.data.role_name, id
        );

        Ok(response)
    }

    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Vec<RoleResponse>>, ServiceError> {
        info!("👥 Finding roles for user ID: {}", user_id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "find_by_user_id",
            vec![
                KeyValue::new("component", "role"),
                KeyValue::new("operation", "find_by_user_id"),
                KeyValue::new("user.id", user_id.to_string()),
            ],
        );

        let mut request = Request::new(user_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let roles = match self.query.find_by_user_id(user_id).await {
            Ok(roles) => {
                info!("✅ Retrieved {} roles for user ID {user_id}", roles.len());
                self.complete_tracing_success(&tracing_ctx, method.clone(), "User roles retrieved")
                    .await;
                roles
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch roles for user ID {user_id}: {e:?}");
                error!("{}", msg);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<RoleResponse> = roles.into_iter().map(RoleResponse::from).collect();

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User roles retrieved successfully".to_string(),
            data,
        };

        info!(
            "✅ Found {} roles for user ID: {}",
            response.data.len(),
            user_id
        );

        Ok(response)
    }
}
