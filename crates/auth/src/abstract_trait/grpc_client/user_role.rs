use crate::domain::{
    requests::user_role::CreateUserRoleRequest,
    response::{api::ApiResponse, user_role::UserRoleResponse},
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::AppErrorGrpc;
use std::sync::Arc;

pub type DynUserRoleGrpcClient = Arc<dyn UserRoleGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait UserRoleGrpcClientTrait {
    async fn assign_role(
        &self,
        req: CreateUserRoleRequest,
    ) -> Result<ApiResponse<UserRoleResponse>, AppErrorGrpc>;
}
