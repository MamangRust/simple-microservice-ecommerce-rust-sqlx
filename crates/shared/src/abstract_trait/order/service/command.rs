use crate::{
    domain::{
        requests::{CreateOrderRequest, UpdateOrderRequest},
        responses::{ApiResponse, OrderResponse, OrderResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::Order as OrderModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynOrderCommandRepository = Arc<dyn OrderCommandRepositoryTrait + Send + Sync>;
pub type DynOrderCommandService = Arc<dyn OrderCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait OrderCommandRepositoryTrait {
    async fn create_order(
        &self,
        req: &CreateOrderRequest,
        total: i64,
    ) -> Result<OrderModel, RepositoryError>;
    async fn update_order(
        &self,
        req: &UpdateOrderRequest,
        total: i64,
    ) -> Result<OrderModel, RepositoryError>;
    async fn trash_order(&self, id: i32) -> Result<OrderModel, RepositoryError>;
    async fn restore_order(&self, id: i32) -> Result<OrderModel, RepositoryError>;
    async fn delete_order(&self, id: i32) -> Result<(), RepositoryError>;
    async fn restore_all_orders(&self) -> Result<(), RepositoryError>;
    async fn delete_all_orders(&self) -> Result<(), RepositoryError>;
}

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
    async fn restore_order(&self, id: i32) -> Result<ApiResponse<OrderResponse>, ServiceError>;
    async fn delete_order(&self, id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_order(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_order(&self) -> Result<ApiResponse<()>, ServiceError>;
}
