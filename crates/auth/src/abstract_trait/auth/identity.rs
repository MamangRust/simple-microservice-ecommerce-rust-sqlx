use crate::domain::response::{api::ApiResponse, token::TokenResponse, user::UserResponse};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynIdentityService = Arc<dyn IdentityServiceTrait + Send + Sync>;

#[async_trait]
pub trait IdentityServiceTrait {
    async fn refresh_token(&self, token: &str) -> Result<ApiResponse<TokenResponse>, ServiceError>;
    async fn get_me(&self, id: i32) -> Result<ApiResponse<Option<UserResponse>>, ServiceError>;
}
