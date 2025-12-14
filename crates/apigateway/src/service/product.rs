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
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
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
pub struct ProductGrpcClientService {
    query_client: ProductQueryServiceClient<Channel>,
    command_client: ProductCommandServiceClient<Channel>,
    metrics: Metrics,
    cache_store: Arc<CacheStore>,
}

impl ProductGrpcClientService {
    pub fn new(
        query_client: ProductQueryServiceClient<Channel>,
        command_client: ProductCommandServiceClient<Channel>,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let metrics = Metrics::new(global::meter("product-service-client"));

        Ok(Self {
            query_client,
            command_client,
            metrics,
            cache_store,
        })
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

        self.metrics.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl ProductGrpcClientTrait for ProductGrpcClientService {
    async fn find_all(
        &self,
        req: &DomainFindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponse>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all product (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindAllProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_all"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllProductRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "product:find_all:page:{page}:size:{page_size}:search:{}",
            req.search.clone(),
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<ProductResponse>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} roles in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_all(request).await {
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

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: products,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched {product_len} Products");
        Ok(api_response)
    }

    async fn find_active(
        &self,
        req: &DomainFindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all product active (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindActiveProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_active"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllProductRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "product:find_active:page:{page}:size:{page_size}:search:{}",
            req.search.clone()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<ProductResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} active roles in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_active(request).await {
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

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: products,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched {products_len} active Products");
        Ok(api_response)
    }

    async fn find_trashed(
        &self,
        req: &DomainFindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, HttpError> {
        let page = req.page;
        let page_size = req.page_size;

        info!(
            "Retrieving all product trashed (page: {page}, size: {page_size} search: {})",
            req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindTrashedProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "find_trashed"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllProductRequest {
            page,
            page_size,
            search: req.search.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "product:find_trashed:page:{page}:size:{page_size}:search:{:?}",
            req.search.clone()
        );

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<ProductResponseDeleteAt>>>(&cache_key)
            .await
        {
            let log_msg = format!("✅ Found {} trashed roles in cache", cache.data.len());
            info!("{log_msg}");
            self.complete_tracing_success(&tracing_ctx, method, &log_msg)
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_trashed(request).await {
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

        let api_response = ApiResponsePagination {
            status: inner.status,
            message: inner.message,
            data: products,
            pagination: inner.pagination.unwrap_or_default().into(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched {products_len} trashed Products");
        Ok(api_response)
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

        let cache_key = format!("product:find_by_id:id:{id}");

        if let Some(cache) = self
            .cache_store
            .get_from_cache::<ApiResponse<ProductResponse>>(&cache_key)
            .await
        {
            info!("✅ Found role in cache");
            self.complete_tracing_success(&tracing_ctx, method, "Role retrieved from cache")
                .await;
            return Ok(cache);
        }

        let response = match self.query_client.clone().find_by_id(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product.clone(),
        };

        self.cache_store
            .set_to_cache(&cache_key, &api_response, Duration::minutes(30))
            .await;

        info!("Successfully fetched Product: {product_name}");
        Ok(api_response)
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

        let response = match self.command_client.clone().create(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {} created successfully", req.name);
        Ok(api_response)
    }

    async fn update_product(
        &self,
        req: &DomainUpdateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, HttpError> {
        info!("Updating Product: {:?}", req.id);

        let product_id = req
            .id
            .ok_or_else(|| HttpError::BadRequest("product id is required".into()))?;

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateProduct",
            vec![
                KeyValue::new("component", "product"),
                KeyValue::new("operation", "update"),
                KeyValue::new("product.id", product_id.to_string()),
                KeyValue::new("product.name", req.name.clone()),
                KeyValue::new("product.price", req.price.to_string()),
                KeyValue::new("product.stock", req.stock.to_string()),
            ],
        );

        let mut request = Request::new(UpdateProductRequest {
            id: product_id,
            name: req.name.clone(),
            price: req.price,
            stock: req.stock,
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.command_client.clone().update(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {product_name} updated successfully");
        Ok(api_response)
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

        let response = match self.command_client.clone().trashed(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {} soft deleted successfully", id);
        Ok(api_response)
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

        let response = match self.command_client.clone().restore(request).await {
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_product,
        };

        info!("Product {} restored successfully", id);
        Ok(api_response)
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
            .clone()
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("Product {} permanently deleted", id);
        Ok(api_response)
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
            .clone()
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All Products restored successfully");
        Ok(api_response)
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
            .clone()
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

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: (),
        };

        info!("All trashed Products permanently deleted");
        Ok(api_response)
    }
}
