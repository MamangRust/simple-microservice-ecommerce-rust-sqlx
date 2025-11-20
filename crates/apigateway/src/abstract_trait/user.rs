use crate::domain::{
    requests::user::{FindAllUsers, UpdateUserRequest},
    response::{
        api::{ApiResponse, ApiResponsePagination},
        user::{UserResponse, UserResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::HttpError;
use std::sync::Arc;

pub type DynUserGrpcClient = Arc<dyn UserGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait UserGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, HttpError>;
    async fn find_active(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, HttpError>;
    async fn find_trashed(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, HttpError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, HttpError>;
    async fn update_user(
        &self,
        req: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, HttpError>;
    async fn trash_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, HttpError>;
    async fn restore_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, HttpError>;
    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, HttpError>;
    async fn restore_all_user(&self) -> Result<ApiResponse<()>, HttpError>;
    async fn delete_all_user(&self) -> Result<ApiResponse<()>, HttpError>;
}
