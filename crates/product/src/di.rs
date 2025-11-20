use crate::{
    repository::{command::ProductCommandRepository, query::ProductQueryRepository},
    service::{command::ProductCommandService, query::ProductQueryService},
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use shared::{
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
    utils::Metrics,
};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DependenciesInject {
    pub product_query: ProductQueryService,
    pub product_command: ProductCommandService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("product_query", &"OrderQueryService")
            .field("product_command", &"ProductCommandService")
            .finish()
    }
}

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub redis: RedisClient,
}

impl DependenciesInject {
    pub async fn new(deps: DependenciesInjectDeps) -> Result<Self> {
        let DependenciesInjectDeps {
            pool,
            metrics,
            registry,
            redis,
        } = deps;

        let product_query_repo = Arc::new(ProductQueryRepository::new(pool.clone()));
        let product_command_repo = Arc::new(ProductCommandRepository::new(pool.clone()));

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let product_query = ProductQueryService::new(
            product_query_repo.clone(),
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await;

        let product_command = ProductCommandService::new(
            product_command_repo.clone(),
            metrics.clone(),
            registry.clone(),
        )
        .await;

        Ok(Self {
            product_query,
            product_command,
        })
    }
}
