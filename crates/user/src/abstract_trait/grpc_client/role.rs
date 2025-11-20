use crate::domain::response::{api::ApiResponse, role::RoleResponse};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::AppErrorGrpc;
use std::sync::Arc;

pub type DynRoleGrpcClient = Arc<dyn RoleGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait RoleGrpcClientTrait {
    async fn find_by_name(&self, name: &str) -> Result<ApiResponse<RoleResponse>, AppErrorGrpc>;
}
