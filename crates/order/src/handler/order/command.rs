use std::fmt;

use crate::{
    abstract_trait::order::service::DynOrderCommandService,
    domain::requests::order::{
        CreateOrderItemRequest as DomainCreateOrderItemRequest,
        CreateOrderRequest as DomainCreateOrderRequest,
        UpdateOrderItemRequest as DomainUpdateOrderItemRequest,
        UpdateOrderRequest as DomainUpdateOrderRequest,
    },
};
use genproto::order::{
    ApiResponseOrder, ApiResponseOrderAll, ApiResponseOrderDelete, ApiResponseOrderDeleteAt,
    CreateOrderRequest, FindByIdOrderRequest, UpdateOrderRequest,
    order_command_service_server::OrderCommandService,
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct OrderCommandGrpcServiceImpl {
    pub order_command_service: DynOrderCommandService,
}

impl fmt::Debug for OrderCommandGrpcServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderCommandGrpcServiceImpl")
            .field("order_command_service", &"DynOrderCommandService")
            .finish()
    }
}

impl OrderCommandGrpcServiceImpl {
    pub fn new(order: DynOrderCommandService) -> Self {
        Self {
            order_command_service: order,
        }
    }
}

#[tonic::async_trait]
impl OrderCommandService for OrderCommandGrpcServiceImpl {
    async fn create(
        &self,
        request: Request<CreateOrderRequest>,
    ) -> Result<Response<ApiResponseOrder>, Status> {
        info!("Creating new order");

        let req = request.into_inner();

        let items = req
            .items
            .into_iter()
            .map(|i| DomainCreateOrderItemRequest {
                product_id: i.product_id,
                quantity: i.quantity,
                price: i.price,
            })
            .collect::<Vec<_>>();

        let domain_req = DomainCreateOrderRequest {
            user_id: req.user_id,
            items,
        };

        let api_response = self
            .order_command_service
            .create_order(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrder {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.clone().into()),
        };

        info!(
            "Order created successfully with ID: {}",
            api_response.data.id
        );
        Ok(Response::new(reply))
    }

    async fn update(
        &self,
        request: Request<UpdateOrderRequest>,
    ) -> Result<Response<ApiResponseOrder>, Status> {
        info!("Updating order");

        let req = request.into_inner();

        let items = req
            .items
            .into_iter()
            .map(|i| DomainUpdateOrderItemRequest {
                order_item_id: i.order_item_id,
                product_id: i.product_id,
                quantity: i.quantity,
                price: i.price,
            })
            .collect::<Vec<_>>();

        let domain_req = DomainUpdateOrderRequest {
            order_id: req.order_id,
            user_id: req.user_id,
            items,
        };

        let api_response = self
            .order_command_service
            .update_order(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrder {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Order updated successfully: ID={}", req.order_id);
        Ok(Response::new(reply))
    }

    async fn trashed(
        &self,
        request: Request<FindByIdOrderRequest>,
    ) -> Result<Response<ApiResponseOrderDeleteAt>, Status> {
        info!("Soft deleting order");

        let req = request.into_inner();

        let api_response = self
            .order_command_service
            .trash_order(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrderDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Order soft deleted: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn restore(
        &self,
        request: Request<FindByIdOrderRequest>,
    ) -> Result<Response<ApiResponseOrderDeleteAt>, Status> {
        info!("Restoring order");

        let req = request.into_inner();

        let api_response = self
            .order_command_service
            .restore_order(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrderDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Order restored: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn delete_order_permanent(
        &self,
        request: Request<FindByIdOrderRequest>,
    ) -> Result<Response<ApiResponseOrderDelete>, Status> {
        info!("Permanently deleting order");

        let req = request.into_inner();

        let api_response = self
            .order_command_service
            .delete_order(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrderDelete {
            status: "success".into(),
            message: api_response.message,
        };

        info!("Order permanently deleted: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn restore_all_order(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseOrderAll>, Status> {
        info!("Restoring all soft-deleted orders");

        let _req = request.into_inner();

        let api_response = self
            .order_command_service
            .restore_all_order()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrderAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All orders restored successfully");
        Ok(Response::new(reply))
    }

    async fn delete_all_order(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseOrderAll>, Status> {
        info!("Permanently deleting all soft-deleted orders");

        let _req = request.into_inner();

        let api_response = self
            .order_command_service
            .delete_all_order()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseOrderAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All orders permanently deleted");
        Ok(Response::new(reply))
    }
}
