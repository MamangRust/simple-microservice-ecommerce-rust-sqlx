use crate::di::{DependenciesInject, DependenciesInjectDeps};
use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use shared::{
    config::{ConnectionPool, RedisClient, RedisConfig},
    utils::{Metrics, SystemMetrics, run_metrics_collector},
};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub di_container: DependenciesInject,
    pub registry: Arc<Mutex<Registry>>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub system_metrics: Arc<SystemMetrics>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("registry", &self.registry)
            .field("metrics", &self.metrics)
            .field("system_metrics", &self.system_metrics)
            .finish()
    }
}

impl AppState {
    pub async fn new(pool: ConnectionPool) -> Result<Self> {
        let registry = Arc::new(Mutex::new(Registry::default()));
        let metrics = Arc::new(Mutex::new(Metrics::new()));
        let system_metrics = Arc::new(SystemMetrics::new());

        let config = RedisConfig::new("redis".into(), 6379, 1, Some("dragon_knight".into()));

        let redis = RedisClient::new(&config)
            .await
            .context("Failed to connect to Redis")
            .unwrap();

        redis.ping().context("Failed to ping Redis server").unwrap();

        let deps = DependenciesInjectDeps {
            pool: pool.clone(),
            metrics: metrics.clone(),
            registry: registry.clone(),
            redis: redis.clone(),
        };

        let di_container = {
            DependenciesInject::new(deps)
                .await
                .context("Failed to initialize dependency injection container")?
        };

        registry.lock().await.register_metrics(&system_metrics);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        Ok(Self {
            di_container,
            registry,
            metrics,
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
