use crate::{
    domain::{
        requests::{CreateOrderRequest, FindAllOrders, UpdateOrderRequest},
        responses::{ApiResponse, ApiResponsePagination, OrderResponse, OrderResponseDeleteAt},
    },
    errors::AppErrorHttp,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynOrderGrpcClient = Arc<dyn OrderGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait OrderGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, AppErrorHttp>;
    async fn find_active(
        &self,
        req: &FindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, AppErrorHttp>;
    async fn find_trashed(
        &self,
        req: &FindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, AppErrorHttp>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<OrderResponse>, AppErrorHttp>;
    async fn create_order(
        &self,
        req: &CreateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, AppErrorHttp>;
    async fn update_order(
        &self,
        req: &UpdateOrderRequest,
    ) -> Result<ApiResponse<OrderResponse>, AppErrorHttp>;
    async fn trash_order(
        &self,
        id: i32,
    ) -> Result<ApiResponse<OrderResponseDeleteAt>, AppErrorHttp>;
    async fn restore_order(&self, id: i32) -> Result<ApiResponse<OrderResponse>, AppErrorHttp>;
    async fn delete_order(&self, id: i32) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn restore_all_order(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn delete_all_order(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
}
