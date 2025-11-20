use crate::domain::{
    requests::auth::RegisterRequest,
    response::{api::ApiResponse, user::UserResponse},
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynRegisterService = Arc<dyn RegisterServiceTrait + Send + Sync>;

#[async_trait]
pub trait RegisterServiceTrait {
    async fn register(
        &self,
        register_request: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
}
