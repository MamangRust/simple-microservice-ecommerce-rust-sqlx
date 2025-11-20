use crate::{
    domain::{
        requests::FindAllOrders,
        responses::{ApiResponse, ApiResponsePagination, OrderResponse, OrderResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::Order as OrderModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynOrderQueryRepository = Arc<dyn OrderQueryRepositoryTrait + Send + Sync>;
pub type DynOrderQueryService = Arc<dyn OrderQueryServiceTrait + Send + Sync>;

#[async_trait]
pub trait OrderQueryRepositoryTrait {
    async fn find_all(
        &self,
        req: &FindAllOrders,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError>;
    async fn find_active(
        &self,
        req: &FindAllOrders,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError>;
    async fn find_trashed(
        &self,
        req: &FindAllOrders,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<OrderModel>, RepositoryError>;
}

#[async_trait]
pub trait OrderQueryServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, ServiceError>;
    async fn find_active(
        &self,
        req: &FindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, ServiceError>;
    async fn find_trashed(
        &self,
        req: &FindAllOrders,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, ServiceError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<OrderResponse>, ServiceError>;
}
