use crate::{
    abstract_trait::order_item::OrderItemGrpcClientTrait,
    domain::{
        requests::order_item::FindAllOrderItems,
        response::{
            api::{ApiResponse, ApiResponsePagination},
            order_item::{OrderItemResponse, OrderItemResponseDeleteAt},
        },
    },
};
use async_trait::async_trait;
use genproto::order_item::{
    FindAllOrderItemRequest, FindByIdOrderItemRequest,
    order_item_service_client::OrderItemServiceClient,
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
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct OrderItemGrpcClientService {
    client: Arc<Mutex<OrderItemServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl OrderItemGrpcClientService {
    pub async fn new(
        client: Arc<Mutex<OrderItemServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        registry.lock().await.register(
            "order_item_service_client_request_counter",
            "Total number of requests to the OrderItemGrpcClientService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "order_item_service_client_duration",
            "Histogram of request durations for the OrderItemGrpcClientService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("order_item-client-service")
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
impl OrderItemGrpcClientTrait for OrderItemGrpcClientService {
    async fn find_all(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponse>>, HttpError> {
        info!(
            "Retrieving all order items (page: {}, size: {}, search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllOrderItems",
            vec![
                KeyValue::new("component", "order_item"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllOrderItemRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.lock().await.find_all(request).await {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched order items",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_all failed for OrderItem: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();
        let order_items: Vec<OrderItemResponse> = inner.data.into_iter().map(Into::into).collect();
        let order_item_len = order_items.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: order_items,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {order_item_len} Order Items");
        Ok(reply)
    }
    async fn find_by_active(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving active order items (page: {}, size: {})",
            req.page, req.page_size
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindActiveOrderItems",
            vec![
                KeyValue::new("component", "order_item"),
                KeyValue::new("operation", "find_by_active"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
            ],
        );

        let mut request = Request::new(FindAllOrderItemRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.lock().await.find_by_active(request).await {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched active order items",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_by_active failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();
        let order_items: Vec<OrderItemResponseDeleteAt> =
            inner.data.into_iter().map(Into::into).collect();
        let order_item_len = order_items.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: order_items,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {order_item_len} active Order Items");
        Ok(reply)
    }

    async fn find_by_trashed(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving trashed order items (page: {}, size: {})",
            req.page, req.page_size
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindTrashedOrderItems",
            vec![
                KeyValue::new("component", "order_item"),
                KeyValue::new("operation", "find_by_trashed"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
            ],
        );

        let mut request = Request::new(FindAllOrderItemRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.lock().await.find_by_active(request).await {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched trashed order items",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_by_trashed failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();
        let order_items: Vec<OrderItemResponseDeleteAt> =
            inner.data.into_iter().map(Into::into).collect();
        let order_item_len = order_items.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: order_items,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {order_item_len} trashed Order Items");
        Ok(reply)
    }

    async fn find_order_item_by_order(
        &self,
        order_id: i32,
    ) -> Result<ApiResponse<Vec<OrderItemResponse>>, HttpError> {
        info!("Retrieving order items for order_id: {}", order_id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindOrderItemsByOrderId",
            vec![
                KeyValue::new("component", "order_item"),
                KeyValue::new("operation", "find_by_order_id"),
                KeyValue::new("order_id", order_id.to_string()),
            ],
        );

        let grpc_req = FindByIdOrderItemRequest { id: order_id };
        let mut request = Request::new(grpc_req);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .client
            .lock()
            .await
            .find_order_item_by_order(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched items for order",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_order_item_by_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();
        let order_items: Vec<OrderItemResponse> = inner.data.into_iter().map(Into::into).collect();
        let order_item_len = order_items.len();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: order_items,
        };

        info!(
            "Successfully fetched {order_item_len} Order Items for order_id {}",
            order_id
        );
        Ok(reply)
    }
}
