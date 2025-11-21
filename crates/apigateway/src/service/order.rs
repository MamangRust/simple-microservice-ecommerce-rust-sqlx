use crate::{
    abstract_trait::order::OrderGrpcClientTrait,
    domain::{
        requests::order::{
            CreateOrderRequest as DomainCreateOrderRequest, FindAllOrder as DomainFindAllOrders,
            UpdateOrderRequest as DomainUpdateOrderRequest,
        },
        response::{
            api::{ApiResponse, ApiResponsePagination},
            order::{OrderResponse, OrderResponseDeleteAt},
        },
    },
};
use async_trait::async_trait;
use genproto::order::{
    CreateOrderItemRequest, CreateOrderRequest, FindAllOrderRequest, FindByIdOrderRequest,
    UpdateOrderItemRequest, UpdateOrderRequest,
    order_command_service_client::OrderCommandServiceClient,
    order_query_service_client::OrderQueryServiceClient,
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
pub struct OrderGrpcClientService {
    query_client: Arc<Mutex<OrderQueryServiceClient<Channel>>>,
    command_client: Arc<Mutex<OrderCommandServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl OrderGrpcClientService {
    pub async fn new(
        query_client: Arc<Mutex<OrderQueryServiceClient<Channel>>>,
        command_client: Arc<Mutex<OrderCommandServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        registry.lock().await.register(
            "order_service_client_request_counter",
            "Total number of requests to the OrderGrpcClientService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "order_service_client_duration",
            "Histogram of request durations for the OrderGrpcClientService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            query_client,
            command_client,
            metrics,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("order-client-service")
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
impl OrderGrpcClientTrait for OrderGrpcClientService {
    async fn find_all(
        &self,
        req: &DomainFindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, HttpError> {
        info!(
            "Retrieving all order (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllOrders",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllOrderRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.lock().await.find_all(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully fetched orders")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_all failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let orders: Vec<OrderResponse> = inner.data.into_iter().map(Into::into).collect();

        let order_len = orders.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: orders,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {order_len} Orders");
        Ok(reply)
    }

    async fn find_active(
        &self,
        req: &DomainFindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving all active order (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllActiveOrders",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllOrderRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.lock().await.find_by_active(request).await {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched active orders",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_active failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let orders: Vec<OrderResponseDeleteAt> = inner.data.into_iter().map(Into::into).collect();

        let orders_len = orders.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: orders,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {orders_len} active Orders");
        Ok(reply)
    }

    async fn find_trashed(
        &self,
        req: &DomainFindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving all trashed order (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllTrashedOrders",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllOrderRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .query_client
            .lock()
            .await
            .find_by_trashed(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched trashed orders",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_trashed failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let orders: Vec<OrderResponseDeleteAt> = inner.data.into_iter().map(Into::into).collect();

        let orders_len = orders.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: orders,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {orders_len} trashed Orders");
        Ok(reply)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<OrderResponse>, HttpError> {
        info!("Fetching Order by ID: {}", id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindByIdOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("saldo.id", id as i64),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.lock().await.find_by_id(request).await {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched Order by ID",
                )
                .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC find_by_id failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let order_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Order data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_order: OrderResponse = order_data.into();

        let order_id = domain_order.clone().id;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order.clone(),
        };

        info!("Successfully fetched Order: {order_id}");
        Ok(reply)
    }

    async fn create_order(
        &self,
        req: &DomainCreateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, HttpError> {
        info!("Creating new Order: {}", req.user_id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "create"),
                KeyValue::new("user_id", req.user_id.to_string()),
            ],
        );

        if req.items.is_empty() {
            return Err(HttpError::BadRequest("Items cannot be empty".into()));
        }

        let mut request = Request::new(CreateOrderRequest {
            user_id: req.user_id,
            items: req
                .items
                .iter()
                .cloned()
                .map(|i| CreateOrderItemRequest {
                    price: i.price,
                    product_id: i.product_id,
                    quantity: i.quantity,
                })
                .collect(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.create(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Order created")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC create_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let order_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Order data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_order: OrderResponse = order_data.into();

        let order_id = domain_order.id;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {order_id} created successfully");
        Ok(reply)
    }

    async fn update_order(
        &self,
        req: &DomainUpdateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, HttpError> {
        info!("Updating Order: {:?}", req.order_id);

        let order_id = req
            .order_id
            .ok_or_else(|| HttpError::BadRequest("order_id is required".into()))?;

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "update"),
                KeyValue::new("order_id", order_id.to_string()),
                KeyValue::new("product_id", req.user_id.to_string()),
            ],
        );

        if req.items.is_empty() {
            return Err(HttpError::BadRequest("Items cannot be empty".into()));
        }

        let request = Request::new(UpdateOrderRequest {
            order_id: order_id,
            user_id: req.user_id,
            items: req
                .items
                .iter()
                .cloned()
                .map(|i| UpdateOrderItemRequest {
                    order_item_id: i.order_item_id,
                    price: i.price,
                    product_id: i.product_id,
                    quantity: i.quantity,
                })
                .collect(),
        });

        let response = match self.command_client.lock().await.update(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Order updated")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC update_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let order_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Order data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_order: OrderResponse = order_data.into();

        let order_id = domain_order.clone().id;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {order_id} updated successfully");
        Ok(reply)
    }

    async fn trash_order(&self, id: i32) -> Result<ApiResponse<OrderResponseDeleteAt>, HttpError> {
        info!("Soft deleting Order: {id}");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "TrashOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("order_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.trashed(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Order soft deleted")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC trash_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let order_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Order data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_order: OrderResponseDeleteAt = order_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {} soft deleted successfully", id);
        Ok(reply)
    }

    async fn restore_order(
        &self,
        id: i32,
    ) -> Result<ApiResponse<OrderResponseDeleteAt>, HttpError> {
        info!("Restoring Order: {}", id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("order_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.restore(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Order restored")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC restore_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let order_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Order data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_order: OrderResponseDeleteAt = order_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {} restored successfully", id);
        Ok(reply)
    }

    async fn delete_order(&self, id: i32) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting Order: {}", id);

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("order_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdOrderRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .lock()
            .await
            .delete_order_permanent(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Order permanently deleted")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC delete_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
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

        info!("Order {} permanently deleted", id);
        Ok(reply)
    }

    async fn restore_all_order(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Restoring all trashed Orders");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreAllOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "restore"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .lock()
            .await
            .restore_all_order(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Orders restored")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC restore_all_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
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

        info!("All Orders restored successfully");
        Ok(reply)
    }

    async fn delete_all_order(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting all trashed Orders");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteAllOrder",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "delete"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .lock()
            .await
            .delete_all_order(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Orders permanently deleted")
                    .await;
                resp
            }
            Err(status) => {
                let error_message = format!(
                    "gRPC delete_all_order failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, &error_message)
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

        info!("All trashed Orders permanently deleted");
        Ok(reply)
    }
}
