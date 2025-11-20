use crate::{errors::ServiceError, utils::EmailTemplateData};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type DynEmailService = Arc<dyn EmailServiceTrait>;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailRequest {
    pub to: String,
    pub subject: String,
    pub data: EmailTemplateData,
}

#[async_trait]
pub trait EmailServiceTrait: Send + Sync {
    async fn send(&self, req: &EmailRequest) -> Result<(), ServiceError>;
}
