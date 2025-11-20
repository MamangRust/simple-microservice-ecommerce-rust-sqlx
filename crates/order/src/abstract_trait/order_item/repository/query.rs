use crate::{
    domain::requests::order_item::FindAllOrderItems, model::order_item::OrderItem as OrderItemModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynOrderItemQueryRepository = Arc<dyn OrderItemQueryRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait OrderItemQueryRepositoryTrait {
    async fn find_order_item_by_order(
        &self,
        order_id: i32,
    ) -> Result<Vec<OrderItemModel>, RepositoryError>;

    async fn calculate_total_price(&self, order_id: i32) -> Result<i32, RepositoryError>;

    async fn find_all_order_items(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<(Vec<OrderItemModel>, i64), RepositoryError>;

    async fn find_by_active(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<(Vec<OrderItemModel>, i64), RepositoryError>;

    async fn find_by_trashed(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<(Vec<OrderItemModel>, i64), RepositoryError>;
}
