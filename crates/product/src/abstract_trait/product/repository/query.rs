use crate::{domain::requests::product::FindAllProducts, model::product::Product as ProductModel};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynProductQueryRepository = Arc<dyn ProductQueryRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait ProductQueryRepositoryTrait {
    async fn find_all(
        &self,
        req: &FindAllProducts,
    ) -> Result<(Vec<ProductModel>, i64), RepositoryError>;
    async fn find_active(
        &self,
        req: &FindAllProducts,
    ) -> Result<(Vec<ProductModel>, i64), RepositoryError>;
    async fn find_trashed(
        &self,
        req: &FindAllProducts,
    ) -> Result<(Vec<ProductModel>, i64), RepositoryError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<ProductModel>, RepositoryError>;
}
