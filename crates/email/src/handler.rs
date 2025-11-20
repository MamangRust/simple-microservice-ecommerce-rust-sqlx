use crate::{abstract_trait::EmailServiceTrait, domain::EmailRequest};
use rdkafka::{Message, message::BorrowedMessage};
use shared::errors::ServiceError;
use std::sync::Arc;
use tracing::error;

pub struct EmailHandler {
    mailer: Arc<dyn EmailServiceTrait>,
}

impl EmailHandler {
    pub fn new(mailer: Arc<dyn EmailServiceTrait>) -> Self {
        Self { mailer }
    }

    pub async fn handle_message(&self, message: &BorrowedMessage<'_>) -> Result<(), ServiceError> {
        self.process_message(message).await
    }

    async fn process_message(&self, message: &BorrowedMessage<'_>) -> Result<(), ServiceError> {
        let payload = message
            .payload()
            .ok_or_else(|| ServiceError::Custom("Empty message payload".to_string()))?;

        let payload_str = String::from_utf8_lossy(payload);

        let email_payload: EmailRequest = serde_json::from_slice(payload).map_err(|e| {
            error!("Failed to unmarshal message: {e}, payload: {payload_str}");
            ServiceError::Custom(format!("Invalid JSON payload: {e}"))
        })?;

        let email_request = EmailRequest {
            to: email_payload.to,
            subject: email_payload.subject,
            data: email_payload.data,
        };

        self.mailer.send(&email_request).await.map_err(|e| {
            error!("Failed to send email: {e:?}");
            ServiceError::Custom(format!("Email sending failed: {e:?}"))
        })
    }
}
