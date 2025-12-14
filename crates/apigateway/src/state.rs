use crate::{
    abstract_trait::{rate_limit::DynRateLimitMiddleware, session::DynSessionMiddleware},
    cache::{rate_limit::RateLimiter, session::SessionStore},
    config::GrpcClientConfig,
    di::DependenciesInject,
    service::GrpcClients,
};
use anyhow::{Context, Result};
use shared::{
    abstract_trait::DynJwtService,
    config::{JwtConfig, RedisConfig, RedisPool},
    utils::{SystemMetrics, run_metrics_collector},
};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub jwt_config: DynJwtService,
    pub rate_limit: DynRateLimitMiddleware,
    pub session: DynSessionMiddleware,
    pub di_container: DependenciesInject,
    pub system_metrics: Arc<SystemMetrics>,
    pub redis: Arc<RedisPool>,
}

impl AppState {
    pub async fn new(jwt_secret: &str) -> Result<Self> {
        let jwt_config = Arc::new(JwtConfig::new(jwt_secret)) as DynJwtService;
        let system_metrics = Arc::new(SystemMetrics::new());
        let grpc_config = GrpcClientConfig::init().context("failed config grpc")?;

        info!("Initializing Redis connection for API Gateway");

        let config = RedisConfig::new();

        let redis = RedisPool::new(&config).context("Failed to connect to Redis")?;

        redis.ping().await.context("Failed to ping Redis server")?;

        let rate_limiter_middleware =
            Arc::new(RateLimiter::new(redis.pool.clone())) as DynRateLimitMiddleware;

        let session_middleware =
            Arc::new(SessionStore::new(redis.pool.clone())) as DynSessionMiddleware;

        let clients = GrpcClients::init(grpc_config)
            .await
            .context("failed grpc client")?;

        let di_container = DependenciesInject::new(clients, redis.clone())
            .context("Failed to initialized depencency injection container")?;

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        Ok(Self {
            jwt_config,
            di_container,
            system_metrics,
            rate_limit: rate_limiter_middleware,
            session: session_middleware,
            redis: Arc::new(redis),
        })
    }
}
