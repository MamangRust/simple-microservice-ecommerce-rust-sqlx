use crate::domain::{
    requests::product::FindAllProducts,
    response::{
        api::{ApiResponse, ApiResponsePagination},
        product::{ProductResponse, ProductResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynProductQueryService = Arc<dyn ProductQueryServiceTrait + Send + Sync>;

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
