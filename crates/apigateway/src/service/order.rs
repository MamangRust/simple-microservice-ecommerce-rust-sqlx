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
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
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
pub struct OrderGrpcClientService {
    query_client: OrderQueryServiceClient<Channel>,
    command_client: OrderCommandServiceClient<Channel>,
    metrics: Metrics,
    cache_store: Arc<CacheStore>,
}

impl OrderGrpcClientService {
    pub fn new(
        query_client: OrderQueryServiceClient<Channel>,
        command_client: OrderCommandServiceClient<Channel>,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let metrics = Metrics::new(global::meter("order-client-service"));

        Ok(Self {
            query_client,
            command_client,
            metrics,
            cache_store,
        })
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

        self.metrics.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl OrderGrpcClientTrait for OrderGrpcClientService {
    async fn find_all(
        &self,
        req: &DomainFindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all order (page: {page}, size: {page_size} search: {})",
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

        let mut request = Request::new(FindAllOrderRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order:find_all:page:{page}:size:{page_size}:search:{}",
            req.search.clone(),
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderResponse>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} roles in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_all(request).await {
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

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: orders,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched {order_len} Orders");
        Ok(api_response)
    }

    async fn find_active(
        &self,
        req: &DomainFindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all active order (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllActiveOrders",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllOrderRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order:find_active:page:{page}:size:{page_size}:search:{}",
            req.search.clone(),
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} roles in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_active(request).await {
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

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: orders,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched {orders_len} active Orders");
        Ok(api_response)
    }

    async fn find_trashed(
        &self,
        req: &DomainFindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all trashed order (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllTrashedOrders",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllOrderRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "order:find_trashed:page:{page}:size:{page_size}:search:{}",
            req.search.clone(),
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<OrderResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} roles in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_trashed(request).await {
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

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: orders,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched {orders_len} trashed Orders");
        Ok(api_response)
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

        let cache_key = format!("order:find_by_id:id:{id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<OrderResponse>>(&cache_key)
            .await
        {
            info!("✅ Found role in cache");
            self.complete_tracing_success(&tracing_ctx, method, "Role retrieved from cache")
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_id(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order.clone(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched Order: {order_id}");
        Ok(api_response)
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
                .map(|i| CreateOrderItemRequest {
                    price: i.price,
                    product_id: i.product_id,
                    quantity: i.quantity,
                })
                .collect(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().create(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {order_id} created successfully");
        Ok(api_response)
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
            order_id,
            user_id: req.user_id,
            items: req
                .items
                .iter()
                .map(|i| UpdateOrderItemRequest {
                    order_item_id: i.order_item_id,
                    price: i.price,
                    product_id: i.product_id,
                    quantity: i.quantity,
                })
                .collect(),
        });

        let response = match self.command_client.clone().update(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {order_id} updated successfully");
        Ok(api_response)
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

        let response = match self.command_client.clone().trashed(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {} soft deleted successfully", id);
        Ok(api_response)
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

        let response = match self.command_client.clone().restore(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_order,
        };

        info!("Order {} restored successfully", id);
        Ok(api_response)
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
            .clone()
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("Order {} permanently deleted", id);
        Ok(api_response)
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

        let response = match self.command_client.clone().restore_all_order(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All Orders restored successfully");
        Ok(api_response)
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

        let response = match self.command_client.clone().delete_all_order(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All trashed Orders permanently deleted");
        Ok(api_response)
    }
}
