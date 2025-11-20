mod command;
mod query;

use self::command::ProductCommandService;
use self::query::ProductQueryService;
use crate::{
    abstract_trait::{
        DynProductCommandRepository, DynProductCommandService, DynProductQueryRepository,
        DynProductQueryService,
    },
    cache::CacheStore,
    utils::Metrics,
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ProductService {
    pub query: DynProductQueryService,
    pub command: DynProductCommandService,
}

impl fmt::Debug for ProductService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProductService")
            .field("query", &"Arc<dyn ProductQueryServiceTrait>")
            .field("command", &"Arc<dyn ProductCommandServiceTrait>")
            .finish()
    }
}

impl ProductService {
    pub async fn new(
        query: DynProductQueryRepository,
        command: DynProductCommandRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let query_service = Arc::new(
            ProductQueryService::new(
                query,
                metrics.clone(),
                registry.clone(),
                cache_store.clone(),
            )
            .await,
        ) as DynProductQueryService;
        let command_service =
            Arc::new(ProductCommandService::new(command, metrics.clone(), registry.clone()).await)
                as DynProductCommandService;

        Ok(Self {
            query: query_service,
            command: command_service,
        })
    }
}
