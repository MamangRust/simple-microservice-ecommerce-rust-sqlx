use crate::{
    abstract_trait::user::{repository::DynUserQueryRepository, service::UserQueryServiceTrait},
    domain::{
        requests::user::FindAllUsers,
        response::{
            api::{ApiResponse, ApiResponsePagination},
            pagination::Pagination,
            user::{UserResponse, UserResponseDeleteAt, UserResponseWithPassword},
        },
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
use shared::{
    cache::CacheStore,
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::time::Instant;
use tonic::Request;
use tracing::{error, info};

#[derive(Clone)]
pub struct UserQueryService {
    pub query: DynUserQueryRepository,
    pub metrics: Metrics,
    pub cache_store: Arc<CacheStore>,
}

impl UserQueryService {
    pub fn new(query: DynUserQueryRepository, cache_store: Arc<CacheStore>) -> Result<Self> {
        let metrics = Metrics::new(global::meter("user-query-service"));

        Ok(Self {
            query,
            metrics,
            cache_store,
        })
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-query-service")
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
impl UserQueryServiceTrait for UserQueryService {
    async fn find_all(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ServiceError> {
        info!(
            "🔍 Finding all users | Page: {}, Size: {}, Search: '{}'",
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
            "find_all",
            vec![
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "user:find_all:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
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

        let (users, total) = match self.query.find_all(req).await {
            Ok(res) => {
                let log_msg = format!("✅ Found {} users", res.0.len());
                info!("{log_msg}");
                self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                    .await;
                res
            }
            Err(e) => {
                error!("❌ Failed to fetch all users: {e:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    &format!("❌ Failed to fetch all users: {e:?}"),
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();

        let total_pages = ((total - 1) / req.page_size as i64) + 1;

        let pagination = Pagination {
            page: req.page,
            page_size: req.page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Users retrieved successfully".to_string(),
            data: user_response,
            pagination,
        };

        info!("✅ Found {} users (total: {total})", response.data.len());

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        Ok(response)
    }

    async fn find_active(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, ServiceError> {
        info!(
            "🟢 Finding active users | Page: {}, Size: {}, Search: '{}'",
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
            "find_active",
            vec![
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "user:find_active:page:{page}:size:{page_size}:search:{}",
            search.clone().unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} active users in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method.clone(), &log_msg)
                .await;
            return Ok(cache);
        }

        let (users, total) = match self.query.find_active(req).await {
            Ok(res) => {
                let log_msg = format!("✅ Found {} active users", res.0.len());
                info!("{log_msg}");
                self.complete_tracing_success(&tracing_ctx, method.clone(), &log_msg)
                    .await;
                res
            }
            Err(e) => {
                error!("❌ Failed to fetch active users: {e:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    &format!("❌ Failed to fetch active users: {e:?}"),
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response: Vec<UserResponseDeleteAt> =
            users.into_iter().map(UserResponseDeleteAt::from).collect();

        let total_pages = ((total - 1) / req.page_size as i64) + 1;

        let pagination = Pagination {
            page: req.page,
            page_size: req.page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Active users retrieved successfully".to_string(),
            data: user_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!(
            "✅ Found {} active users (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_trashed(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, ServiceError> {
        info!(
            "🗑️ Finding trashed users | Page: {}, Size: {}, Search: '{}'",
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
            "find_trashed",
            vec![
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "user:find_trashed:page:{page}:size:{page_size}:search:{}",
            search.clone().unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} trashed users in cache", cache.data.len());
            info!("{}", log_msg);
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let (users, total) = match self.query.find_trashed(req).await {
            Ok(res) => {
                let log_msg = format!("✅ Found {} trashed users", res.0.len());
                info!("{log_msg}");
                self.complete_tracing_success(&tracing_ctx, method.clone(), &log_msg)
                    .await;
                res
            }
            Err(e) => {
                error!("❌ Failed to fetch trashed users: {e:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "❌ Failed to fetch trashed users",
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response: Vec<UserResponseDeleteAt> =
            users.into_iter().map(UserResponseDeleteAt::from).collect();

        let total_pages = ((total - 1) / req.page_size as i64) + 1;

        let pagination = Pagination {
            page: req.page,
            page_size: req.page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Trashed users retrieved successfully".to_string(),
            data: user_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!(
            "✅ Found {} trashed users (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("🆔 Finding user by ID: {id}");

        let method = Method::Get;

        let tracing_ctx =
            self.start_tracing("find_by_id", vec![KeyValue::new("id", id.to_string())]);

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("user:find_by_id:id:{id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<UserResponse>>(&cache_key)
            .await
        {
            info!("✅ Found user in cache");
            self.complete_tracing_success(&tracing_ctx, method, "User retrieved from cache")
                .await;
            return Ok(cache);
        }
        let user = match self.query.find_by_id(id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                error!("❌ Failed to find user by ID: {id}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "User not found")
                    .await;
                return Err(ServiceError::Custom(
                    "Failed to find user by ID".to_string(),
                ));
            }
            Err(e) => {
                error!("❌ Database error while finding user ID {id}: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let user_response = UserResponse::from(user);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "User retrieved successfully".to_string(),
            data: user_response,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!("✅ Found user: '{}' (ID: {id})", response.data.email);

        Ok(response)
    }

    async fn find_by_email(
        &self,
        email: String,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("📧 Finding user by email: {email}");

        let method = Method::Get;
        let tracing_ctx =
            self.start_tracing("find_by_email", vec![KeyValue::new("email", email.clone())]);

        let mut request = Request::new(email.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("user:find_by_email:{email}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<UserResponse>>(&cache_key)
            .await
        {
            info!("✅ Found user in cache (email)");
            self.complete_tracing_success(&tracing_ctx, method, "User from cache")
                .await;
            return Ok(cache);
        }

        let user = match self.query.find_by_email(email.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                error!("❌ No user found for email: {email}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "User not found")
                    .await;
                return Err(ServiceError::Custom("User not found".into()));
            }
            Err(e) => {
                error!("❌ DB error find_by_email({email}): {e:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".into(),
            message: "User retrieved successfully".into(),
            data: UserResponse::from(user),
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!("📧 User found: {}", response.data.email);

        Ok(response)
    }

    async fn find_by_email_and_verify(
        &self,
        email: String,
    ) -> Result<ApiResponse<UserResponseWithPassword>, ServiceError> {
        info!("🔐 Finding & verifying user by email: {email}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "find_by_email_and_verify",
            vec![KeyValue::new("email", email.clone())],
        );

        let mut request = Request::new(email.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("user:find_by_email_and_verify:{email}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<UserResponseWithPassword>>(&cache_key)
            .await
        {
            info!("✅ Found verified user in cache");
            self.complete_tracing_success(&tracing_ctx, method, "Verified user from cache")
                .await;
            return Ok(cache);
        }

        let user = match self.query.find_by_email_and_verify(email.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                error!("❌ User not found or not verified: {email}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "User not found")
                    .await;
                return Err(ServiceError::Custom("User not found or invalid".into()));
            }
            Err(e) => {
                error!("❌ DB error find_by_email_and_verify({email}): {e:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".into(),
            message: "User verified & retrieved successfully".into(),
            data: UserResponseWithPassword::from(user),
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!("🔐 Verified user found: {}", response.data.email);

        Ok(response)
    }

    async fn find_by_verification_code(
        &self,
        code: String,
    ) -> Result<ApiResponse<UserResponse>, ServiceError> {
        info!("📨 Finding user by verification code: {code}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "find_by_verification_code",
            vec![KeyValue::new("verification_code", code.clone())],
        );

        let mut request = Request::new(code.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("user:find_by_verification_code:{code}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<UserResponse>>(&cache_key)
            .await
        {
            info!("✅ Found user in cache (verification_code)");
            self.complete_tracing_success(&tracing_ctx, method, "User from cache")
                .await;
            return Ok(cache);
        }

        let user = match self.query.find_verify_code(code.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                error!("❌ Invalid verification code: {code}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Invalid code")
                    .await;
                return Err(ServiceError::Custom("Invalid verification code".into()));
            }
            Err(e) => {
                error!("❌ DB error find_by_verification_code({code}): {e:?}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".into(),
            message: "User retrieved successfully".into(),
            data: UserResponse::from(user),
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!("📨 User found: {}", response.data.email);

        Ok(response)
    }
}
