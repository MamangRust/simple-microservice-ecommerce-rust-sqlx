use crate::domain::{
    requests::order_item::FindAllOrderItems,
    response::{
        api::{ApiResponse, ApiResponsePagination},
        order_item::{OrderItemResponse, OrderItemResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::HttpError;
use std::sync::Arc;

pub type DynOrderItemGrpcClient = Arc<dyn OrderItemGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait OrderItemGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<OrderItemResponse>, HttpError>;
    async fn find_by_active(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<OrderItemResponseDeleteAt>, HttpError>;

    async fn find_by_trashed(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<OrderItemResponseDeleteAt>, HttpError>;

    async fn find_order_item_by_order(
        &self,
        order_id: i32,
    ) -> Result<ApiResponse<Vec<OrderItemResponse>>, HttpError>;
}
