use std::sync::Arc;

use crate::{
    domain::requests::{CreateRefreshToken, UpdateRefreshToken},
    errors::RepositoryError,
    model::RefreshToken as RefreshTokenModel,
};
use anyhow::Result;
use async_trait::async_trait;

pub type DynRefreshTokenQueryRepository = Arc<dyn RefreshTokenQueryRepositoryTrait + Send + Sync>;

pub type DynRefreshTokenCommandRepository =
    Arc<dyn RefreshTokenCommandRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait RefreshTokenQueryRepositoryTrait {
    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<Option<RefreshTokenModel>, RepositoryError>;
    async fn find_by_token(
        &self,
        token: String,
    ) -> Result<Option<RefreshTokenModel>, RepositoryError>;
}

#[async_trait]
pub trait RefreshTokenCommandRepositoryTrait {
    async fn create(
        &self,
        request: &CreateRefreshToken,
    ) -> Result<RefreshTokenModel, RepositoryError>;
    async fn update(
        &self,
        request: &UpdateRefreshToken,
    ) -> Result<RefreshTokenModel, RepositoryError>;
    async fn delete_token(&self, token: String) -> Result<(), RepositoryError>;
    async fn delete_by_user_id(&self, user_id: i32) -> Result<(), RepositoryError>;
}
