use crate::{
    abstract_trait::order::{repository::DynOrderQueryRepository, service::OrderQueryServiceTrait},
    domain::{
        requests::order::FindAllOrder,
        response::{
            api::{ApiResponse, ApiResponsePagination},
            order::{OrderResponse, OrderResponseDeleteAt},
            pagination::Pagination,
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
pub struct OrderQueryService {
    pub query: DynOrderQueryRepository,
    pub metrics: Metrics,
    pub cache_store: Arc<CacheStore>,
}

impl OrderQueryService {
    pub fn new(query: DynOrderQueryRepository, cache_store: Arc<CacheStore>) -> Result<Self> {
        let metrics = Metrics::new(global::meter("order-query-service"));

        Ok(Self {
            query,
            metrics,
            cache_store,
        })
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("order-query-service")
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
impl OrderQueryServiceTrait for OrderQueryService {
    async fn find_all(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, ServiceError> {
        info!(
            "📦 Finding all orders | Page: {}, Size: {}, Search: '{}'",
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
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order:find_all:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderResponse>>>(&cache_key)
            .await
        {
            let log_message = format!("✅ Found cached orders (total: {})", cache.data.len());
            info!("{log_message}");
            self.complete_tracing_success(&tracing_ctx, method, &log_message)
                .await;
            return Ok(cache);
        }

        let (orders, total) = match self.query.find_all(req).await {
            Ok(res) => {
                let log_message = format!("Found {} orders", res.0.len());
                info!("{}", log_message);
                self.complete_tracing_success(&tracing_ctx, method.clone(), &log_message)
                    .await;
                res
            }
            Err(e) => {
                let log_message = format!("❌ Failed to find orders: {e:?}");
                error!("{log_message}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &log_message)
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let order_response: Vec<OrderResponse> =
            orders.into_iter().map(OrderResponse::from).collect();

        let total_pages = ((total - 1) / req.page_size as i64) + 1;

        let pagination = Pagination {
            page: req.page,
            page_size: req.page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Orders retrieved successfully".to_string(),
            data: order_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!("✅ Found {} orders (total: {total})", response.data.len());

        Ok(response)
    }

    async fn find_active(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, ServiceError> {
        info!(
            "🟢 Finding active orders | Page: {}, Size: {}, Search: '{}'",
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
                KeyValue::new("component", "find_active"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order:find_active:page:{page}:size:{page_size}:search:{}",
            search.clone().unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_message = format!(
                "✅ Found cached active orders (total: {})",
                cache.data.len()
            );
            info!("{log_message}");
            self.complete_tracing_success(&tracing_ctx, method, &log_message)
                .await;
            return Ok(cache);
        }

        let (orders, total) = match self.query.find_active(req).await {
            Ok(res) => {
                let log_msg = format!("✅ Found {} active orders (total: {})", res.0.len(), res.1);
                info!("{}", log_msg);
                self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                    .await;
                res
            }
            Err(e) => {
                error!("❌ Failed to fetch all order active: {e:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    &format!("❌ Failed to fetch all order active: {e:?}"),
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_response: Vec<OrderResponseDeleteAt> = orders
            .into_iter()
            .map(OrderResponseDeleteAt::from)
            .collect();

        let total_pages = ((total - 1) / req.page_size as i64) + 1;

        let pagination = Pagination {
            page: req.page,
            page_size: req.page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Active orders retrieved successfully".to_string(),
            data: order_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!(
            "✅ Found {} active orders (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_trashed(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, ServiceError> {
        info!(
            "🗑️ Finding trashed orders | Page: {}, Size: {}, Search: '{}'",
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
                KeyValue::new("component", "find_trashed"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order:find_trashed:page:{page}:size:{page_size}:search:{}",
            search.clone().unwrap()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_message = format!(
                "✅ Found cached trashed orders (total: {})",
                cache.data.len()
            );
            info!("{log_message}");
            self.complete_tracing_success(&tracing_ctx, method, &log_message)
                .await;
            return Ok(cache);
        }

        let (orders, total) = match self.query.find_trashed(req).await {
            Ok(res) => {
                let log_msg = format!("✅ Found {} trashed orders (total: {})", res.0.len(), res.1);
                info!("{log_msg}");
                self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                    .await;
                res
            }
            Err(e) => {
                error!("❌ Failed to fetch all order trashed: {e:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    &format!("❌ Failed to fetch all order trashed: {e:?}"),
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_response: Vec<OrderResponseDeleteAt> = orders
            .into_iter()
            .map(OrderResponseDeleteAt::from)
            .collect();

        let total_pages = ((total - 1) / req.page_size as i64) + 1;

        let pagination = Pagination {
            page: req.page,
            page_size: req.page_size,
            total_items: total as i32,
            total_pages: total_pages as i32,
        };

        let response = ApiResponsePagination {
            status: "success".to_string(),
            message: "Trashed orders retrieved successfully".to_string(),
            data: order_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response.clone(), Duration::minutes(5))
            .await;

        info!(
            "✅ Found {} trashed orders (total: {total})",
            response.data.len(),
        );

        Ok(response)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<OrderResponse>, ServiceError> {
        info!("🆔 Finding order by ID: {id}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "find_by_id",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_by_id"),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("order:find_by_id:id:{id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<OrderResponse>>(&cache_key)
            .await
        {
            info!("✅ Found order in cache");
            self.complete_tracing_success(&tracing_ctx, method, "Order retrieved from cache")
                .await;
            return Ok(cache);
        }

        let order = match self.query.find_by_id(id).await {
            Ok(Some(order)) => {
                info!("✅ Found order by ID: {id}");
                self.complete_tracing_success(&tracing_ctx, method, "Order retrieved")
                    .await;
                order
            }
            Ok(None) => {
                error!("❌ Failed to find order by ID: {id}");
                self.complete_tracing_error(&tracing_ctx, method, "Order not found")
                    .await;
                return Err(ServiceError::Custom(
                    "Failed to find order by ID".to_string(),
                ));
            }
            Err(e) => {
                error!("❌ Database error while finding order ID {id}: {e:?}");
                self.complete_tracing_error(&tracing_ctx, method, "Database error")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_response = OrderResponse::from(order);

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Order retrieved successfully".to_string(),
            data: order_response,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5))
            .await;

        info!(
            "✅ Found order: ID={id}, total_price={}",
            response.data.total_price
        );

        Ok(response)
    }
}
