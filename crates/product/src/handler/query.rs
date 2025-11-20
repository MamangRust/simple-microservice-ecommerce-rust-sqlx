use crate::{
    abstract_trait::product::service::DynProductQueryService,
    domain::requests::product::FindAllProducts,
};
use genproto::product::{
    ApiResponsePaginationProduct, ApiResponsePaginationProductDeleteAt, ApiResponseProduct,
    FindAllProductRequest, FindByIdProductRequest,
    product_query_service_server::ProductQueryService,
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct ProductQueryServiceImpl {
    pub query: DynProductQueryService,
}

impl ProductQueryServiceImpl {
    pub fn new(query: DynProductQueryService) -> Self {
        Self { query }
    }
}

#[tonic::async_trait]
impl ProductQueryService for ProductQueryServiceImpl {
    async fn find_all(
        &self,
        request: Request<FindAllProductRequest>,
    ) -> Result<Response<ApiResponsePaginationProduct>, Status> {
        info!("Handling gRPC request: FindAll Products");

        let req = request.into_inner();

        let domain_req = FindAllProducts {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .query
            .find_all(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::product::ProductResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationProduct {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} Products", len);

        Ok(Response::new(reply))
    }

    async fn find_by_id(
        &self,
        request: Request<FindByIdProductRequest>,
    ) -> Result<Response<ApiResponseProduct>, Status> {
        info!("Handling gRPC request: Find Product by ID");

        let req = request.into_inner();

        let api_response = self
            .query
            .find_by_id(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseProduct {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Successfully fetched Product with ID: {}", req.id);
        Ok(Response::new(reply))
    }

    async fn find_by_active(
        &self,
        request: Request<FindAllProductRequest>,
    ) -> Result<Response<ApiResponsePaginationProductDeleteAt>, Status> {
        info!("Handling gRPC request: Find active Products");

        let req = request.into_inner();

        let domain_req = FindAllProducts {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .query
            .find_active(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::product::ProductResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationProductDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} active Products", len);
        Ok(Response::new(reply))
    }

    async fn find_by_trashed(
        &self,
        request: Request<FindAllProductRequest>,
    ) -> Result<Response<ApiResponsePaginationProductDeleteAt>, Status> {
        info!("Handling gRPC request: Find trashed Products");

        let req = request.into_inner();

        let domain_req = FindAllProducts {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .query
            .find_trashed(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::product::ProductResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationProductDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} trashed Products", len);
        Ok(Response::new(reply))
    }
}
