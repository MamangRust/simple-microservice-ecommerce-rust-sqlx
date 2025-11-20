use std::sync::Arc;

use async_trait::async_trait;

use crate::errors::ServiceError;

pub type DynKafka = Arc<dyn KafkaTrait + Send + Sync>;

#[async_trait]
pub trait KafkaTrait {
    async fn publish(&self, topic: &str, key: &str, value: &[u8]) -> Result<(), ServiceError>;

    async fn subscribe(&self, topics: Vec<&str>, group_id: &str) -> Result<(), ServiceError>;
}
