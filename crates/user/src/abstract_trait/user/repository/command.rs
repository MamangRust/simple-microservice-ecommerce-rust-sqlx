use crate::{
    domain::requests::user::{
        CreateUserRequest, UpdateUserPasswordRequest, UpdateUserRequest, UpdateUserVerifiedRequest,
    },
    model::user::User as UserModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynUserCommandRepository = Arc<dyn UserCommandRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait UserCommandRepositoryTrait {
    async fn create_user(&self, req: &CreateUserRequest) -> Result<UserModel, RepositoryError>;
    async fn update_user(&self, req: &UpdateUserRequest) -> Result<UserModel, RepositoryError>;
    async fn update_isverifed(
        &self,
        req: &UpdateUserVerifiedRequest,
    ) -> Result<UserModel, RepositoryError>;
    async fn update_password(
        &self,
        req: &UpdateUserPasswordRequest,
    ) -> Result<UserModel, RepositoryError>;
    async fn trash_user(&self, id: i32) -> Result<UserModel, RepositoryError>;
    async fn restore_user(&self, id: i32) -> Result<UserModel, RepositoryError>;
    async fn delete_user(&self, id: i32) -> Result<(), RepositoryError>;
    async fn restore_all_user(&self) -> Result<(), RepositoryError>;
    async fn delete_all_user(&self) -> Result<(), RepositoryError>;
}
