use crate::{
    domain::requests::order_item::{CreateOrderItemRecordRequest, UpdateOrderItemRecordRequest},
    model::order_item::OrderItem as OrderItemModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynOrderItemCommandRepository = Arc<dyn OrderItemCommandRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait OrderItemCommandRepositoryTrait {
    async fn create_order_item(
        &self,
        req: &CreateOrderItemRecordRequest,
    ) -> Result<OrderItemModel, RepositoryError>;

    async fn update_order_item(
        &self,
        req: &UpdateOrderItemRecordRequest,
    ) -> Result<OrderItemModel, RepositoryError>;

    async fn trashed_order_item(
        &self,
        order_item_id: i32,
    ) -> Result<OrderItemModel, RepositoryError>;

    async fn restore_order_item(
        &self,
        order_item_id: i32,
    ) -> Result<OrderItemModel, RepositoryError>;

    async fn delete_order_item_permanent(
        &self,
        order_item_id: i32,
    ) -> Result<bool, RepositoryError>;

    async fn restore_all_order_item(&self) -> Result<bool, RepositoryError>;

    async fn delete_all_order_item_permanent(&self) -> Result<bool, RepositoryError>;
}
