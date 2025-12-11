use crate::{
    config::{grpc_config::GrpcClientConfig, myconfig::Config},
    di::{DependenciesInject, DependenciesInjectDeps},
    grpc_client::GrpcClients,
};
use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynKafka,
    config::{ConnectionPool, Kafka, RedisClient, RedisConfig},
    utils::{SystemMetrics, run_metrics_collector},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub di_container: DependenciesInject,
    pub registry: Arc<Registry>,
    pub kafka_config: DynKafka,
    pub system_metrics: Arc<SystemMetrics>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("deps", &self.di_container)
            .field("registry", &self.registry)
            .field("system_metrics", &self.system_metrics)
            .finish()
    }
}

impl AppState {
    pub async fn new(pool: ConnectionPool, config: Config) -> Result<Self> {
        let kafka_config = Arc::new(Kafka::new(&config.kafka_broker)) as DynKafka;
        let mut registry = Registry::default();
        let system_metrics = Arc::new(SystemMetrics::new());

        let config = RedisConfig::new("redis".into(), 6379, 1, Some("dragon_knight".into()));

        let redis = RedisClient::new(&config).context("Failed to connect to Redis")?;

        redis.ping().context("Failed to ping Redis server")?;

        let deps = DependenciesInjectDeps {
            pool: pool.clone(),
            kafka: kafka_config.clone(),
            redis: redis.clone(),
        };

        let grpc_config = GrpcClientConfig::init().context("failed config grpc")?;

        let clients = GrpcClients::init(grpc_config)
            .await
            .context("failed grpc client")?;

        let di_container = {
            DependenciesInject::new(deps, clients, &mut registry)
                .await
                .context("Failed to initialize dependency injection container")?
        };

        registry.register_metrics(&system_metrics);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        Ok(Self {
            di_container,
            kafka_config,
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
