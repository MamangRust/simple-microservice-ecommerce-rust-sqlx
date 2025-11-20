use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::abstract_trait::KafkaTrait;
use crate::errors::ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_type: String,
    pub aggregate_id: String,
    pub data: serde_json::Value,
    pub timestamp: String,
    pub version: i32,
}

impl DomainEvent {
    pub fn new(
        event_type: String,
        aggregate_id: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type,
            aggregate_id: aggregate_id.clone(),
            data,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: 1,
        }
    }
}

pub struct EventBus {
    kafka: Arc<dyn KafkaTrait>,
}

impl EventBus {
    pub fn new(kafka: Arc<dyn KafkaTrait>) -> Self {
        Self { kafka }
    }

    pub async fn publish(&self, topic: &str, event: &DomainEvent) -> Result<(), ServiceError> {
        let payload = serde_json::to_vec(event)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        self.kafka
            .publish(&topic, &event.aggregate_id, &payload)
            .await
            .map_err(|e| {
                error!("Failed to publish event to {}: {:?}", topic, e);
                ServiceError::Internal(format!("Event publishing failed: {}", e))
            })?;

        info!(
            "Event published: type={}, aggregate_id={}, topic={}",
            event.event_type, event.aggregate_id, topic
        );

        Ok(())
    }

    pub async fn subscribe(
        &self,
        topics: Vec<&str>,
        group_id: &str,
    ) -> Result<(), ServiceError> {
        debug!("Subscribing to topics: {:?} with group_id: {}", topics, group_id);

        self.kafka
            .subscribe(topics, group_id)
            .await
            .map_err(|e| {
                error!("Failed to subscribe to topics: {:?}", e);
                ServiceError::Internal(format!("Subscription failed: {}", e))
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserEvent {
    UserCreated {
        user_id: String,
        email: String,
        username: String,
    },
    UserUpdated {
        user_id: String,
        email: String,
    },
    UserDeleted {
        user_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProductEvent {
    ProductCreated {
        product_id: String,
        name: String,
        price: f64,
    },
    ProductUpdated {
        product_id: String,
        name: String,
        price: f64,
    },
    ProductDeleted {
        product_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderEvent {
    OrderCreated {
        order_id: String,
        user_id: String,
        total: f64,
    },
    OrderUpdated {
        order_id: String,
        status: String,
    },
    OrderCancelled {
        order_id: String,
    },
}
