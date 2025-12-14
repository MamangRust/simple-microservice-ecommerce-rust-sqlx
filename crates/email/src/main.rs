use anyhow::Context;
use email::{app::EmailServiceApp, config::MyConfigConfig};
use shared::utils::{Telemetry, init_logger, shutdown_signal};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let is_dev = std::env::var("DEV_MODE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let is_enable_file = std::env::var("ENABLE_FILE_LOG")
        .map(|v| v == "true")
        .unwrap_or(false);

    let config = MyConfigConfig::init().context("failed to load config")?;

    let telemetry = Telemetry::new("email-service", "http://otel-collector:4317".to_string());
    let logger_provider = telemetry.init_logger();

    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(
        logger_provider.clone(),
        "email-service",
        is_dev,
        is_enable_file,
    );

    let app = EmailServiceApp::new(config).await?;

    app.run().await?;

    info!("✅ User Service shutdown gracefully.");
    telemetry.shutdown().await?;

    shutdown_signal().await;

    Ok(())
}
