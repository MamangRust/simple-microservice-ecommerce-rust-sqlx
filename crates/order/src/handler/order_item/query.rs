use crate::{
    abstract_trait::order_item::service::DynOrderItemQueryService,
    domain::requests::order_item::FindAllOrderItems as DomainFindAllOrderItem,
};
use genproto::order_item::{
    ApiResponsePaginationOrderItem, ApiResponsePaginationOrderItemDeleteAt, ApiResponsesOrderItem,
    FindAllOrderItemRequest, FindByIdOrderItemRequest, OrderItemResponse,
    OrderItemResponseDeleteAt, order_item_service_server::OrderItemService,
};
use shared::errors::AppErrorGrpc;
use std::fmt;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct OrderItemGrpcServiceImpl {
    pub order_item_query_service: DynOrderItemQueryService,
}

impl fmt::Debug for OrderItemGrpcServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderItemGrpcServiceImpl")
            .field("order_item_query_service", &"DynOrderItemQueryService")
            .finish()
    }
}

impl OrderItemGrpcServiceImpl {
    pub fn new(order_item_query_service: DynOrderItemQueryService) -> Self {
        Self {
            order_item_query_service,
        }
    }
}

#[tonic::async_trait]
impl OrderItemService for OrderItemGrpcServiceImpl {
    async fn find_all(
        &self,
        request: Request<FindAllOrderItemRequest>,
    ) -> Result<Response<ApiResponsePaginationOrderItem>, Status> {
        info!("Handling gRPC request: FindAll order items");

        let req = request.into_inner();

        let domain_req = DomainFindAllOrderItem {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .order_item_query_service
            .find_all_order_items(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<OrderItemResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationOrderItem {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} order items", len);

        Ok(Response::new(reply))
    }

    async fn find_by_active(
        &self,
        request: Request<FindAllOrderItemRequest>,
    ) -> Result<Response<ApiResponsePaginationOrderItemDeleteAt>, Status> {
        info!("Handling gRPC request: Find active order items");

        let req = request.into_inner();

        let domain_req = DomainFindAllOrderItem {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .order_item_query_service
            .find_by_active(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<OrderItemResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationOrderItemDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} active order items", len);
        Ok(Response::new(reply))
    }

    async fn find_by_trashed(
        &self,
        request: Request<FindAllOrderItemRequest>,
    ) -> Result<Response<ApiResponsePaginationOrderItemDeleteAt>, Status> {
        info!("Handling gRPC request: Find trashed order items");

        let req = request.into_inner();

        let domain_req = DomainFindAllOrderItem {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .order_item_query_service
            .find_by_trashed(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<OrderItemResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationOrderItemDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} trashed order items", len);
        Ok(Response::new(reply))
    }

    async fn find_order_item_by_order(
        &self,
        request: Request<FindByIdOrderItemRequest>,
    ) -> Result<Response<ApiResponsesOrderItem>, Status> {
        info!("Handling gRPC request: Find order items by Order ID");

        let req = request.into_inner();

        let api_response = self
            .order_item_query_service
            .find_order_item_by_order(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<OrderItemResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsesOrderItem {
            status: "success".into(),
            message: format!("Found {} items for order {}", len, req.id),
            data,
        };

        info!(
            "Successfully fetched {} items for order ID: {}",
            len, req.id
        );
        Ok(Response::new(reply))
    }
}
