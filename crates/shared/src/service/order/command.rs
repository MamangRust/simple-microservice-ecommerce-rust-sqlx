use crate::{
    abstract_trait::{
        DynKafka, DynOrderCommandRepository, DynOrderQueryRepository, DynProductQueryRepository,
        OrderCommandServiceTrait,
    },
    domain::{
        event::OrderEvent,
        requests::{CreateOrderRequest, UpdateOrderRequest},
        responses::{ApiResponse, OrderResponse, OrderResponseDeleteAt},
    },
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
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

pub struct OrderCommandService {
    pub product_query: DynProductQueryRepository,
    pub command: DynOrderCommandRepository,
    pub query: DynOrderQueryRepository,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
}

pub struct OrderCommandServiceDeps {
    pub product_query: DynProductQueryRepository,
    pub command: DynOrderCommandRepository,
    pub query: DynOrderQueryRepository,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
}

impl OrderCommandService {
    pub async fn new(deps: OrderCommandServiceDeps) -> Self {
        let OrderCommandServiceDeps {
            product_query,
            command,
            query,
            kafka,
            metrics,
            registry,
        } = deps;

        registry.lock().await.register(
            "order_service_request_counter",
            "Total number of requests to the OrderService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "order_service_request_duration",
            "Histogram of request durations for the OrderService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            product_query,
            command,
            query,
            kafka,
            metrics,
        }
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("order-command-service")
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
impl OrderCommandServiceTrait for OrderCommandService {
    async fn create_order(
        &self,
        req: &CreateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, ServiceError> {
        info!("🏗️ Creating new order for product_id={}", req.product_id);

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "create_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "create"),
                KeyValue::new("order.product_id", req.product_id.to_string()),
                KeyValue::new("order.quantity", req.quantity.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let product = match self.product_query.find_by_id(req.product_id).await {
            Ok(Some(product)) => {
                info!("✅ Product found with ID={}", req.product_id);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched product",
                )
                .await;
                product
            }
            Ok(None) => {
                error!("❌ Product not found with ID={}", req.product_id);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Product not found")
                    .await;
                return Err(ServiceError::Custom("Product not found".to_string()));
            }
            Err(e) => {
                error!("❌ Failed to fetch product: {e:?}");

                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to fetch product",
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        if req.quantity > product.stock {
            error!(
                "❌ Not enough stock for product_id={}, requested={}, available={}",
                req.product_id, req.quantity, product.stock
            );
            return Err(ServiceError::Custom("Insufficient stock".to_string()));
        }

        let total_price = req.quantity as i64 * product.price;

        let order_model = match self.command.create_order(req, total_price).await {
            Ok(order) => {
                info!("✅ Order created with ID={}", order.order_id);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully created order",
                )
                .await;

                order
            }

            Err(e) => {
                error!("❌ Failed to create order: {e:?}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to create order")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let mut response = OrderResponse::from(order_model);
        response.total = total_price;

        let event = OrderEvent::Created {
            order_id: response.id,
            product_id: response.product_id,
            quantity: response.quantity,
        };

        let payload = serde_json::to_vec(&event)
            .map_err(|e| ServiceError::Custom(format!("Failed to serialize event: {e}")))?;

        if let Err(e) = self
            .kafka
            .publish("order.created", &response.id.to_string(), &payload)
            .await
        {
            error!("❌ Failed to publish order.created event: {e:?}");
        } else {
            info!(
                "📤 Published event: order.created | order_id={} product_id={} quantity={} total_price={}",
                response.id, response.product_id, response.quantity, total_price
            );
        }

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Order created successfully".to_string(),
            data: response,
        })
    }

    async fn update_order(
        &self,
        req: &UpdateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, ServiceError> {
        info!("✏️ Updating order ID={}", req.id);

        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "update_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "update"),
                KeyValue::new("order_id", req.id.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let old_older = match self.query.find_by_id(req.id).await {
            Ok(Some(order)) => {
                info!("✅ Order found with ID={}", req.id);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched order",
                )
                .await;

                order
            }
            Ok(None) => {
                error!("❌ Order not found with ID={}", req.id);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Order not found")
                    .await;
                return Err(ServiceError::Custom("Order not found".to_string()));
            }
            Err(e) => {
                error!("❌ Failed to fetch order: {e:?}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to fetch order")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let old_quantity = old_older.quantity;

        let product = match self.product_query.find_by_id(req.product_id).await {
            Ok(Some(product)) => {
                info!("✅ Product found with ID={}", req.product_id);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched product",
                )
                .await;
                product
            }
            Ok(None) => {
                error!("❌ Product not found with ID={}", req.product_id);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Product not found")
                    .await;
                return Err(ServiceError::Custom("Product not found".to_string()));
            }
            Err(e) => {
                error!("❌ Failed to fetch product: {e:?}");

                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to fetch product",
                )
                .await;
                return Err(ServiceError::Repo(e));
            }
        };

        if req.quantity > product.stock {
            error!(
                "❌ Not enough stock for product_id={}, requested={}, available={}",
                product.product_id, req.quantity, product.stock
            );
            return Err(ServiceError::Custom("Insufficient stock".to_string()));
        }

        let total_price = req.quantity as i64 * product.price;

        let order_model = match self.command.update_order(req, total_price).await {
            Ok(order) => {
                info!("✅ Order updated with ID={}", order.order_id);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully updated order",
                )
                .await;

                order
            }
            Err(e) => {
                error!("❌ Failed to update order ID={}: {e:?}", req.id);

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to update order")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let mut response = OrderResponse::from(order_model);
        response.total = total_price;

        if req.quantity != old_quantity {
            let event = OrderEvent::Updated {
                order_id: response.id,
                product_id: response.product_id,
                old_quantity,
                new_quantity: req.quantity,
            };

            let payload = serde_json::to_vec(&event)
                .map_err(|e| ServiceError::Custom(format!("Failed to serialize event: {e}")))?;

            if let Err(e) = self
                .kafka
                .publish("order.updated", &response.id.to_string(), &payload)
                .await
            {
                error!("❌ Failed to publish order.updated event: {e:?}");

                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to publish order.updated event",
                )
                .await;

                return Err(ServiceError::Kafka(e.to_string()));
            } else {
                info!(
                    "📤 Published event: order.updated | order_id={} old={} new={} total_price={total_price}",
                    response.id, old_quantity, req.quantity,
                );

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully published order.updated event",
                )
                .await;
            }
        } else {
            info!("🔁 Quantity not changed, skipping event publish");
        }

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Order updated successfully".to_string(),
            data: response,
        })
    }

    async fn trash_order(
        &self,
        order_id: i32,
    ) -> Result<ApiResponse<OrderResponseDeleteAt>, ServiceError> {
        info!("🗑️ Soft deleting Order with ID: {order_id}");

        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "trash_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("order.id", order_id.to_string()),
            ],
        );

        let mut request = Request::new(order_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let order_model = match self.command.trash_order(order_id).await {
            Ok(order) => {
                info!("✅ Order found with ID={order_id}");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched order",
                )
                .await;

                order
            }
            Err(e) => {
                error!("❌ Failed to fetch order ID={order_id}: {e:?}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to fetch order")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let response = OrderResponseDeleteAt::from(order_model);

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Order moved to trash successfully".to_string(),
            data: response,
        })
    }

    async fn restore_order(
        &self,
        order_id: i32,
    ) -> Result<ApiResponse<OrderResponse>, ServiceError> {
        info!("🔄 Restoring Order with ID: {}", order_id);

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "restore_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("order_id", order_id.to_string()),
            ],
        );

        let mut request = Request::new(order_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let order_model = match self.command.restore_order(order_id).await {
            Ok(order) => {
                info!("✅ Order found with ID={order_id}");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched order",
                )
                .await;

                order
            }
            Err(e) => {
                error!("❌ Failed to fetch order ID={order_id}: {e:?}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to fetch order")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        let response = OrderResponse::from(order_model);

        info!(
            "✅ Order restored: {} (ID: {order_id})",
            response.product_id,
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Order restored successfully".to_string(),
            data: response,
        })
    }

    async fn delete_order(&self, order_id: i32) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting Order with ID: {order_id}");

        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "delete_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("order_id", order_id.to_string()),
            ],
        );

        let mut request = Request::new(order_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let order_model = match self.query.find_by_id(order_id).await {
            Ok(Some(order)) => {
                info!("✅ Order found with ID={order_id}");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Successfully fetched order",
                )
                .await;

                order
            }
            Ok(None) => {
                error!("❌ Order not found with ID={order_id}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Order not found")
                    .await;

                return Err(ServiceError::Custom("Order not found".to_string()));
            }
            Err(e) => {
                error!("❌ Failed to fetch order ID={order_id}: {e:?}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to fetch order")
                    .await;

                return Err(ServiceError::Repo(e));
            }
        };

        if let Err(e) = self.command.delete_order(order_id).await {
            error!("❌ Failed to delete Order ID {order_id}: {e:?}");

            self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to delete order")
                .await;

            return Err(ServiceError::Repo(e));
        };

        let event = OrderEvent::Deleted {
            order_id,
            product_id: order_model.product_id,
            quantity: order_model.quantity,
        };

        let payload = serde_json::to_vec(&event)
            .map_err(|e| ServiceError::Custom(format!("Failed to serialize event: {e}")))?;

        if let Err(e) = self
            .kafka
            .publish("order.deleted", &order_id.to_string(), &payload)
            .await
        {
            error!("❌ Failed to publish event: {e:?}");
            self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to publish event")
                .await;

            return Err(ServiceError::Kafka(e.to_string()));
        } else {
            info!(
                "📤 Published event: order.deleted | order_id={order_id} product_id={} quantity={}",
                order_model.product_id, order_model.quantity
            );

            self.complete_tracing_success(
                &tracing_ctx,
                method.clone(),
                "Successfully published event",
            )
            .await;
        }

        info!("✅ Order permanently deleted: {order_id}");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Order deleted permanently".to_string(),
            data: (),
        })
    }

    async fn restore_all_order(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("🔄 Restoring all soft-deleted Orders");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "restore_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "restore_all_order"),
            ],
        );

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        if let Err(e) = self.command.restore_all_orders().await {
            error!("❌ Failed to restore all Orders: {e:?}");

            self.complete_tracing_error(
                &tracing_ctx,
                method.clone(),
                "Failed to restore all Orders",
            )
            .await;

            return Err(ServiceError::Repo(e));
        };

        info!("✅ All Orders restored successfully");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "All Orders restored successfully".to_string(),
            data: (),
        })
    }

    async fn delete_all_order(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting all Orders");

        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "restore_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "delete_all_order"),
            ],
        );

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        if let Err(e) = self.command.delete_all_orders().await {
            error!("❌ Failed to delete all Orders: {e:?}");

            self.complete_tracing_error(
                &tracing_ctx,
                method.clone(),
                "Failed to delete all Orders",
            )
            .await;

            return Err(ServiceError::Repo(e));
        };

        info!("✅ All Orders deleted permanently");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "All Orders deleted permanently".to_string(),
            data: (),
        })
    }
}
