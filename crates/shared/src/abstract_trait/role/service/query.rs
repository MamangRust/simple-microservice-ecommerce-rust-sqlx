use crate::{
    domain::{
        requests::FindAllRole,
        responses::{ApiResponse, ApiResponsePagination, RoleResponse, RoleResponseDeleteAt},
    },
    errors::{RepositoryError, ServiceError},
    model::Role as RoleModel,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynRoleQueryRepository = Arc<dyn RoleQueryRepositoryTrait + Send + Sync>;
pub type DynRoleQueryService = Arc<dyn RoleQueryServiceTrait + Send + Sync>;

#[async_trait]
pub trait RoleQueryRepositoryTrait {
    async fn find_all(&self, req: &FindAllRole) -> Result<(Vec<RoleModel>, i64), RepositoryError>;
    async fn find_active(
        &self,
        req: &FindAllRole,
    ) -> Result<(Vec<RoleModel>, i64), RepositoryError>;
    async fn find_trashed(
        &self,
        req: &FindAllRole,
    ) -> Result<(Vec<RoleModel>, i64), RepositoryError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<RoleModel>, RepositoryError>;
    async fn find_by_user_id(&self, user_id: i32) -> Result<Vec<RoleModel>, RepositoryError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<RoleModel>, RepositoryError>;
}

#[async_trait]
pub trait RoleQueryServiceTrait {
    async fn find_all(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponse>>, ServiceError>;
    async fn find_active(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, ServiceError>;
    async fn find_trashed(
        &self,
        req: &FindAllRole,
    ) -> Result<ApiResponsePagination<Vec<RoleResponseDeleteAt>>, ServiceError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<RoleResponse>, ServiceError>;
    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Vec<RoleResponse>>, ServiceError>;
}
