use crate::{
    abstract_trait::{
        grpc_client::DynProductGrpcClient,
        order::{
            repository::{DynOrderCommandRepository, DynOrderQueryRepository},
            service::OrderCommandServiceTrait,
        },
        order_item::repository::{DynOrderItemCommandRepository, DynOrderItemQueryRepository},
    },
    domain::{
        event::{OrderEvent, OrderItemEvent, OrderItemUpdateEvent},
        requests::{
            order::{
                CreateOrderRecordRequest, CreateOrderRequest, UpdateOrderRecordRequest,
                UpdateOrderRequest,
            },
            order_item::{CreateOrderItemRecordRequest, UpdateOrderItemRecordRequest},
        },
        response::{
            api::ApiResponse,
            order::{OrderResponse, OrderResponseDeleteAt},
        },
    },
};
use shared::{
    abstract_trait::DynKafka,
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

#[derive(Clone)]
pub struct OrderCommandService {
    product_client: DynProductGrpcClient,
    command: DynOrderCommandRepository,
    order_item_query: DynOrderItemQueryRepository,
    order_item_command: DynOrderItemCommandRepository,
    query: DynOrderQueryRepository,
    kafka: DynKafka,
    metrics: Arc<Mutex<Metrics>>,
}

pub struct OrderCommandServiceDeps {
    pub product_client: DynProductGrpcClient,
    pub order_item_query: DynOrderItemQueryRepository,
    pub order_item_command: DynOrderItemCommandRepository,
    pub command: DynOrderCommandRepository,
    pub query: DynOrderQueryRepository,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
}

impl OrderCommandService {
    pub async fn new(deps: OrderCommandServiceDeps) -> Self {
        let OrderCommandServiceDeps {
            order_item_command,
            order_item_query,
            product_client,
            command,
            query,
            kafka,
            metrics,
            registry,
        } = deps;

        registry.lock().await.register(
            "order_command_service_request_counter",
            "Total number of requests to the OrderCommandService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "order_command_service_request_duration",
            "Histogram of request durations for the OrderCommandService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            order_item_query,
            order_item_command,
            product_client,
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
        info!("🏗️ Creating new order for user_id={}", req.user_id);

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "create_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "create"),
                KeyValue::new("order.user_id", req.user_id.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        if req.items.is_empty() {
            return Err(ServiceError::Custom("Items cannot be empty".into()));
        }

        let mut total_price: i32 = 0;
        let mut prepared_items = Vec::new();

        for item in &req.items {
            let product_check = self.product_client.find_by_id(item.product_id).await;

            let product = match product_check {
                Ok(api_response) => {
                    let product_data = api_response.data;
                    info!("✅ Product found: id={}", product_data.id);
                    product_data
                }
                Err(e) => {
                    error!("❌ gRPC error fetching product: {:?}", e);
                    self.complete_tracing_error(&tracing_ctx, method, "Product query failed")
                        .await;
                    return Err(ServiceError::Internal("Product service unavailable".into()));
                }
            };

            if item.quantity > product.stock {
                return Err(ServiceError::Custom(format!(
                    "Insufficient stock for product {}: requested={}, available={}",
                    item.product_id, item.quantity, product.stock
                )));
            }

            let item_price: i32 = (product.price as i32) * item.quantity;
            total_price += item_price as i32;

            prepared_items.push((item.product_id, item.quantity, product.price, item_price));
        }

        let order_record = self
            .command
            .create_order(&CreateOrderRecordRequest {
                user_id: req.user_id,
                total_price,
            })
            .await
            .map_err(ServiceError::Repo)?;

        for (product_id, quantity, unit_price, _) in prepared_items.iter() {
            self.order_item_command
                .create_order_item(&CreateOrderItemRecordRequest {
                    order_id: order_record.order_id,
                    product_id: *product_id,
                    quantity: *quantity,
                    price: *unit_price as i32,
                })
                .await
                .map_err(ServiceError::Repo)?;
        }

        let mut response = OrderResponse::from(order_record);
        response.total_price = total_price;

        let event_items: Vec<OrderItemEvent> = req
            .items
            .iter()
            .cloned()
            .map(OrderItemEvent::from)
            .collect();

        let event = OrderEvent::Created {
            order_id: response.id,
            user_id: response.user_id,
            items: event_items,
        };

        let payload = serde_json::to_vec(&event)
            .map_err(|e| ServiceError::Custom(format!("Kafka error: {e}")))?;

        if let Err(e) = self
            .kafka
            .publish("order.created", &response.id.to_string(), &payload)
            .await
        {
            error!("❌ Failed to publish event: {e:?}");
        }

        Ok(ApiResponse {
            status: "success".into(),
            message: "Order created successfully".into(),
            data: response,
        })
    }

    async fn update_order(
        &self,
        req: &UpdateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, ServiceError> {
        let method = Method::Put;
        info!("✏️ Updating order ID={}", req.order_id);

        let tracing_ctx = self.start_tracing(
            "update_order",
            vec![
                KeyValue::new("component", "order"),
                KeyValue::new("operation", "update"),
                KeyValue::new("order_id", req.order_id.to_string()),
            ],
        );

        let _old_order = self
            .query
            .find_by_id(req.order_id)
            .await
            .map_err(ServiceError::Repo)?
            .ok_or(ServiceError::Custom("Order not found".into()))?;

        let old_items = self
            .order_item_query
            .find_order_item_by_order(req.order_id)
            .await
            .map_err(ServiceError::Repo)?;

        let mut old_map = std::collections::HashMap::new();
        for item in &old_items {
            old_map.insert(item.order_item_id, item.quantity);
        }

        let mut updates: Vec<OrderItemUpdateEvent> = vec![];

        for item_req in &req.items {
            let product_check = self.product_client.find_by_id(item_req.product_id).await;

            let product = match product_check {
                Ok(api_response) => {
                    let product_data = api_response.data;
                    info!("✅ Product found: id={}", product_data.id);
                    product_data
                }
                Err(e) => {
                    error!("❌ gRPC error fetching product: {:?}", e);
                    self.complete_tracing_error(&tracing_ctx, method, "Product query failed")
                        .await;
                    return Err(ServiceError::Internal("Product service unavailable".into()));
                }
            };

            if item_req.quantity > product.stock {
                return Err(ServiceError::Custom(format!(
                    "Not enough stock for product {}",
                    item_req.product_id
                )));
            }

            let old_quantity = old_map.get(&item_req.order_item_id).copied().unwrap_or(0);

            if old_quantity != item_req.quantity {
                updates.push(OrderItemUpdateEvent {
                    product_id: item_req.product_id,
                    old_quantity,
                    new_quantity: item_req.quantity,
                });
            }
        }

        let mut total_price: i32 = 0;

        for req_item in &req.items {
            let product_check = self.product_client.find_by_id(req_item.product_id).await;

            let product = match product_check {
                Ok(api_response) => {
                    let product_data = api_response.data;
                    info!("✅ Product found: id={}", product_data.id);
                    product_data
                }
                Err(e) => {
                    error!("❌ gRPC error fetching product: {:?}", e);
                    self.complete_tracing_error(&tracing_ctx, method, "Product query failed")
                        .await;
                    return Err(ServiceError::Internal("Product service unavailable".into()));
                }
            };

            total_price += req_item.quantity * product.price as i32;
        }

        let update_order_record = &UpdateOrderRecordRequest {
            order_id: req.order_id,
            user_id: req.user_id,
            total_price,
        };

        let updated_order = self
            .command
            .update_order(update_order_record)
            .await
            .map_err(ServiceError::Repo)?;

        for item_req in &req.items {
            let record = UpdateOrderItemRecordRequest {
                order_item_id: item_req.order_item_id,
                order_id: req.order_id,
                product_id: item_req.product_id,
                quantity: item_req.quantity,
                price: item_req.price,
            };

            self.order_item_command
                .update_order_item(&record)
                .await
                .map_err(ServiceError::Repo)?;
        }

        if !updates.is_empty() {
            let event = OrderEvent::Updated {
                order_id: req.order_id,
                updates,
            };

            let payload = serde_json::to_vec(&event)
                .map_err(|e| ServiceError::Custom(format!("Failed to serialize event: {e}")))?;

            self.kafka
                .publish("order.updated", &req.order_id.to_string(), &payload)
                .await
                .map_err(|e| ServiceError::Kafka(e.to_string()))?;

            info!(
                "📤 Published order.updated event for order_id={}",
                req.order_id
            );
        } else {
            info!("🔁 No quantity changed, skipping event publish.");
        }

        let mut response = OrderResponse::from(updated_order);
        response.total_price = total_price;

        Ok(ApiResponse {
            status: "success".into(),
            message: "Order updated successfully".into(),
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
    ) -> Result<ApiResponse<OrderResponseDeleteAt>, ServiceError> {
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

        let response = OrderResponseDeleteAt::from(order_model);

        info!("✅ Order restored: {} (ID: {order_id})", response.user_id,);

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

        let _order = match self.query.find_by_id(order_id).await {
            Ok(Some(order)) => order,
            Ok(None) => {
                self.complete_tracing_error(&tracing_ctx, method, "Order not found")
                    .await;
                return Err(ServiceError::Custom("Order not found".into()));
            }
            Err(e) => {
                self.complete_tracing_error(&tracing_ctx, method, "Failed to fetch order")
                    .await;
                return Err(ServiceError::Repo(e));
            }
        };

        let order_items = self
            .order_item_query
            .find_order_item_by_order(order_id)
            .await
            .map_err(ServiceError::Repo)?;

        if order_items.is_empty() {
            info!("⚠️ Order has no items, skipping item deletion");
        }

        let deleted_items: Vec<OrderItemEvent> = order_items
            .iter()
            .map(|item| OrderItemEvent {
                product_id: item.product_id,
                quantity: item.quantity,
            })
            .collect();

        self.command
            .delete_order(order_id)
            .await
            .map_err(ServiceError::Repo)?;

        for item in &order_items {
            if let Err(e) = self
                .order_item_command
                .delete_order_item_permanent(item.order_item_id)
                .await
            {
                error!(
                    "❌ Failed to delete order_item_id={}: {:?}",
                    item.order_item_id, e
                );
                return Err(ServiceError::Repo(e));
            }
        }

        let event = OrderEvent::Deleted {
            order_id,
            deleted_items,
        };

        let payload = serde_json::to_vec(&event)
            .map_err(|e| ServiceError::Custom(format!("Failed to serialize event: {e}")))?;

        self.kafka
            .publish("order.deleted", &order_id.to_string(), &payload)
            .await
            .map_err(|e| ServiceError::Kafka(e.to_string()))?;

        info!("📤 Published event: order.deleted | order_id={order_id}");

        self.complete_tracing_success(&tracing_ctx, method, "Order permanently deleted")
            .await;

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
