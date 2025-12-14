use crate::{
    abstract_trait::product::{
        repository::DynProductQueryRepository, service::ProductQueryServiceTrait,
    },
    domain::{
        requests::product::FindAllProducts,
        response::{
            api::{ApiResponse, ApiResponsePagination},
            pagination::Pagination,
            product::{ProductResponse, ProductResponseDeleteAt},
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
pub struct ProductQueryService {
    pub query: DynProductQueryRepository,
    pub metrics: Metrics,
    pub cache_store: Arc<CacheStore>,
}

impl ProductQueryService {
    pub fn new(query: DynProductQueryRepository, cache_store: Arc<CacheStore>) -> Result<Self> {
        let metrics = Metrics::new(global::meter("product-query-service"));

        Ok(Self {
            query,
            metrics,
            cache_store,
        })
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("product-query-service")
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
impl ProductQueryServiceTrait for ProductQueryService {
    async fn find_all(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponse>>, ServiceError> {
        info!(
            "🔍 Finding all products | Page: {}, Size: {}, Search: '{}'",
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
            "product_find_all",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "product:find_all:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<ProductResponse>>>(&cache_key)
            .await
        {
            info!("✅ Found {} products in cache", cached.data.len());
            self.complete_tracing_success(&tracing_ctx, method, "Products retrieved from cache")
                .await;
            return Ok(cached);
        }

        let (products, total) = match self.query.find_all(req).await {
            Ok(res) => {
                info!("✅ Retrieved {} products from DB", res.0.len());
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Products retrieved from DB",
                )
                .await;
                res
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch all products: {e:?}");
                error!("{}", msg);
                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<ProductResponse> = products.into_iter().map(ProductResponse::from).collect();
        let total_pages = ((total - 1) / page_size as i64) + 1;

        let pagination = Pagination {
            page,
            page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Products retrieved successfully".to_string(),
            data,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5))
            .await;

        info!("✅ Found {} products (total: {total})", response.data.len());

        Ok(response)
    }

    async fn find_active(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, ServiceError> {
        info!(
            "🟢 Finding active products | Page: {}, Size: {}, Search: '{}'",
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
            "product_find_active",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "product:find_active:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<ProductResponseDeleteAt>>>(&cache_key)
            .await
        {
            info!("✅ Found {} active products in cache", cached.data.len());
            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "Active products retrieved from cache",
            )
            .await;
            return Ok(cached);
        }

        let (products, total) = match self.query.find_active(req).await {
            Ok(res) => {
                info!("✅ Retrieved {} active products from DB", res.0.len());
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Active products retrieved from DB",
                )
                .await;
                res
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch active products: {e:?}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<ProductResponseDeleteAt> = products
            .into_iter()
            .map(ProductResponseDeleteAt::from)
            .collect();
        let total_pages = ((total - 1) / page_size as i64) + 1;

        let pagination = Pagination {
            page,
            page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Active products retrieved successfully".to_string(),
            data,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5))
            .await;

        info!(
            "✅ Found {} active products (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_trashed(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, ServiceError> {
        info!(
            "🗑️ Finding trashed products | Page: {}, Size: {}, Search: '{}'",
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
            "product_find_trashed",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "product:find_trashed:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<ProductResponseDeleteAt>>>(&cache_key)
            .await
        {
            info!("✅ Found {} trashed products in cache", cached.data.len());
            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "Trashed products retrieved from cache",
            )
            .await;
            return Ok(cached);
        }

        let (products, total) = match self.query.find_trashed(req).await {
            Ok(res) => {
                info!("✅ Retrieved {} trashed products from DB", res.0.len());
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Trashed products retrieved from DB",
                )
                .await;
                res
            }
            Err(e) => {
                let msg = format!("❌ Failed to fetch trashed products: {e:?}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let data: Vec<ProductResponseDeleteAt> = products
            .into_iter()
            .map(ProductResponseDeleteAt::from)
            .collect();
        let total_pages = ((total - 1) / page_size as i64) + 1;

        let pagination = Pagination {
            page,
            page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Trashed products retrieved successfully".to_string(),
            data,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5))
            .await;

        info!(
            "✅ Found {} trashed products (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, ServiceError> {
        info!("🆔 Finding product by ID: {id}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "product_find_by_id",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("product.id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("product:find_by_id:id:{id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<ProductResponse>>(&cache_key)
            .await
        {
            info!("✅ Found product ID {} in cache", id);
            self.complete_tracing_success(&tracing_ctx, method, "Product retrieved from cache")
                .await;
            return Ok(cached);
        }

        let product = match self.query.find_by_id(id).await {
            Ok(Some(product)) => {
                info!("✅ Found product in DB: '{}' (ID: {id})", product.name);
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Product retrieved from DB",
                )
                .await;
                product
            }
            Ok(None) => {
                let msg = format!("❌ Product not found with ID: {id}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Product not found")
                    .await;
                return Err(ServiceError::Custom("Product not found".to_string()));
            }
            Err(e) => {
                let msg = format!("❌ Database error while finding product ID {id}: {e:?}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Product retrieved successfully".to_string(),
            data: ProductResponse::from(product),
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5))
            .await;

        info!("✅ Product retrieved: '{}' (ID: {id})", response.data.name);

        Ok(response)
    }
}
