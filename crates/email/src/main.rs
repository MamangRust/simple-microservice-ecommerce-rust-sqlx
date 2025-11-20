use anyhow::Context;
use email::app::EmailServiceApp;
use shared::{
    config::Config,
    utils::{Telemetry, init_logger, shutdown_signal},
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let config = Config::init().context("failed to load config")?;

    let telemetry = Telemetry::new("email-service", "http://otel-collector:4317".to_string());
    let logger_provider = telemetry.init_logger();

    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(logger_provider.clone(), "email-service");

    let app = EmailServiceApp::new(config.email_config, &config.kafka_broker).await?;

    app.run().await?;

    info!("✅ User Service shutdown gracefully.");
    telemetry.shutdown().await?;

    shutdown_signal().await;

    Ok(())
}
