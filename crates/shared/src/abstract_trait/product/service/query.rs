use crate::{
    domain::{
        requests::FindAllProducts,
        responses::{ApiResponse, ApiResponsePagination, ProductResponse, ProductResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::Product as ProductModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynProductQueryRepository = Arc<dyn ProductQueryRepositoryTrait + Send + Sync>;
pub type DynProductQueryService = Arc<dyn ProductQueryServiceTrait + Send + Sync>;

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

#[async_trait]
pub trait ProductQueryServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponse>>, ServiceError>;
    async fn find_active(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, ServiceError>;
    async fn find_trashed(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, ServiceError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, ServiceError>;
}
