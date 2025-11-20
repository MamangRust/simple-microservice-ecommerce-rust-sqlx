use crate::domain::{
    requests::{
        auth::{AuthRequest, RegisterRequest},
        reset_token::CreateResetPasswordRequest,
    },
    response::{api::ApiResponse, token::TokenResponse, user::UserResponse},
};

use anyhow::Result;
use async_trait::async_trait;
use shared::errors::HttpError;
use std::sync::Arc;

pub type DynAuthGrpcClient = Arc<dyn AuthGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait AuthGrpcClientTrait {
    async fn register_user(
        &self,
        input: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, HttpError>;
    async fn login_user(
        &self,
        input: &AuthRequest,
    ) -> Result<ApiResponse<TokenResponse>, HttpError>;
    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, HttpError>;
    async fn forgot(&self, email: &str) -> Result<ApiResponse<bool>, HttpError>;
    async fn reset_password(
        &self,
        request: &CreateResetPasswordRequest,
    ) -> Result<ApiResponse<bool>, HttpError>;
    async fn verify_code(&self, code: &str) -> Result<ApiResponse<bool>, HttpError>;
    async fn refresh_token(&self, token: &str) -> Result<ApiResponse<TokenResponse>, HttpError>;
}
