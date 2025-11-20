use crate::handler::EmailHandler;
use rdkafka::{
    config::ClientConfig,
    consumer::{CommitMode, Consumer, StreamConsumer},
};
use shared::errors::ServiceError;
use tokio::time::{Duration, sleep};
use tracing::{error, info};

pub struct KafkaEmailService {
    consumer: StreamConsumer,
    handler: EmailHandler,
}

impl KafkaEmailService {
    pub fn new(
        brokers: &str,
        group_id: &str,
        topics: &[&str],
        handler: EmailHandler,
    ) -> Result<Self, ServiceError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set("allow.auto.create.topics", "false")
            .create()
            .map_err(ServiceError::from)?;

        consumer.subscribe(topics).map_err(ServiceError::from)?;

        Ok(Self { consumer, handler })
    }

    pub async fn start_consuming(&self) -> Result<(), ServiceError> {
        info!("🚀 Starting Kafka consumer...");

        loop {
            match self.consumer.recv().await {
                Err(e) => {
                    error!("⚠️ Kafka not connected or error: {e:?}");

                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
                Ok(message) => {
                    if let Err(e) = self.handler.handle_message(&message).await {
                        error!("Failed to handle message: {e:?}");
                    }

                    if let Err(e) = self.consumer.commit_message(&message, CommitMode::Async) {
                        error!("Failed to commit message: {e:?}");
                    }
                }
            }
        }
    }
}
