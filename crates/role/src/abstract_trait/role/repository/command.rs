use crate::{
    domain::requests::role::{CreateRoleRequest, UpdateRoleRequest},
    model::role::Role as RoleModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynRoleCommandRepository = Arc<dyn RoleCommandRepositoryTrait + Send + Sync>;

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
