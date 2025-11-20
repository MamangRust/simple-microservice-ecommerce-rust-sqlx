use anyhow::{Context, Result};
use auth::{
    config::{myconfig::Config, server_config::ServerConfig},
    handler::auth::{AuthGrpcServiceDeps, AuthGrpcServiceImpl},
    metrics::metrics_handler,
    state::AppState,
};
use axum::Router;
use genproto::auth::auth_service_server::AuthServiceServer;
use shared::{
    config::ConnectionManager,
    utils::{Telemetry, init_logger},
};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let config = Config::init().context("Failed to load configuration")?;

    let server_config = ServerConfig::from_config(&config)?;

    let telemetry = Telemetry::new("auth-service", "http://otel-collector:4317".to_string());

    let logger_provider = telemetry.init_logger();

    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(logger_provider.clone(), "auth-service");

    info!("Starting server auth initialization...");

    let db_pool = ConnectionManager::new_pool(&server_config.database_url)
        .await
        .context("Failed to initialize database pool")?;

    run_migrations(&db_pool)
        .await
        .context("failed to migration database")?;

    let state = Arc::new(
        AppState::new(db_pool, config)
            .await
            .context("Failed to create AppState")?,
    );

    let auth_grpc_deps = AuthGrpcServiceDeps {
        identity_service: Arc::new(state.di_container.identity_service.clone()),
        login_service: Arc::new(state.di_container.login_service.clone()),
        register_service: Arc::new(state.di_container.register_service.clone()),
        password_reset_service: Arc::new(state.di_container.password_reset_service.clone()),
    };

    let service = AuthGrpcServiceImpl::new(auth_grpc_deps);

    let (shutdown_tx, _) = broadcast::channel(1);

    let grpc_addr = server_config.grpc_addr;
    let grpc_shutdown_rx = shutdown_tx.subscribe();
    let grpc_handle = tokio::spawn(async move {
        loop {
            match start_grpc_server(service.clone(), grpc_addr, grpc_shutdown_rx.resubscribe())
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
                if let Err(e) = signal_shutdown_tx.send(()) {
                    warn!("Failed to send shutdown signal: {}", e);
                }
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
        let _ = tokio::join!(grpc_handle, metrics_handle);
    })
    .await;

    match shutdown_result {
        Ok(()) => info!("✅ All servers shutdown gracefully"),
        Err(_) => {
            warn!("⚠️  Shutdown timeout reached, forcing exit");
        }
    }

    if let Err(e) = telemetry.shutdown().await {
        error!("Failed to shutdown telemetry: {}", e);
    }

    info!("✅ Auth Service shutdown complete.");

    Ok(())
}

async fn start_grpc_server(
    service: AuthGrpcServiceImpl,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting gRPC server on {addr}");

    let shutdown_future = async move {
        let _ = shutdown_rx.recv().await;
        info!("gRPC server received shutdown signal");
    };

    tonic::transport::Server::builder()
        .add_service(AuthServiceServer::new(service))
        .serve_with_shutdown(addr, shutdown_future)
        .await
        .context("gRPC server failed to start or serve")
}

async fn start_metrics_server(
    state: Arc<AppState>,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting metrics server on {}", addr);

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/health", axum::routing::get(health_check))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind metrics listener on {addr}"))?;

    let shutdown_future = async move {
        let _ = shutdown_rx.recv().await;
        info!("Metrics server received shutdown signal");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_future)
        .await
        .context("Metrics server failed to start or serve")
}

async fn health_check() -> &'static str {
    "OK"
}

pub async fn run_migrations(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;

    Ok(())
}
