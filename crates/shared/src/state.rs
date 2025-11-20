use crate::{
    abstract_trait::{DynHashing, DynJwtService, DynKafka},
    config::{Config, ConnectionPool, Hashing, JwtConfig, Kafka, RedisClient, RedisConfig},
    di::{DependenciesInject, DependenciesInjectDeps},
    utils::{Metrics, SystemMetrics, run_metrics_collector},
};
use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub di_container: DependenciesInject,
    pub jwt_config: DynJwtService,
    pub kafka_config: DynKafka,
    pub registry: Arc<Mutex<Registry>>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub system_metrics: Arc<SystemMetrics>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("di_container", &self.di_container)
            .field("jwt_config", &"<dyn JwtService>")
            .field("kafka_config", &"<dyn Kafka>")
            .finish()
    }
}

impl AppState {
    pub async fn new(pool: ConnectionPool, config: Config) -> Result<Self> {
        let jwt_config = Arc::new(JwtConfig::new(&config.jwt_secret)) as DynJwtService;
        let hashing = Arc::new(Hashing::new()) as DynHashing;
        let kafka_config = Arc::new(Kafka::new(&config.kafka_broker)) as DynKafka;
        let registry = Arc::new(Mutex::new(Registry::default()));
        let metrics = Arc::new(Mutex::new(Metrics::new()));
        let system_metrics = Arc::new(SystemMetrics::new());

        let config = RedisConfig::new("localhost".into(), 6379, 1, Some("dragon_knight".into()));

        let redis = RedisClient::new(&config)
            .await
            .context("Failed to connect to Redis")
            .unwrap();

        redis.ping().context("Failed to ping Redis server").unwrap();

        let deps = DependenciesInjectDeps {
            pool: pool.clone(),
            hash: hashing.clone(),
            jwt_config: jwt_config.clone(),
            kafka: kafka_config.clone(),
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
            jwt_config,
            kafka_config,
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
