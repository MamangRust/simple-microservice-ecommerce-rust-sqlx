use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::{
    requests::user_role::CreateUserRoleRequest,
    response::{api::ApiResponse, user_role::UserRoleResponse},
};
use shared::errors::ServiceError;

pub type DynUserRoleCommandService = Arc<dyn UserRoleCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserRoleCommandServiceTrait {
    async fn assign_role_to_user(
        &self,
        req: &CreateUserRoleRequest,
    ) -> Result<ApiResponse<UserRoleResponse>, ServiceError>;
    async fn update_role_to_user(
        &self,
        req: &CreateUserRoleRequest,
    ) -> Result<ApiResponse<UserRoleResponse>, ServiceError>;
}
