use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct MyConfigConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub kafka_broker: String,
}

impl MyConfigConfig {
    pub fn init() -> Result<Self> {
        let smtp_username = std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME not set");
        let smtp_password = std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set");
        let smtp_host = std::env::var("SMTP_HOST").expect("SMTP_HOST not set");
        let smtp_port: u16 = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .expect("Invalid SMTP_PORT");
        let kafka_broker = std::env::var("KAFKA").context("Missing environment variable: KAFKA")?;

        Ok(Self {
            smtp_server: smtp_host,
            smtp_port,
            smtp_user: smtp_username,
            smtp_pass: smtp_password,
            kafka_broker,
        })
    }
}
