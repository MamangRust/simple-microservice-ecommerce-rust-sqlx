use crate::domain::{
    requests::order::{CreateOrderRequest, UpdateOrderRequest},
    response::{
        api::ApiResponse,
        order::{OrderResponse, OrderResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynOrderCommandService = Arc<dyn OrderCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait OrderCommandServiceTrait {
    async fn create_order(
        &self,
        req: &CreateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, ServiceError>;
    async fn update_order(
        &self,
        req: &UpdateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, ServiceError>;
    async fn trash_order(
        &self,
        id: i32,
    ) -> Result<ApiResponse<OrderResponseDeleteAt>, ServiceError>;
    async fn restore_order(
        &self,
        id: i32,
    ) -> Result<ApiResponse<OrderResponseDeleteAt>, ServiceError>;
    async fn delete_order(&self, id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_order(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_order(&self) -> Result<ApiResponse<()>, ServiceError>;
}
