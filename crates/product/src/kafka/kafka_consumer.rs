use crate::{domain::event::OrderEvent, kafka::event::OrderEventHandler};
use anyhow::Result;
use rdkafka::{
    Message,
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
};
use std::sync::Arc;
use tokio::{
    sync::oneshot,
    task::{JoinHandle, spawn},
    time::{Duration, sleep},
};
use tracing::{debug, error, info, warn};

pub struct KafkaEventConsumer {
    consumer: StreamConsumer,
    handler: Arc<OrderEventHandler>,
}

impl KafkaEventConsumer {
    pub fn new(brokers: &str, group_id: &str, handler: Arc<OrderEventHandler>) -> Self {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest")
            .create()
            .expect("Failed to create Kafka consumer");

        Self { consumer, handler }
    }

    pub async fn start(self) -> Result<JoinHandle<()>> {
        self.consumer
            .subscribe(&["order.created", "order.updated", "order.deleted"])?;

        info!("✅ Kafka consumer started, subscribed to order events");

        let handler = self.handler;
        let consumer = self.consumer;

        let handle = spawn(async move {
            loop {
                match consumer.recv().await {
                    Err(e) => {
                        error!("Kafka receive error: {e}");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                    Ok(message) => {
                        let topic = message.topic().to_string();

                        let payload = match message.payload() {
                            None => {
                                error!(topic, "Empty message payload");
                                continue;
                            }
                            Some(p) => {
                                debug!(topic, payload_len = p.len(), "Payload received");
                                p
                            }
                        };

                        let key = message
                            .key()
                            .and_then(|k| std::str::from_utf8(k).ok())
                            .map(|s| s.to_string());

                        if let Some(ref k) = key {
                            debug!(topic, key = k, "Received message");
                        } else {
                            warn!(topic, "Message has no key");
                        }

                        let event_result: Result<OrderEvent, _> = serde_json::from_slice(payload);
                        let event = match event_result {
                            Ok(e) => {
                                debug!(topic, key = ?key, event_type = ?e, "Event deserialized successfully");
                                e
                            }
                            Err(e) => {
                                error!(
                                    topic,
                                    key = ?key,
                                    "Failed to deserialize event: {e}"
                                );
                                continue;
                            }
                        };

                        if let Some(key_str) = &key {
                            let expected_id = match &event {
                                OrderEvent::Created { order_id, .. } => *order_id,
                                OrderEvent::Updated { order_id, .. } => *order_id,
                                OrderEvent::Deleted { order_id, .. } => *order_id,
                            };

                            if let Ok(key_id) = key_str.parse::<i32>()
                                && key_id != expected_id
                            {
                                warn!(
                                    topic,
                                    key,
                                    event_order_id = expected_id,
                                    "Key ID does not match event order_id"
                                );
                                debug!(topic, key_id, expected_id, "Key mismatch detected");
                            }
                        }

                        if let Err(e) = handler.handle_event(event).await {
                            error!(topic, key = ?key, "Failed to handle event: {e}");
                        } else {
                            info!(topic, key = ?key, "✅ Event processed successfully");
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    pub async fn start_with_shutdown(self, mut shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
        self.consumer
            .subscribe(&["order.created", "order.updated", "order.deleted"])?;

        info!("✅ Kafka consumer started, subscribed to order events");

        let handler = self.handler;
        let consumer = self.consumer;

        loop {
            tokio::select! {

                _ = &mut shutdown_rx => {
                    info!("🛑 Kafka consumer received shutdown signal");
                    break;
                }

                message_result = consumer.recv() => {
                    match message_result {
                        Err(e) => {
                            error!("Kafka receive error: {e}");
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                        Ok(message) => {
                            let topic = message.topic().to_string();

                            let payload = match message.payload() {
                                None => {
                                    error!(topic, "Empty message payload");
                                    continue;
                                }
                                Some(p) => {
                                    debug!(topic, payload_len = p.len(), "Payload received");
                                    p
                                }
                            };

                            let key = message
                                .key()
                                .and_then(|k| std::str::from_utf8(k).ok())
                                .map(|s| s.to_string());

                            if let Some(ref k) = key {
                                debug!(topic, key = k, "Received message");
                            } else {
                                warn!(topic, "Message has no key");
                            }

                            let event_result: Result<OrderEvent, _> = serde_json::from_slice(payload);
                            let event = match event_result {
                                Ok(e) => {
                                    debug!(topic, key = ?key, event_type = ?e, "Event deserialized successfully");
                                    e
                                }
                                Err(e) => {
                                    error!(
                                        topic,
                                        key = ?key,
                                        "Failed to deserialize event: {e}"
                                    );
                                    continue;
                                }
                            };

                            if let Some(key_str) = &key {
                                let expected_id = match &event {
                                    OrderEvent::Created { order_id, .. } => *order_id,
                                    OrderEvent::Updated { order_id, .. } => *order_id,
                                    OrderEvent::Deleted { order_id, .. } => *order_id,
                                };

                                if let Ok(key_id) = key_str.parse::<i32>()
                                    && key_id != expected_id
                                {
                                    warn!(
                                        topic,
                                        key,
                                        event_order_id = expected_id,
                                        "Key ID does not match event order_id"
                                    );
                                    debug!(topic, key_id, expected_id, "Key mismatch detected");
                                }
                            }

                            if let Err(e) = handler.handle_event(event).await {
                                error!(topic, key = ?key, "Failed to handle event: {e}");
                            } else {
                                info!(topic, key = ?key, "✅ Event processed successfully");
                            }
                        }
                    }
                }
            }
        }

        info!("✅ Kafka consumer stopped gracefully");
        Ok(())
    }
}
