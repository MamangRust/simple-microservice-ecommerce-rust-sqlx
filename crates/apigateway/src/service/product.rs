use crate::{
    abstract_trait::product::ProductGrpcClientTrait,
    domain::{
        requests::product::{
            CreateProductRequest as DomainCreateProductRequest,
            FindAllProducts as DomainFindAllProducts,
            UpdateProductRequest as DomainUpdateProductRequest,
        },
        response::{
            api::{ApiResponse, ApiResponsePagination},
            product::{ProductResponse, ProductResponseDeleteAt},
        },
    },
};
use async_trait::async_trait;
use genproto::product::{
    CreateProductRequest, FindAllProductRequest, FindByIdProductRequest, UpdateProductRequest,
    product_command_service_client::ProductCommandServiceClient,
    product_query_service_client::ProductQueryServiceClient,
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
pub struct ProductGrpcClientService {
    query_client: Arc<Mutex<ProductQueryServiceClient<Channel>>>,
    command_client: Arc<Mutex<ProductCommandServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl ProductGrpcClientService {
    pub async fn new(
        query_client: Arc<Mutex<ProductQueryServiceClient<Channel>>>,
        command_client: Arc<Mutex<ProductCommandServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Self {
        registry.lock().await.register(
            "product_service_client_request_counter",
            "Total number of requests to the ProductGrpcClientService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "product_service_client_request_duration",
            "Histogram of request durations for the ProductGrpcClientService",
            metrics.lock().await.request_duration.clone(),
        );
        Self {
            query_client,
            command_client,
            metrics,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("product-service-client")
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
impl ProductGrpcClientTrait for ProductGrpcClientService {
    async fn find_all(
        &self,
        req: &DomainFindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponse>>, HttpError> {
        info!(
            "Retrieving all product (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllProductRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.lock().await.find_all(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched all products",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC find_all failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let products: Vec<ProductResponse> = inner.data.into_iter().map(Into::into).collect();

        let product_len = products.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: products,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {product_len} Products");
        Ok(reply)
    }

    async fn find_active(
        &self,
        req: &DomainFindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving all product active (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindActiveProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllProductRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.lock().await.find_by_active(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched active Products",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC find_by_active failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let products: Vec<ProductResponseDeleteAt> =
            inner.data.into_iter().map(Into::into).collect();

        let products_len = products.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: products,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {products_len} active Products");
        Ok(reply)
    }

    async fn find_trashed(
        &self,
        req: &DomainFindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, HttpError> {
        info!(
            "Retrieving all product trashed (page: {}, size: {} search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindTrashedProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllProductRequest {
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
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "success fetch trashed")
                    .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC find_by_trashed failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let products: Vec<ProductResponseDeleteAt> =
            inner.data.into_iter().map(Into::into).collect();

        let products_len = products.len();

        let reply = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: products,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        info!("Successfully fetched {products_len} trashed Products");
        Ok(reply)
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, HttpError> {
        info!("Fetching Product by ID: {id}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindByIdProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_by_id"),
                KeyValue::new("product.id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.query_client.lock().await.find_by_id(request).await {
            Ok(response) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Successfully fetched Product by ID",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC find_by_id failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let product_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Product data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_product: ProductResponse = product_data.into();

        let product_name = domain_product.clone().name;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product.clone(),
        };

        info!("Successfully fetched Product: {product_name}");
        Ok(reply)
    }

    async fn create_product(
        &self,
        req: &DomainCreateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, HttpError> {
        info!("Creating new Product: {}", req.name);

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

        let mut request = Request::new(CreateProductRequest {
            name: req.name.clone(),
            price: req.price,
            stock: req.stock,
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.create(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully created Product")
                    .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC create_product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let product_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Product data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_product: ProductResponse = product_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {} created successfully", req.name);
        Ok(reply)
    }

    async fn update_product(
        &self,
        req: &DomainUpdateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, HttpError> {
        info!("Updating Product: {}", req.id);

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "update"),
                KeyValue::new("product.id", req.id.to_string()),
                KeyValue::new("product.name", req.name.clone()),
                KeyValue::new("product.price", req.price.to_string()),
                KeyValue::new("product.stock", req.stock.to_string()),
            ],
        );

        let mut request = Request::new(UpdateProductRequest {
            id: req.id,
            name: req.name.clone(),
            price: req.price,
            stock: req.stock,
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.update(request).await {
            Ok(response) => {
                self.complete_tracing_success(&tracing_ctx, method, "Successfully updated Product")
                    .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC update_product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let product_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Product data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_product: ProductResponse = product_data.into();

        let product_name = domain_product.clone().name;

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {product_name} updated successfully");
        Ok(reply)
    }

    async fn trash_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, HttpError> {
        info!("Soft deleting Product: {id}");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "TrashProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "trash"),
                KeyValue::new("product_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.trashed(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Product soft deleted")
                    .await;
                resp
            }
            Err(status) => {
                error!(
                    "gRPC trash_product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let product_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Product data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_product: ProductResponseDeleteAt = product_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {} soft deleted successfully", id);
        Ok(reply)
    }

    async fn restore_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, HttpError> {
        info!("Restoring Product: {}", id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "restore"),
                KeyValue::new("product_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.lock().await.restore(request).await {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Product restored")
                    .await;
                resp
            }
            Err(status) => {
                error!(
                    "gRPC restore_product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
                    .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let product_data = inner.data.ok_or_else(|| {
            let err: HttpError =
                AppErrorGrpc::Unhandled("Product data is missing in gRPC response".into()).into();
            err
        })?;

        let domain_product: ProductResponseDeleteAt = product_data.into();

        let reply = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {} restored successfully", id);
        Ok(reply)
    }

    async fn delete_product(&self, id: i32) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting Product: {}", id);

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("product_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindByIdProductRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .lock()
            .await
            .delete_product_permanent(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "Product permanently deleted")
                    .await;
                resp
            }
            Err(status) => {
                error!(
                    "gRPC delete_product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
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

        info!("Product {} permanently deleted", id);
        Ok(reply)
    }

    async fn restore_all_product(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Restoring all trashed Products");

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RestoreAllProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "restore"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .lock()
            .await
            .restore_all_product(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(&tracing_ctx, method, "All Products restored")
                    .await;
                resp
            }
            Err(status) => {
                error!(
                    "gRPC restore_all_Product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
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

        info!("All Products restored successfully");
        Ok(reply)
    }

    async fn delete_all_product(&self) -> Result<ApiResponse<()>, HttpError> {
        info!("Permanently deleting all trashed Products");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteAllProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "delete"),
            ],
        );

        let mut request = Request::new(());

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self
            .command_client
            .lock()
            .await
            .delete_all_product(request)
            .await
        {
            Ok(resp) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "All Products permanently deleted",
                )
                .await;
                resp
            }
            Err(status) => {
                error!(
                    "gRPC delete_all_product failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(&tracing_ctx, method, status.message())
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

        info!("All trashed Products permanently deleted");
        Ok(reply)
    }
}
