use crate::{
    domain::{
        requests::LoginRequest,
        responses::{ApiResponse, TokenResponse},
    },
    errors::ServiceError,
};

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynLoginService = Arc<dyn LoginServiceTrait + Send + Sync>;

#[async_trait]
pub trait LoginServiceTrait {
    async fn login(
        &self,
        request: &LoginRequest,
    ) -> Result<ApiResponse<TokenResponse>, ServiceError>;
}
