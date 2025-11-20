use crate::{abstract_trait::EmailServiceTrait, domain::EmailRequest};

use shared::{errors::ServiceError, utils::render_email};

use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Mailbox, Message},
    transport::smtp::authentication::Credentials,
};
use tracing::{error, info};

type SmtpTransport = AsyncSmtpTransport<Tokio1Executor>;

#[derive(Clone)]
pub struct EmailService {
    mailer: SmtpTransport,
    from: Mailbox,
}

impl EmailService {
    pub async fn new(username: &str, password: &str, host: &str, port: u16) -> Self {
        let creds = Credentials::new(username.to_string(), password.to_string());

        let mailer = SmtpTransport::starttls_relay(host)
            .expect("❌ Failed to create SMTP relay")
            .credentials(creds)
            .port(port)
            .build();

        let from: Mailbox = "no-reply@sanedge.com"
            .parse()
            .expect("❌ Invalid sender email format");

        Self { mailer, from }
    }
}

#[async_trait]
impl EmailServiceTrait for EmailService {
    async fn send(&self, req: &EmailRequest) -> Result<(), ServiceError> {
        let body = render_email(&req.data).map_err(|e| {
            error!("❌ Failed to render email template: {}", e);
            ServiceError::Custom(format!("Failed to render email template: {e}"))
        })?;

        let to: Mailbox = req.to.parse().map_err(|e| {
            error!("❌ Invalid recipient email: {}", e);
            ServiceError::Custom(format!("Invalid recipient email: {e}"))
        })?;

        let email = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(&req.subject)
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(body)
            .map_err(|e| {
                error!("❌ Failed to build email: {}", e);
                ServiceError::Custom(format!("Failed to build email: {e}"))
            })?;

        match self.mailer.send(email).await {
            Ok(_) => {
                info!("✅ Email sent to {}", req.to);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to send email to {}: {}", req.to, e);
                Err(ServiceError::Custom(format!("Failed to send email: {e}")))
            }
        }
    }
}
