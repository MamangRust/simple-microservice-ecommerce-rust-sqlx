use crate::domain::{
    requests::role::{CreateRoleRequest, UpdateRoleRequest},
    response::{
        api::ApiResponse,
        role::{RoleResponse, RoleResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynRoleCommandService = Arc<dyn RoleCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait RoleCommandServiceTrait {
    async fn create_role(
        &self,
        role: &CreateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, ServiceError>;
    async fn update_role(
        &self,
        role: &UpdateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, ServiceError>;
    async fn trash_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, ServiceError>;
    async fn restore_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, ServiceError>;
    async fn delete_ole(&self, role_id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_role(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_role(&self) -> Result<ApiResponse<()>, ServiceError>;
}
