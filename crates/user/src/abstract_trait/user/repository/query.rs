use crate::{domain::requests::user::FindAllUsers, model::user::User as UserModel};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynUserQueryRepository = Arc<dyn UserQueryRepositoryTrait + Send + Sync>;

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
    async fn find_by_email(&self, email: String) -> Result<Option<UserModel>, RepositoryError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<UserModel>, RepositoryError>;
    async fn find_by_email_and_verify(
        &self,
        email: String,
    ) -> Result<Option<UserModel>, RepositoryError>;
    async fn find_verify_code(&self, code: String) -> Result<Option<UserModel>, RepositoryError>;
}
