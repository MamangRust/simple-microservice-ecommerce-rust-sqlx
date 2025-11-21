use crate::{
    abstract_trait::{rate_limit::DynRateLimitMiddleware, session::DynSessionMiddleware},
    cache::{rate_limit::RateLimiter, session::SessionStore},
    config::GrpcClientConfig,
    di::DependenciesInject,
    service::GrpcClients,
};
use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynJwtService,
    config::{JwtConfig, RedisClient, RedisConfig},
    utils::{Metrics, SystemMetrics, run_metrics_collector},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub jwt_config: DynJwtService,
    pub rate_limit: DynRateLimitMiddleware,
    pub session: DynSessionMiddleware,
    pub di_container: DependenciesInject,
    pub registry: Arc<Mutex<Registry>>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub system_metrics: Arc<SystemMetrics>,
    pub redis: Arc<RedisClient>,
}

impl AppState {
    pub async fn new(jwt_secret: &str) -> Result<Self> {
        let jwt_config = Arc::new(JwtConfig::new(jwt_secret)) as DynJwtService;
        let registry = Arc::new(Mutex::new(Registry::default()));
        let metrics = Arc::new(Mutex::new(Metrics::new()));
        let system_metrics = Arc::new(SystemMetrics::new());
        let grpc_config = GrpcClientConfig::init().context("failed config grpc")?;

        info!("Initializing Redis connection for API Gateway");
        let redis_config = RedisConfig::new("redis".into(), 6379, 0, Some("dragon_knight".into()));
        let redis = RedisClient::new(&redis_config)
            .await
            .context("Failed to connect to Redis")?;

        redis.ping().context("Failed to ping Redis server")?;

        let rate_limiter_middleware =
            Arc::new(RateLimiter::new(redis.client.clone())) as DynRateLimitMiddleware;

        let session_middleware =
            Arc::new(SessionStore::new(redis.client.clone())) as DynSessionMiddleware;

        let clients = GrpcClients::init(grpc_config)
            .await
            .context("failed grpc client")?;

        let di_container = {
            DependenciesInject::new(clients, metrics.clone(), registry.clone())
                .await
                .context("Failed to initialized depencency injection container")?
        };

        registry.lock().await.register_metrics(&system_metrics);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        Ok(Self {
            jwt_config,
            di_container,
            registry,
            metrics,
            system_metrics,
            rate_limit: rate_limiter_middleware,
            session: session_middleware,
            redis: Arc::new(redis),
        })
    }
}

trait MetricsRegister {
    fn register_metrics(&mut self, metrics: &SystemMetrics);
}

impl MetricsRegister for Registry {
    fn register_metrics(&mut self, metrics: &SystemMetrics) {
        metrics.register(self);
    }
}
