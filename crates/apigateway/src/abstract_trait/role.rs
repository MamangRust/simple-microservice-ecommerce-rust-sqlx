use crate::domain::{
    requests::role::{CreateRoleRequest, FindAllRole, UpdateRoleRequest},
    response::{
        api::{ApiResponse, ApiResponsePagination},
        role::{RoleResponse, RoleResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::HttpError;
use std::sync::Arc;

pub type DynRoleGrpcClient = Arc<dyn RoleGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait RoleGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponse>>, HttpError>;
    async fn find_active(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, HttpError>;
    async fn find_trashed(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, HttpError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<RoleResponse>, HttpError>;
    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Vec<RoleResponse>>, HttpError>;

    async fn create_role(
        &self,
        role: &CreateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, HttpError>;
    async fn update_role(
        &self,
        role: &UpdateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, HttpError>;
    async fn trash_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, HttpError>;
    async fn restore_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, HttpError>;
    async fn delete_ole(&self, role_id: i32) -> Result<ApiResponse<()>, HttpError>;
    async fn restore_all_role(&self) -> Result<ApiResponse<()>, HttpError>;
    async fn delete_all_role(&self) -> Result<ApiResponse<()>, HttpError>;
}
