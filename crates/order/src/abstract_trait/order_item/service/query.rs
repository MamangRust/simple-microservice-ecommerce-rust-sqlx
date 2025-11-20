use std::sync::Arc;

use crate::domain::{
    requests::order_item::FindAllOrderItems,
    response::{
        api::{ApiResponse, ApiResponsePagination},
        order_item::{OrderItemResponse, OrderItemResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;

pub type DynOrderItemQueryService = Arc<dyn OrderItemQueryServiceTrait + Send + Sync>;

#[async_trait]
pub trait OrderItemQueryServiceTrait {
    async fn find_all_order_items(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponse>>, ServiceError>;

    async fn find_by_active(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>, ServiceError>;

    async fn find_by_trashed(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>, ServiceError>;

    async fn find_order_item_by_order(
        &self,
        order_id: i32,
    ) -> Result<ApiResponse<Vec<OrderItemResponse>>, ServiceError>;
}
