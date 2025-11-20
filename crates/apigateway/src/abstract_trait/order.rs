use crate::domain::{
    requests::order::{CreateOrderRequest, FindAllOrder, UpdateOrderRequest},
    response::{
        api::{ApiResponse, ApiResponsePagination},
        order::{OrderResponse, OrderResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::HttpError;
use std::sync::Arc;

pub type DynOrderGrpcClient = Arc<dyn OrderGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait OrderGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, HttpError>;
    async fn find_active(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, HttpError>;
    async fn find_trashed(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, HttpError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<OrderResponse>, HttpError>;
    async fn create_order(
        &self,
        req: &CreateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, HttpError>;
    async fn update_order(
        &self,
        req: &UpdateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, HttpError>;
    async fn trash_order(&self, id: i32) -> Result<ApiResponse<OrderResponseDeleteAt>, HttpError>;
    async fn restore_order(&self, id: i32)
    -> Result<ApiResponse<OrderResponseDeleteAt>, HttpError>;
    async fn delete_order(&self, id: i32) -> Result<ApiResponse<()>, HttpError>;
    async fn restore_all_order(&self) -> Result<ApiResponse<()>, HttpError>;
    async fn delete_all_order(&self) -> Result<ApiResponse<()>, HttpError>;
}
