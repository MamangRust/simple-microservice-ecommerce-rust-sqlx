use std::sync::Arc;

use crate::{
    domain::{requests::CreateResetPasswordRequest, responses::ApiResponse},
    errors::ServiceError,
};
use async_trait::async_trait;

pub type DynPasswordResetService = Arc<dyn PasswordServiceTrait + Send + Sync>;

#[async_trait]
pub trait PasswordServiceTrait {
    async fn forgot(&self, email: &str) -> Result<ApiResponse<bool>, ServiceError>;
    async fn reset_password(
        &self,
        request: &CreateResetPasswordRequest,
    ) -> Result<ApiResponse<bool>, ServiceError>;
    async fn verify_code(&self, code: &str) -> Result<ApiResponse<bool>, ServiceError>;
}
