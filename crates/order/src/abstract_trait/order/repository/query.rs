use crate::{domain::requests::order::FindAllOrder, model::order::Order as OrderModel};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynOrderQueryRepository = Arc<dyn OrderQueryRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait OrderQueryRepositoryTrait {
    async fn find_all(&self, req: &FindAllOrder)
    -> Result<(Vec<OrderModel>, i64), RepositoryError>;
    async fn find_active(
        &self,
        req: &FindAllOrder,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError>;
    async fn find_trashed(
        &self,
        req: &FindAllOrder,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<OrderModel>, RepositoryError>;
}
