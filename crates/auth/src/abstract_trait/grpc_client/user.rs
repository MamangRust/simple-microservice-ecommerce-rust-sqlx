use crate::domain::{
    requests::user::{CreateUserRequest, UpdateUserPasswordRequest, UpdateUserVerifiedRequest},
    response::{
        api::ApiResponse,
        user::{UserResponse, UserResponseWithPassword},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::AppErrorGrpc;
use std::sync::Arc;

pub type DynUserGrpcClient = Arc<dyn UserGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait UserGrpcClientTrait {
    async fn create_user(
        &self,
        req: CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc>;

    async fn update_user_is_verified(
        &self,
        req: UpdateUserVerifiedRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc>;

    async fn update_user_password(
        &self,
        req: UpdateUserPasswordRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc>;

    async fn find_verification_code(
        &self,
        code: String,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc>;

    async fn find_by_email(&self, email: String)
    -> Result<ApiResponse<UserResponse>, AppErrorGrpc>;

    async fn find_by_email_and_verify(
        &self,
        email: String,
    ) -> Result<ApiResponse<UserResponseWithPassword>, AppErrorGrpc>;

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, AppErrorGrpc>;
}
