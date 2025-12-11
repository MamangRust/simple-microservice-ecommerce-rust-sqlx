use crate::{
    abstract_trait::order_item::{
        repository::DynOrderItemQueryRepository, service::OrderItemQueryServiceTrait,
    },
    domain::{
        requests::order_item::FindAllOrderItems,
        response::{
            api::{ApiResponse, ApiResponsePagination},
            order_item::{OrderItemResponse, OrderItemResponseDeleteAt},
            pagination::Pagination,
        },
    },
};
use async_trait::async_trait;
use chrono::Duration;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    cache::CacheStore,
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use anyhow::Result;
use std::sync::Arc;
use tokio::time::Instant;
use tonic::Request;
use tracing::{error, info};

#[derive(Clone)]
pub struct OrderItemQueryService {
    pub query: DynOrderItemQueryRepository,
    pub metrics: Metrics,
    pub cache_store: Arc<CacheStore>,
}

impl OrderItemQueryService {
    pub fn new(
        query: DynOrderItemQueryRepository,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let metrics = Metrics::new();

        registry.register(
            "order_item_query_service_request_counter",
            "Total number of requests to the OrderItemQueryService",
            metrics.request_counter.clone(),
        );
        registry.register(
            "order_item_query_service_request_duration",
            "Histogram of request durations for the OrderItemQueryService",
            metrics.request_duration.clone(),
        );

        Ok(Self {
            query,
            metrics,
            cache_store,
        })
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("order_item-query-service")
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
impl OrderItemQueryServiceTrait for OrderItemQueryService {
    async fn find_all_order_items(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponse>>, ServiceError> {
        info!(
            "🏷️ Finding all order items | Page: {}, Size: {}, Search: '{}'",
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
            "find_all_order_items",
            vec![
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order_item:find_all:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderItemResponse>>>(&cache_key)
        {
            let log_message = format!("✅ Found cached order items (total: {})", cache.data.len());
            info!("{log_message}");
            self.complete_tracing_success(&tracing_ctx, method, &log_message)
                .await;
            return Ok(cache);
        }

        let (order_items, total) = match self.query.find_all_order_items(req).await {
            Ok(res) => {
                let log_message = format!("Found {} order items in DB", res.0.len());
                info!("{}", log_message);
                self.complete_tracing_success(&tracing_ctx, method.clone(), &log_message)
                    .await;
                res
            }
            Err(e) => {
                let log_message = format!("❌ Failed to find order items: {e:?}");
                error!("{log_message}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &log_message)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_response: Vec<OrderItemResponse> = order_items
            .into_iter()
            .map(OrderItemResponse::from)
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
            message: "Order items retrieved successfully".to_string(),
            data: order_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));
        info!(
            "✅ Found {} order items (total: {total})",
            response.data.len()
        );

        Ok(response)
    }
    async fn find_by_active(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>, ServiceError> {
        info!(
            "✅ Finding all active order items | Page: {}, Size: {}, Search: '{}'",
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
            "find_by_active_order_items",
            vec![
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order_item:find_by_active:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>>(&cache_key)
        {
            let log_message = format!(
                "✅ Found cached active order items (total: {})",
                cache.data.len()
            );
            info!("{log_message}");
            self.complete_tracing_success(&tracing_ctx, method, &log_message)
                .await;
            return Ok(cache);
        }

        let (order_items, total) = match self.query.find_by_active(req).await {
            Ok(res) => {
                let log_message = format!("Found {} active order items in DB", res.0.len());
                info!("{}", log_message);
                self.complete_tracing_success(&tracing_ctx, method.clone(), &log_message)
                    .await;
                res
            }
            Err(e) => {
                let log_message = format!("❌ Failed to find active order items: {e:?}");
                error!("{log_message}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &log_message)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_response: Vec<OrderItemResponseDeleteAt> = order_items
            .into_iter()
            .map(OrderItemResponseDeleteAt::from)
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
            message: "Active order items retrieved successfully".to_string(),
            data: order_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));
        info!(
            "✅ Found {} active order items (total: {total})",
            response.data.len()
        );

        Ok(response)
    }

    async fn find_by_trashed(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>, ServiceError> {
        info!(
            "🗑️ Finding all trashed order items | Page: {}, Size: {}, Search: '{}'",
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
            "find_by_trashed_order_items",
            vec![
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order_item:find_by_trashed:page:{page}:size:{page_size}:search:{}",
            search.unwrap_or_default()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>>(&cache_key)
        {
            let log_message = format!(
                "✅ Found cached trashed order items (total: {})",
                cache.data.len()
            );
            info!("{log_message}");
            self.complete_tracing_success(&tracing_ctx, method, &log_message)
                .await;
            return Ok(cache);
        }

        let (order_items, total) = match self.query.find_by_trashed(req).await {
            Ok(res) => {
                let log_message = format!("Found {} trashed order items in DB", res.0.len());
                info!("{}", log_message);
                self.complete_tracing_success(&tracing_ctx, method.clone(), &log_message)
                    .await;
                res
            }
            Err(e) => {
                let log_message = format!("❌ Failed to find trashed order items: {e:?}");
                error!("{log_message}");
                self.complete_tracing_error(&tracing_ctx, method.clone(), &log_message)
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_response: Vec<OrderItemResponseDeleteAt> = order_items
            .into_iter()
            .map(OrderItemResponseDeleteAt::from)
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
            message: "Trashed order items retrieved successfully".to_string(),
            data: order_response,
            pagination,
        };

        self.cache_store
            .set_to_cache(&cache_key, &response, Duration::minutes(5));
        info!(
            "✅ Found {} trashed order items (total: {total})",
            response.data.len()
        );

        Ok(response)
    }
    async fn find_order_item_by_order(
        &self,
        order_id: i32,
    ) -> Result<ApiResponse<Vec<OrderItemResponse>>, ServiceError> {
        info!("📦 Finding items for order {order_id}");

        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "find_order_item_by_order",
            vec![KeyValue::new("order_id", order_id.to_string())],
        );

        let result = match self.query.find_order_item_by_order(order_id).await {
            Ok(items) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched order items",
                )
                .await;
                items
            }
            Err(e) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    &format!("Failed to fetch items: {e:?}"),
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let response_items: Vec<OrderItemResponse> =
            result.into_iter().map(OrderItemResponse::from).collect();

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Order items retrieved successfully".to_string(),
            data: response_items,
        };

        Ok(response)
    }
}
