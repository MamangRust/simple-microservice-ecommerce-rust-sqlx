use crate::{
    abstract_trait::{DynProductCommandRepository, ProductCommandServiceTrait},
    domain::{
        requests::{CreateProductRequest, UpdateProductRequest},
        responses::{ApiResponse, ProductResponse, ProductResponseDeleteAt},
    },
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
use genproto::product::FindByIdProductRequest;
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

pub struct ProductCommandService {
    pub command: DynProductCommandRepository,
    pub metrics: Arc<Mutex<Metrics>>,
}

impl ProductCommandService {
    pub async fn new(
        command: DynProductCommandRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        registry.lock().await.register(
            "product_command_service_request_counter",
            "Total number of requests to the ProductCommandService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "product_command_service_request_duration",
            "Histogram of request durations for the ProductCommandService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { command, metrics }
    }
    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("product-command-service")
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
impl ProductCommandServiceTrait for ProductCommandService {
    async fn increasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError> {
        info!("📈 Increasing stock for product ID={product_id}: +{qty}");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "increasing_stock",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "increase"),
                KeyValue::new("product.id", product_id.to_string()),
                KeyValue::new("product.quantity", qty.to_string()),
            ],
        );

        if qty <= 0 {
            error!("❌ Quantity to increase must be positive");
            self.complete_tracing_error(
                &tracing_ctx,
                method.clone(),
                "Quantity to increase must be positive",
            )
            .await;
            return Err(ServiceError::Custom(
                "Quantity to increase must be positive".to_string(),
            ));
        }

        let product_model = match self.command.increasing_stock(product_id, qty).await {
            Ok(product) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Stock increased successfully",
                )
                .await;
                product
            }
            Err(err) => {
                error!("❌ Failed to increase stock: {err:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to increase stock",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = ProductResponse::from(product_model);

        info!(
            "✅ Stock increased: {} (ID: {}), new stock: {}",
            response.name, response.id, response.stock
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product stock increased successfully".to_string(),
            data: response,
        })
    }

    async fn decreasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError> {
        info!("📉 Decreasing stock for product ID={product_id}: -{qty}",);

        let tracing_ctx = self.start_tracing(
            "decreasing_stock",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "decrease"),
                KeyValue::new("product.id", product_id.to_string()),
                KeyValue::new("product.quantity", qty.to_string()),
            ],
        );

        if qty <= 0 {
            error!("❌ Quantity to decrease must be positive");
            self.complete_tracing_error(
                &tracing_ctx,
                Method::Post,
                "Quantity to decrease must be positive",
            )
            .await;
            return Err(ServiceError::Custom(
                "Quantity to decrease must be positive".to_string(),
            ));
        }

        let product_model = match self.command.decreasing_stock(product_id, qty).await {
            Ok(product) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    Method::Post,
                    "Stock decreased successfully",
                )
                .await;
                product
            }
            Err(err) => {
                error!("❌ Failed to decrease stock: {err:?}");
                self.complete_tracing_error(&tracing_ctx, Method::Post, "Failed to decrease stock")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = ProductResponse::from(product_model);

        info!(
            "✅ Stock decreased: {} (ID: {}), new stock: {}",
            response.name, response.id, response.stock
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product stock decreased successfully".to_string(),
            data: response,
        })
    }
    async fn create_product(
        &self,
        req: &CreateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError> {
        info!("🏗️ Creating new Product: {}", req.name);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "create"),
                KeyValue::new("product.name", req.name.clone()),
                KeyValue::new("product.price", req.price.to_string()),
                KeyValue::new("product.stock", req.stock.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let product_model = match self.command.create_product(req).await {
            Ok(product) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Product created successfully",
                )
                .await;
                product
            }
            Err(err) => {
                error!("❌ Failed to create product: {err:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to create product",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = ProductResponse::from(product_model);

        info!(
            "✅ Product created successfully: {} (ID: {})",
            response.name, response.id,
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product created successfully".to_string(),
            data: response,
        })
    }

    async fn update_product(
        &self,
        req: &UpdateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError> {
        info!("✏️ Updating Product with ID: {}", req.id);

        let tracing_ctx = self.start_tracing(
            "update_product",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "update"),
                KeyValue::new("product.id", req.id.to_string()),
                KeyValue::new("product.name", req.name.clone()),
                KeyValue::new("product.price", req.price.to_string()),
                KeyValue::new("product.stock", req.stock.to_string()),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let product_model = match self.command.update_product(req).await {
            Ok(product) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    Method::Put,
                    "Product updated successfully",
                )
                .await;
                product
            }
            Err(err) => {
                error!("❌ Failed to update product: {err:?}");
                self.complete_tracing_error(&tracing_ctx, Method::Put, "Failed to update product")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = ProductResponse::from(product_model);

        info!(
            "✅ Product updated successfully: {} (ID: {})",
            response.name, response.id,
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product updated successfully".to_string(),
            data: response,
        })
    }

    async fn trash_product(
        &self,
        product_id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, ServiceError> {
        info!("🗑️ Soft deleting Product with ID: {}", product_id);

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "trash_product",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("product.id", product_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id: product_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let product_model = match self.command.trash_product(product_id).await {
            Ok(product) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Product moved to trash successfully",
                )
                .await;
                product
            }
            Err(err) => {
                error!(
                    "❌ Failed to soft delete Product ID {}: {err:?}",
                    product_id
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to trash product",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = ProductResponseDeleteAt::from(product_model);

        info!("✅ Product moved to trash: ID {}", response.id);

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product moved to trash successfully".to_string(),
            data: response,
        })
    }

    async fn restore_product(
        &self,
        product_id: i32,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError> {
        info!("🔄 Restoring Product with ID: {product_id}");
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "restore_product",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("product.id", product_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id: product_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let product_model = match self.command.restore_product(product_id).await {
            Ok(product) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Product restored successfully",
                )
                .await;
                product
            }
            Err(err) => {
                error!("❌ Failed to restore Product ID {}: {err:?}", product_id);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to restore product",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let response = ProductResponse::from(product_model);

        info!(
            "✅ Product restored: {} (ID: {})",
            response.name, response.id,
        );

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product restored successfully".to_string(),
            data: response,
        })
    }

    async fn delete_product(&self, product_id: i32) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting Product with ID: {product_id}");

        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "delete_product",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("product.id", product_id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id: product_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.command.delete_product(product_id).await {
            Ok(()) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Product deleted permanently",
                )
                .await;
            }
            Err(err) => {
                error!(
                    "❌ Failed to permanently delete Product ID {}: {err:?}",
                    product_id
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to delete product",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        }

        info!("✅ Product permanently deleted: {product_id}");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Product deleted permanently".to_string(),
            data: (),
        })
    }

    async fn restore_all_product(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("🔄 Restoring all soft-deleted Products");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "restore_all_product",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "restore_all"),
            ],
        );

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.command.restore_all_products().await {
            Ok(()) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "All products restored successfully",
                )
                .await;
            }
            Err(err) => {
                error!("❌ Failed to restore all Products: {err:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to restore all products",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        }

        info!("✅ All Products restored successfully");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "All Products restored successfully".to_string(),
            data: (),
        })
    }

    async fn delete_all_product(&self) -> Result<ApiResponse<()>, ServiceError> {
        info!("💀 Permanently deleting all Products");

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "delete_all_product",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "delete_all"),
            ],
        );

        let mut request = Request::new(());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.command.delete_all_products().await {
            Ok(()) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "All products deleted permanently",
                )
                .await;
            }
            Err(err) => {
                error!("❌ Failed to delete all Products: {err:?}");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to delete all products",
                )
                .await;
                return Err(ServiceError::Repo(err));
            }
        }

        info!("✅ All Products deleted permanently");

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "All Products deleted permanently".to_string(),
            data: (),
        })
    }
}
