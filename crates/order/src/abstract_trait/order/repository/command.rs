use crate::{
    domain::requests::order::{CreateOrderRecordRequest, UpdateOrderRecordRequest},
    model::order::Order as OrderModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynOrderCommandRepository = Arc<dyn OrderCommandRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait OrderCommandRepositoryTrait {
    async fn create_order(
        &self,
        req: &CreateOrderRecordRequest,
    ) -> Result<OrderModel, RepositoryError>;
    async fn update_order(
        &self,
        req: &UpdateOrderRecordRequest,
    ) -> Result<OrderModel, RepositoryError>;
    async fn trash_order(&self, id: i32) -> Result<OrderModel, RepositoryError>;
    async fn restore_order(&self, id: i32) -> Result<OrderModel, RepositoryError>;
    async fn delete_order(&self, id: i32) -> Result<(), RepositoryError>;
    async fn restore_all_orders(&self) -> Result<(), RepositoryError>;
    async fn delete_all_orders(&self) -> Result<(), RepositoryError>;
}
