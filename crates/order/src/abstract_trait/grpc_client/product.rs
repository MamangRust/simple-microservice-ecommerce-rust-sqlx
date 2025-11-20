use crate::domain::response::{api::ApiResponse, product::ProductResponse};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::AppErrorGrpc;
use std::sync::Arc;

pub type DynProductGrpcClient = Arc<dyn ProductGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait ProductGrpcClientTrait {
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, AppErrorGrpc>;
}
