use crate::{
    domain::{
        requests::{CreateRoleRequest, FindAllRole, UpdateRoleRequest},
        responses::{ApiResponse, ApiResponsePagination, RoleResponse, RoleResponseDeleteAt},
    },
    errors::AppErrorHttp,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynRoleGrpcClient = Arc<dyn RoleGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait RoleGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponse>>, AppErrorHttp>;
    async fn find_active(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, AppErrorHttp>;
    async fn find_trashed(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, AppErrorHttp>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<RoleResponse>, AppErrorHttp>;
    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Vec<RoleResponse>>, AppErrorHttp>;

    async fn create_role(
        &self,
        role: &CreateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, AppErrorHttp>;
    async fn update_role(
        &self,
        role: &UpdateRoleRequest,
    ) -> Result<ApiResponse<RoleResponse>, AppErrorHttp>;
    async fn trash_role(
        &self,
        role_id: i32,
    ) -> Result<ApiResponse<RoleResponseDeleteAt>, AppErrorHttp>;
    async fn restore_role(&self, role_id: i32) -> Result<ApiResponse<RoleResponse>, AppErrorHttp>;
    async fn delete_ole(&self, role_id: i32) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn restore_all_role(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn delete_all_role(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
}
