use crate::domain::{
    requests::user::FindAllUsers,
    response::{
        api::{ApiResponse, ApiResponsePagination},
        user::{UserResponse, UserResponseDeleteAt, UserResponseWithPassword},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynUserQueryService = Arc<dyn UserQueryServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserQueryServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ServiceError>;
    async fn find_active(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, ServiceError>;
    async fn find_trashed(
        &self,
        req: &FindAllUsers,
    ) -> Result<ApiResponsePagination<Vec<UserResponseDeleteAt>>, ServiceError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, ServiceError>;

    async fn find_by_email(&self, email: String)
    -> Result<ApiResponse<UserResponse>, ServiceError>;

    async fn find_by_email_and_verify(
        &self,
        email: String,
    ) -> Result<ApiResponse<UserResponseWithPassword>, ServiceError>;

    async fn find_by_verification_code(
        &self,
        code: String,
    ) -> Result<ApiResponse<UserResponse>, ServiceError>;
}
