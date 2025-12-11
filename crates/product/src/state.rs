use crate::di::{DependenciesInject, DependenciesInjectDeps};
use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use shared::{
    config::{ConnectionPool, RedisClient, RedisConfig},
    utils::{SystemMetrics, run_metrics_collector},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub di_container: DependenciesInject,
    pub registry: Arc<Registry>,
    pub system_metrics: Arc<SystemMetrics>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("registry", &self.registry)
            .field("system_metrics", &self.system_metrics)
            .finish()
    }
}

impl AppState {
    pub async fn new(pool: ConnectionPool) -> Result<Self> {
        let mut registry = Registry::default();
        let system_metrics = Arc::new(SystemMetrics::new());

        let config = RedisConfig::new("redis".into(), 6379, 1, Some("dragon_knight".into()));

        let redis = RedisClient::new(&config).context("Failed to connect to Redis")?;

        redis.ping().context("Failed to ping Redis server")?;

        let deps = DependenciesInjectDeps {
            pool: pool.clone(),
            redis: redis.clone(),
        };

        let di_container = DependenciesInject::new(deps, &mut registry)
            .context("Failed to initialize dependency injection container")?;

        registry.register_metrics(&system_metrics);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        Ok(Self {
            di_container,
            registry: Arc::new(registry),
            system_metrics,
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
