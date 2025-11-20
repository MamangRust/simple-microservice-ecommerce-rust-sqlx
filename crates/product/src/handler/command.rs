use crate::{
    abstract_trait::product::service::DynProductCommandService,
    domain::requests::product::{
        CreateProductRequest as DomainCreateProductRequest,
        UpdateProductRequest as DomainUpdateProductRequest,
    },
};
use genproto::product::{
    ApiResponseProduct, ApiResponseProductAll, ApiResponseProductDelete,
    ApiResponseProductDeleteAt, CreateProductRequest, FindByIdProductRequest, UpdateProductRequest,
    product_command_service_server::ProductCommandService,
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct ProductCommandServiceImpl {
    pub command: DynProductCommandService,
}

impl ProductCommandServiceImpl {
    pub fn new(command: DynProductCommandService) -> Self {
        Self { command }
    }
}

#[tonic::async_trait]
impl ProductCommandService for ProductCommandServiceImpl {
    async fn create(
        &self,
        request: Request<CreateProductRequest>,
    ) -> Result<Response<ApiResponseProduct>, Status> {
        info!("Creating new Product");

        let req = request.into_inner();

        let domain_req = DomainCreateProductRequest {
            name: req.name,
            price: req.price,
            stock: req.stock,
        };

        let api_response = self
            .command
            .create_product(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProduct {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.clone().into()),
        };

        info!(
            "Product created successfully with ID: {}",
            api_response.data.id
        );
        Ok(Response::new(reply))
    }

    async fn update(
        &self,
        request: Request<UpdateProductRequest>,
    ) -> Result<Response<ApiResponseProduct>, Status> {
        info!("Updating Product");

        let req = request.into_inner();

        let domain_req = DomainUpdateProductRequest {
            id: req.id,
            name: req.name,
            price: req.price,
            stock: req.stock,
        };

        let api_response = self
            .command
            .update_product(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProduct {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Product updated successfully: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn trashed(
        &self,
        request: Request<FindByIdProductRequest>,
    ) -> Result<Response<ApiResponseProductDeleteAt>, Status> {
        info!("Soft deleting Product");

        let req = request.into_inner();

        let api_response = self
            .command
            .trash_product(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProductDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Product soft deleted: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn restore(
        &self,
        request: Request<FindByIdProductRequest>,
    ) -> Result<Response<ApiResponseProductDeleteAt>, Status> {
        info!("Restoring Product");

        let req = request.into_inner();

        let api_response = self
            .command
            .restore_product(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProductDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Product restored: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn delete_product_permanent(
        &self,
        request: Request<FindByIdProductRequest>,
    ) -> Result<Response<ApiResponseProductDelete>, Status> {
        info!("Permanently deleting Product");

        let req = request.into_inner();

        let api_response = self
            .command
            .delete_product(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProductDelete {
            status: "success".into(),
            message: api_response.message,
        };

        info!("Product permanently deleted: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn restore_all_product(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseProductAll>, Status> {
        info!("Restoring all soft-deleted Products");

        let _req = request.into_inner();

        let api_response = self
            .command
            .restore_all_product()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProductAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All Products restored successfully");
        Ok(Response::new(reply))
    }

    async fn delete_all_product(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseProductAll>, Status> {
        info!("Permanently deleting all soft-deleted Products");

        let _req = request.into_inner();

        let api_response = self
            .command
            .delete_all_product()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProductAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All Products permanently deleted");
        Ok(Response::new(reply))
    }
}
