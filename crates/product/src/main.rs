use anyhow::{Context, Result};
use axum::Router;
use genproto::product::{
    product_command_service_server::ProductCommandServiceServer,
    product_query_service_server::ProductQueryServiceServer,
};
use product::{
    config::{myconfig::Config, server_config::ServerConfig},
    handler::{command::ProductCommandServiceImpl, query::ProductQueryServiceImpl},
    kafka::{event::OrderEventHandler, kafka_consumer::KafkaEventConsumer},
    metrics::metrics_handler,
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
    dotenv::dotenv().ok();

    let config = Config::init().context("Failed to load configuration")?;
    let server_config = ServerConfig::from_config(&config)?;

    let telemetry = Telemetry::new("product-service", "http://otel-collector:4317".to_string());
    let logger_provider = telemetry.init_logger();
    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(logger_provider.clone(), "product-service");

    info!("🚀 Starting Product Service initialization...");

    let db_pool = ConnectionManager::new_pool(&server_config.database_url)
        .await
        .context("Failed to initialize database pool")?;

    run_migrations(&db_pool)
        .await
        .context("failed to migration database")?;

    let state = Arc::new(
        AppState::new(db_pool)
            .await
            .context("Failed to create AppState")?,
    );

    let command_service =
        ProductCommandServiceImpl::new(Arc::new(state.di_container.product_command.clone()));

    let query_service =
        ProductQueryServiceImpl::new(Arc::new(state.di_container.product_query.clone()));

    let handler = Arc::new(OrderEventHandler::new(Arc::new(
        state.di_container.product_command.clone(),
    )));

    let (shutdown_tx, _) = broadcast::channel(1);

    let kafka_broker = config.clone().kafka_broker.clone();
    let kafka_shutdown_rx = shutdown_tx.subscribe();
    let kafka_handle = tokio::spawn(async move {
        loop {
            let consumer =
                KafkaEventConsumer::new(&kafka_broker, "product-service-group", handler.clone());
            let mut shutdown_rx = kafka_shutdown_rx.resubscribe();

            let kafka_task = tokio::spawn(async move {
                tokio::select! {
                    result = consumer.start() => {
                        match result {
                            Ok(handle) => {
                                handle.await.map_err(|e| anyhow::anyhow!(e)).context("Kafka consumer task failed")
                            }
                            Err(e) => {
                                Err(e).context("Failed to start Kafka consumer")
                            }
                        }
                    },
                    _ = shutdown_rx.recv() => {
                        info!("🛑 Kafka consumer shutting down...");
                        Ok(())
                    }
                }
            });

            match kafka_task.await {
                Ok(Ok(())) => {
                    info!("Kafka consumer stopped gracefully");
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
    });

    let grpc_addr = server_config.grpc_addr;
    let grpc_shutdown_rx = shutdown_tx.subscribe();
    let grpc_handle = tokio::spawn(async move {
        loop {
            match start_grpc_server(
                command_service.clone(),
                query_service.clone(),
                grpc_addr,
                grpc_shutdown_rx.resubscribe(),
            )
            .await
            {
                Ok(()) => {
                    info!("gRPC server stopped gracefully");
                    break;
                }
                Err(e) => {
                    error!("❌ gRPC server failed: {e}. Restarting in 5s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    let metrics_addr = server_config.metrics_addr;
    let state_clone = state.clone();
    let metrics_shutdown_rx = shutdown_tx.subscribe();
    let metrics_handle = tokio::spawn(async move {
        loop {
            info!("🔧 Starting metrics server on {metrics_addr}");
            match start_metrics_server(
                state_clone.clone(),
                metrics_addr,
                metrics_shutdown_rx.resubscribe(),
            )
            .await
            {
                Ok(()) => {
                    info!("Metrics server stopped gracefully");
                    break;
                }
                Err(e) => {
                    error!("❌ Metrics server failed: {e}. Retrying in 3s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
            }
        }
    });

    let signal_shutdown_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("🛑 Shutdown signal received.");
                let _ = signal_shutdown_tx.send(());
            }
            Err(e) => {
                error!("Failed to listen for shutdown signal: {}", e);
            }
        }
    });

    let mut shutdown_rx = shutdown_tx.subscribe();
    let _ = shutdown_rx.recv().await;

    info!("🛑 Shutting down all servers...");

    let shutdown_timeout = tokio::time::Duration::from_secs(30);
    let shutdown_result = tokio::time::timeout(shutdown_timeout, async {
        let _ = tokio::join!(kafka_handle, grpc_handle, metrics_handle);
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

    Ok(())
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

async fn start_metrics_server(
    state: Arc<AppState>,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("📊 Creating metrics server on {addr}");

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/health", axum::routing::get(|| async { "OK" }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind metrics server on {addr}"))?;

    info!("✅ Metrics server bound to {addr}");

    let shutdown_future = async move {
        let _ = shutdown_rx.recv().await;
        info!("Metrics server received shutdown signal");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_future)
        .await
        .with_context(|| format!("Metrics server crashed on {addr}"))
}

pub async fn run_migrations(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;

    Ok(())
}
