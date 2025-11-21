use crate::{
    abstract_trait::DynEmailService,
    config::MyConfigConfig,
    handler::EmailHandler,
    service::{EmailService, KafkaEmailService},
};
use shared::errors::ServiceError;
use std::sync::Arc;
use tracing::info;

pub struct EmailServiceApp {
    config: MyConfigConfig,
}

impl EmailServiceApp {
    pub async fn new(config: MyConfigConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }

    pub async fn run(self) -> Result<(), ServiceError> {
        let email_service = Arc::new(
            EmailService::new(
                &self.config.smtp_user,
                &self.config.smtp_pass,
                &self.config.smtp_server,
                self.config.smtp_port,
            )
            .await,
        ) as DynEmailService;

        let handler = EmailHandler::new(email_service);

        let topics = vec![
            "email-service-topic-auth-register",
            "email-service-topic-auth-forgot-password",
            "email-service-topic-auth-verify-code-success",
        ];

        let kafka_service = KafkaEmailService::new(
            &self.config.kafka_broker,
            "email-service-group",
            &topics,
            handler,
        )?;

        info!("🚀 Starting Email Service...");
        kafka_service.start_consuming().await
    }
}
