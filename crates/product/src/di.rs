use crate::{
    repository::{command::ProductCommandRepository, query::ProductQueryRepository},
    service::{command::ProductCommandService, query::ProductQueryService},
};
use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use shared::{
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
};
use std::{fmt, sync::Arc};

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
    pub redis: RedisClient,
}

impl DependenciesInject {
    pub fn new(deps: DependenciesInjectDeps, registry: &mut Registry) -> Result<Self> {
        let DependenciesInjectDeps { pool, redis } = deps;

        let product_query_repo = Arc::new(ProductQueryRepository::new(pool.clone()));
        let product_command_repo = Arc::new(ProductCommandRepository::new(pool.clone()));

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let product_query =
            ProductQueryService::new(product_query_repo.clone(), registry, cache.clone())
                .context("failed initialize product query")?;

        let product_command = ProductCommandService::new(product_command_repo.clone(), registry)
            .context("failed initialize product command")?;

        Ok(Self {
            product_query,
            product_command,
        })
    }
}
