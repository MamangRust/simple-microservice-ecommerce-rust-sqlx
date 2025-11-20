use crate::domain::{
    requests::order::FindAllOrder,
    response::{
        api::{ApiResponse, ApiResponsePagination},
        order::{OrderResponse, OrderResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynOrderQueryService = Arc<dyn OrderQueryServiceTrait + Send + Sync>;

#[async_trait]
pub trait OrderQueryServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponse>>, ServiceError>;
    async fn find_active(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, ServiceError>;
    async fn find_trashed(
        &self,
        req: &FindAllOrder,
    ) -> Result<ApiResponsePagination<Vec<OrderResponseDeleteAt>>, ServiceError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<OrderResponse>, ServiceError>;
}
