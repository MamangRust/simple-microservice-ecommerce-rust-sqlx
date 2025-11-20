use crate::{
    abstract_trait::order::service::DynOrderQueryService,
    domain::requests::order::FindAllOrder as DomainFindAllOrder,
};
use genproto::order::{
    ApiResponseOrder, ApiResponsePaginationOrder, ApiResponsePaginationOrderDeleteAt,
    FindAllOrderRequest, FindByIdOrderRequest, order_query_service_server::OrderQueryService,
};
use shared::errors::AppErrorGrpc;
use std::fmt;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct OrderQueryGrpcServiceImpl {
    pub order_query_service: DynOrderQueryService,
}

impl fmt::Debug for OrderQueryGrpcServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderQueryGrpcServiceImpl")
            .field("order_query_service", &"DynOrderQueryService")
            .finish()
    }
}

impl OrderQueryGrpcServiceImpl {
    pub fn new(order: DynOrderQueryService) -> Self {
        Self {
            order_query_service: order,
        }
    }
}

#[tonic::async_trait]
impl OrderQueryService for OrderQueryGrpcServiceImpl {
    async fn find_all(
        &self,
        request: Request<FindAllOrderRequest>,
    ) -> Result<Response<ApiResponsePaginationOrder>, Status> {
        info!("Handling gRPC request: FindAll orders");

        let req = request.into_inner();

        let domain_req = DomainFindAllOrder {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .order_query_service
            .find_all(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::order::OrderResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationOrder {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} orders", len);

        Ok(Response::new(reply))
    }

    async fn find_by_id(
        &self,
        request: Request<FindByIdOrderRequest>,
    ) -> Result<Response<ApiResponseOrder>, Status> {
        info!("Handling gRPC request: Find order by ID");

        let req = request.into_inner();

        let api_response = self
            .order_query_service
            .find_by_id(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrder {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Successfully fetched order with ID: {}", req.id);
        Ok(Response::new(reply))
    }

    async fn find_by_active(
        &self,
        request: Request<FindAllOrderRequest>,
    ) -> Result<Response<ApiResponsePaginationOrderDeleteAt>, Status> {
        info!("Handling gRPC request: Find active orders");

        let req = request.into_inner();

        let domain_req = DomainFindAllOrder {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .order_query_service
            .find_active(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::order::OrderResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationOrderDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} active orders", len);
        Ok(Response::new(reply))
    }

    async fn find_by_trashed(
        &self,
        request: Request<FindAllOrderRequest>,
    ) -> Result<Response<ApiResponsePaginationOrderDeleteAt>, Status> {
        info!("Handling gRPC request: Find trashed orders");

        let req = request.into_inner();

        let domain_req = DomainFindAllOrder {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .order_query_service
            .find_trashed(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::order::OrderResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationOrderDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} trashed orders", len);
        Ok(Response::new(reply))
    }
}
