use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::EmailRequest;
use shared::errors::ServiceError;
pub type DynEmailService = Arc<dyn EmailServiceTrait>;

#[async_trait]
pub trait EmailServiceTrait: Send + Sync {
    async fn send(&self, req: &EmailRequest) -> Result<(), ServiceError>;
}
