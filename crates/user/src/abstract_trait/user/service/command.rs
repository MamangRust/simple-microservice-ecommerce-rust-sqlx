use crate::domain::{
    requests::user::{
        CreateUserRequest, UpdateUserPasswordRequest, UpdateUserRequest, UpdateUserVerifiedRequest,
    },
    response::{
        api::ApiResponse,
        user::{UserResponse, UserResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynUserCommandService = Arc<dyn UserCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserCommandServiceTrait {
    async fn create_user(
        &self,
        req: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn update_user(
        &self,
        req: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn update_user_is_verified(
        &self,
        req: &UpdateUserVerifiedRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn update_user_password(
        &self,
        req: &UpdateUserPasswordRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn trash_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, ServiceError>;
    async fn restore_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<UserResponseDeleteAt>, ServiceError>;
    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_user(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_user(&self) -> Result<ApiResponse<()>, ServiceError>;
}
