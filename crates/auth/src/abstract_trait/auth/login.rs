use crate::domain::{
    requests::auth::AuthRequest,
    response::{api::ApiResponse, token::TokenResponse},
};

use shared::errors::ServiceError;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynLoginService = Arc<dyn LoginServiceTrait + Send + Sync>;

#[async_trait]
pub trait LoginServiceTrait {
    async fn login(
        &self,
        request: &AuthRequest,
    ) -> Result<ApiResponse<TokenResponse>, ServiceError>;
}
