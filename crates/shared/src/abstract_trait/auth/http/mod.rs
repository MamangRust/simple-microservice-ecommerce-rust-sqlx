use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    domain::{
        requests::{CreateResetPasswordRequest, LoginRequest, RegisterRequest},
        responses::{ApiResponse, TokenResponse, UserResponse},
    },
    errors::AppErrorHttp,
};

pub type DynAuthGrpcClient = Arc<dyn AuthGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait AuthGrpcClientTrait {
    async fn register_user(
        &self,
        input: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorHttp>;
    async fn login_user(
        &self,
        input: &LoginRequest,
    ) -> Result<ApiResponse<TokenResponse>, AppErrorHttp>;
    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, AppErrorHttp>;
    async fn forgot(&self, email: &str) -> Result<ApiResponse<bool>, AppErrorHttp>;
    async fn reset_password(
        &self,
        request: &CreateResetPasswordRequest,
    ) -> Result<ApiResponse<bool>, AppErrorHttp>;
    async fn verify_code(&self, code: &str) -> Result<ApiResponse<bool>, AppErrorHttp>;
    async fn refresh_token(&self, token: &str) -> Result<ApiResponse<TokenResponse>, AppErrorHttp>;
}
