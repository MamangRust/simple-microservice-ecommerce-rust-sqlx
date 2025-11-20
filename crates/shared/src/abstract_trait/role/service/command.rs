use crate::{
    domain::{
        requests::{CreateRoleRequest, UpdateRoleRequest},
        responses::{ApiResponse, RoleResponse, RoleResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::Role as RoleModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynRoleCommandRepository = Arc<dyn RoleCommandRepositoryTrait + Send + Sync>;
pub type DynRoleCommandService = Arc<dyn RoleCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait RoleCommandRepositoryTrait {
    async fn create_role(&self, role: &CreateRoleRequest) -> Result<RoleModel, RepositoryError>;
    async fn update_role(&self, role: &UpdateRoleRequest) -> Result<RoleModel, RepositoryError>;
    async fn trash_role(&self, role_id: i32) -> Result<RoleModel, RepositoryError>;
    async fn restore_role(&self, role_id: i32) -> Result<RoleModel, RepositoryError>;
    async fn delete_role(&self, role_id: i32) -> Result<(), RepositoryError>;
    async fn restore_all_role(&self) -> Result<(), RepositoryError>;
    async fn delete_all_role(&self) -> Result<(), RepositoryError>;
}

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
    async fn restore_role(&self, role_id: i32) -> Result<ApiResponse<RoleResponse>, ServiceError>;
    async fn delete_ole(&self, role_id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_role(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_role(&self) -> Result<ApiResponse<()>, ServiceError>;
}
