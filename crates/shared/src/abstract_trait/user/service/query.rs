use crate::{
    domain::{
        requests::FindAllUsers,
        responses::{ApiResponse, ApiResponsePagination, UserResponse, UserResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::User as UserModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynUserQueryRepository = Arc<dyn UserQueryRepositoryTrait + Send + Sync>;
pub type DynUserQueryService = Arc<dyn UserQueryServiceTrait + Send + Sync>;

#[async_trait]
pub trait UserQueryRepositoryTrait {
    async fn find_all(&self, req: &FindAllUsers) -> Result<(Vec<UserModel>, i64), RepositoryError>;
    async fn find_active(
        &self,
        req: &FindAllUsers,
    ) -> Result<(Vec<UserModel>, i64), RepositoryError>;
    async fn find_trashed(
        &self,
        req: &FindAllUsers,
    ) -> Result<(Vec<UserModel>, i64), RepositoryError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserModel>, RepositoryError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<UserModel>, RepositoryError>;
    async fn find_by_email_and_verify(
        &self,
        email: &str,
    ) -> Result<Option<UserModel>, RepositoryError>;
    async fn find_verify_code(&self, code: &str) -> Result<Option<UserModel>, RepositoryError>;
}

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
}
