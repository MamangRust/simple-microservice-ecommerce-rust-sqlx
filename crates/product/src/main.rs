use anyhow::{Context, Result};
use genproto::product::{
    product_command_service_server::ProductCommandServiceServer,
    product_query_service_server::ProductQueryServiceServer,
};
use product::{
    config::{myconfig::Config, server_config::ServerConfig},
    handler::{command::ProductCommandServiceImpl, query::ProductQueryServiceImpl},
    kafka::{event::OrderEventHandler, kafka_consumer::KafkaEventConsumer},
    state::AppState,
};
use shared::{
    config::ConnectionManager,
    utils::{Telemetry, init_logger},
};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::transport::Server;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    let (config, server_config, state, telemetry) =
        setup().await.context("Failed to setup application")?;

    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    let server_handles = run_servers(config, server_config, state, shutdown_tx.clone())
        .await
        .context("Failed to start servers")?;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("🛑 Shutdown signal received (Ctrl+C).");
        }
        _ = shutdown_rx.recv() => {
            info!("🛑 Shutdown signal received from internal component.");
        }
    }

    shutdown(telemetry, server_handles).await;

    Ok(())
}

async fn setup() -> Result<(Config, ServerConfig, Arc<AppState>, Telemetry)> {
    dotenv::dotenv().ok();

    let is_dev = std::env::var("DEV_MODE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let is_enable_file = std::env::var("ENABLE_FILE_LOG")
        .map(|v| v == "true")
        .unwrap_or(false);

    let config = Config::init().context("Failed to load configuration")?;
    let server_config = ServerConfig::from_config(&config)?;

    let telemetry = Telemetry::new("product-service", "http://otel-collector:4317".to_string());
    let logger_provider = telemetry.init_logger();
    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(
        logger_provider.clone(),
        "product-service",
        is_dev,
        is_enable_file,
    );

    info!("🚀 Starting Product Service initialization...");

    let db_pool = ConnectionManager::new_pool(
        &server_config.database_url,
        config.db_min_conn,
        config.db_max_conn,
    )
    .await
    .context("Failed to initialize database pool")?;

    run_migrations(&db_pool)
        .await
        .context("Failed to run database migrations")?;

    let state = Arc::new(
        AppState::new(db_pool)
            .await
            .context("Failed to create AppState")?,
    );

    info!("✅ Application setup completed successfully.");
    Ok((config, server_config, state, telemetry))
}

struct ServerHandles {
    kafka_handle: tokio::task::JoinHandle<()>,
    grpc_handle: tokio::task::JoinHandle<()>,
}

async fn run_servers(
    config: Config,
    server_config: ServerConfig,
    state: Arc<AppState>,
    shutdown_tx: broadcast::Sender<()>,
) -> Result<ServerHandles> {
    let command_service =
        ProductCommandServiceImpl::new(Arc::new(state.di_container.product_command.clone()));

    let query_service =
        ProductQueryServiceImpl::new(Arc::new(state.di_container.product_query.clone()));

    let handler = Arc::new(OrderEventHandler::new(Arc::new(
        state.di_container.product_command.clone(),
    )));

    let kafka_broker = config.kafka_broker.clone();
    let kafka_handle = spawn_kafka_consumer(kafka_broker, handler, shutdown_tx.clone());

    let grpc_addr = server_config.grpc_addr;
    let grpc_handle = run_grpc_server(
        command_service,
        query_service,
        grpc_addr,
        shutdown_tx.clone(),
    );

    shutdown_listener(shutdown_tx);

    Ok(ServerHandles {
        kafka_handle,
        grpc_handle,
    })
}

fn spawn_kafka_consumer(
    kafka_broker: String,
    handler: Arc<OrderEventHandler>,
    shutdown_tx: broadcast::Sender<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let shutdown_rx = shutdown_tx.subscribe();

        loop {
            info!("🔄 Starting Kafka consumer...");

            let consumer =
                KafkaEventConsumer::new(&kafka_broker, "product-service-group", handler.clone());
            let mut consumer_shutdown_rx = shutdown_rx.resubscribe();

            let kafka_task = tokio::spawn(async move {
                tokio::select! {
                    result = consumer.start() => {
                        match result {
                            Ok(handle) => {
                                handle.await
                                    .map_err(|e| anyhow::anyhow!(e))
                                    .context("Kafka consumer task failed")
                            }
                            Err(e) => {
                                Err(e).context("Failed to start Kafka consumer")
                            }
                        }
                    },
                    _ = consumer_shutdown_rx.recv() => {
                        info!("🛑 Kafka consumer shutting down...");
                        Ok(())
                    }
                }
            });

            match kafka_task.await {
                Ok(Ok(())) => {
                    info!("✅ Kafka consumer stopped gracefully");
                    break;
                }
                Ok(Err(e)) => {
                    error!("💀 Kafka consumer error: {e}. Restarting in 5s...");
                }
                Err(e) => {
                    if e.is_cancelled() {
                        info!("Kafka consumer task cancelled during shutdown");
                        break;
                    } else {
                        error!("💀 Kafka task panicked: {e}. Restarting in 5s...");
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    })
}

fn run_grpc_server(
    command_service: ProductCommandServiceImpl,
    query_service: ProductQueryServiceImpl,
    grpc_addr: std::net::SocketAddr,
    shutdown_tx: broadcast::Sender<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let shutdown_rx = shutdown_tx.subscribe();

        loop {
            info!("📡 Attempting to start gRPC server on {grpc_addr}");

            let server_shutdown_rx = shutdown_rx.resubscribe();

            match start_grpc_server(
                command_service.clone(),
                query_service.clone(),
                grpc_addr,
                server_shutdown_rx,
            )
            .await
            {
                Ok(()) => {
                    info!("✅ gRPC server stopped gracefully");
                    break;
                }
                Err(e) => {
                    error!("❌ gRPC server failed: {e}. Restarting in 5s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    })
}

fn shutdown_listener(shutdown_tx: broadcast::Sender<()>) {
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("🛑 Ctrl+C signal detected, broadcasting shutdown...");
                if let Err(e) = shutdown_tx.send(()) {
                    warn!("Failed to send shutdown signal: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to listen for shutdown signal: {}", e);
            }
        }
    });
}

async fn start_grpc_server(
    command_service: ProductCommandServiceImpl,
    query_service: ProductQueryServiceImpl,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("📡 Starting gRPC server on {addr}");

    let shutdown_future = async move {
        let _ = shutdown_rx.recv().await;
        info!("gRPC server received shutdown signal");
    };

    Server::builder()
        .add_service(ProductCommandServiceServer::new(command_service))
        .add_service(ProductQueryServiceServer::new(query_service))
        .serve_with_shutdown(addr, shutdown_future)
        .await
        .with_context(|| format!("gRPC server failed to start on {addr}"))
}

async fn shutdown(telemetry: Telemetry, server_handles: ServerHandles) {
    info!("🛑 Shutting down all servers...");

    let shutdown_timeout = tokio::time::Duration::from_secs(30);
    let shutdown_result = tokio::time::timeout(shutdown_timeout, async {
        let _ = tokio::join!(server_handles.kafka_handle, server_handles.grpc_handle,);
    })
    .await;

    match shutdown_result {
        Ok(()) => info!("✅ All components shutdown gracefully"),
        Err(_) => {
            warn!("⚠️  Shutdown timeout reached, forcing exit");
        }
    }

    if let Err(e) = telemetry.shutdown().await {
        error!("Failed to shutdown telemetry: {}", e);
    }

    info!("✅ Product Service shutdown complete.");
}

pub async fn run_migrations(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
