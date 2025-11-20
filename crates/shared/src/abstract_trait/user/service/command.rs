use crate::{
    domain::{
        requests::{CreateUserRequest, RegisterRequest, UpdateUserRequest},
        responses::{ApiResponse, UserResponse, UserResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::User as UserModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynUserCommandRepository = Arc<dyn UserCommandRepositoryTrait + Send + Sync>;
pub type DynUserCommandService = Arc<dyn UserCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserCommandRepositoryTrait {
    async fn create_user(&self, req: &CreateUserRequest) -> Result<UserModel, RepositoryError>;
    async fn update_user(&self, req: &UpdateUserRequest) -> Result<UserModel, RepositoryError>;
    async fn update_isverifed(
        &self,
        user_id: i32,
        is_verified: bool,
    ) -> Result<UserModel, RepositoryError>;
    async fn update_password(
        &self,
        user_id: i32,
        password: &str,
    ) -> Result<UserModel, RepositoryError>;
    async fn trash_user(&self, id: i32) -> Result<UserModel, RepositoryError>;
    async fn restore_user(&self, id: i32) -> Result<UserModel, RepositoryError>;
    async fn delete_user(&self, id: i32) -> Result<(), RepositoryError>;
    async fn restore_all_user(&self) -> Result<(), RepositoryError>;
    async fn delete_all_user(&self) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait UserCommandServiceTrait {
    async fn create_user(
        &self,
        req: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn update_user(
        &self,
        req: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn trash_user(&self, id: i32) -> Result<ApiResponse<UserResponseDeleteAt>, ServiceError>;
    async fn restore_user(&self, id: i32) -> Result<ApiResponse<UserResponse>, ServiceError>;
    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_user(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_user(&self) -> Result<ApiResponse<()>, ServiceError>;
}
