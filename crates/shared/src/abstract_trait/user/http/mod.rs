use crate::{
    domain::{
        requests::{FindAllUsers, RegisterRequest, UpdateUserRequest},
        responses::{ApiResponse, ApiResponsePagination, UserResponse, UserResponseDeleteAt},
    },
    errors::AppErrorHttp,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynUserGrpcClient = Arc<dyn UserGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait UserGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, AppErrorHttp>;
    async fn find_active(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, AppErrorHttp>;
    async fn find_trashed(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, AppErrorHttp>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, AppErrorHttp>;
    async fn create_user(
        &self,
        req: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorHttp>;
    async fn update_user(
        &self,
        req: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorHttp>;
    async fn trash_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, AppErrorHttp>;
    async fn restore_user(&self, id: i32) -> Result<ApiResponse<UserResponse>, AppErrorHttp>;
    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn restore_all_user(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn delete_all_user(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
}
