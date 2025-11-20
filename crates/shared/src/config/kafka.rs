use async_trait::async_trait;
use rdkafka::Message;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaResult;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use tokio::{
    task,
    time::{Duration, sleep},
};
use tracing::{error, info};

use crate::abstract_trait::KafkaTrait;
use crate::errors::ServiceError;

pub struct Kafka {
    producer: BaseProducer,
    brokers: String,
}

impl Kafka {
    pub fn new(brokers: &str) -> Self {
        let producer: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "6000")
            .set("auto.offset.reset", "earliest")
            .set("allow.auto.create.topics", "true")
            .create()
            .expect("Failed to create Kafka producer");
        info!("Kafka producer connected successfully");
        Kafka {
            producer,
            brokers: brokers.to_string(),
        }
    }

    pub fn send_message(&self, topic: &str, key: &str, value: &[u8]) -> KafkaResult<()> {
        if let Err((kafka_error, _record)) = self
            .producer
            .send(BaseRecord::to(topic).key(key).payload(value))
        {
            return Err(kafka_error);
        }

        let _ = self.producer.flush(Duration::from_secs(1));
        info!(topic, "Message sent successfully");
        Ok(())
    }

    pub async fn start_consumer(&self, topics: Vec<&str>, group_id: &str) -> KafkaResult<()> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.brokers)
            .set("group.id", group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set("allow.auto.create.topics", "true")
            .create()
            .expect("Failed to create Kafka consumer");

        consumer.subscribe(&topics)?;

        task::spawn(async move {
            loop {
                match consumer.recv().await {
                    Err(e) => {
                        error!("Kafka receive error: {e:?}");
                        sleep(Duration::from_secs(5)).await;

                        continue;
                    }
                    Ok(m) => {
                        if let Some(payload) = m.payload() {
                            let topic = m.topic();
                            let msg = String::from_utf8_lossy(payload);
                            info!("📥 Received on {topic}: {msg}");
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl KafkaTrait for Kafka {
    async fn publish(&self, topic: &str, key: &str, value: &[u8]) -> Result<(), ServiceError> {
        self.send_message(topic, key, value)
            .map_err(ServiceError::from)
    }

    async fn subscribe(&self, topics: Vec<&str>, group_id: &str) -> Result<(), ServiceError> {
        self.start_consumer(topics, group_id)
            .await
            .map_err(ServiceError::from)
    }
}
