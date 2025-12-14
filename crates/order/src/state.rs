use crate::{
    config::{grpc_config::GrpcClientConfig, myconfig::Config},
    di::{DependenciesInject, DependenciesInjectDeps},
    grpc_client::GrpcClients,
};
use anyhow::{Context, Result};
use shared::{
    abstract_trait::DynKafka,
    config::{ConnectionPool, Kafka, RedisConfig, RedisPool},
    utils::{SystemMetrics, run_metrics_collector},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub di_container: DependenciesInject,
    pub kafka_config: DynKafka,
    pub system_metrics: Arc<SystemMetrics>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("deps", &self.di_container)
            .field("system_metrics", &self.system_metrics)
            .finish()
    }
}

impl AppState {
    pub async fn new(pool: ConnectionPool, config: Config) -> Result<Self> {
        let kafka_config = Arc::new(Kafka::new(&config.kafka_broker)) as DynKafka;
        let system_metrics = Arc::new(SystemMetrics::new());

        let config = RedisConfig::new();

        let redis = RedisPool::new(&config).context("Failed to connect to Redis")?;

        redis.ping().await.context("Failed to ping Redis server")?;

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
            DependenciesInject::new(deps, clients)
                .await
                .context("Failed to initialize dependency injection container")?
        };

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        Ok(Self {
            di_container,
            kafka_config,
            system_metrics,
        })
    }
}
