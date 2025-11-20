use std::sync::Arc;

use crate::{
    domain::requests::CreateResetTokenRequest, errors::RepositoryError,
    model::ResetToken as ResetTokenModel,
};
use anyhow::Result;
use async_trait::async_trait;

pub type DynResetTokenQueryRepository = Arc<dyn ResetTokenQueryRepositoryTrait + Send + Sync>;
pub type DynResetTokenCommandRepository = Arc<dyn ResetTokenCommandRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait ResetTokenQueryRepositoryTrait {
    async fn find_by_token(&self, token: &str) -> Result<Option<ResetTokenModel>, RepositoryError>;
}

#[async_trait]
pub trait ResetTokenCommandRepositoryTrait {
    async fn create_reset_token(
        &self,
        request: &CreateResetTokenRequest,
    ) -> Result<ResetTokenModel, RepositoryError>;
    async fn delete_reset_token(&self, user_id: i32) -> Result<(), RepositoryError>;
}
