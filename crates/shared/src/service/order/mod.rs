mod command;
mod query;

use self::command::{OrderCommandService, OrderCommandServiceDeps};
use self::query::OrderQueryService;
use crate::{
    abstract_trait::{
        DynKafka, DynOrderCommandRepository, DynOrderCommandService, DynOrderQueryRepository,
        DynOrderQueryService, DynProductQueryRepository,
    },
    cache::CacheStore,
    utils::Metrics,
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct OrderService {
    pub query: DynOrderQueryService,
    pub command: DynOrderCommandService,
}

pub struct OrderServiceDeps {
    pub query: DynOrderQueryRepository,
    pub command: DynOrderCommandRepository,
    pub product_query: DynProductQueryRepository,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub cache_store: Arc<CacheStore>,
}

impl fmt::Debug for OrderService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderService")
            .field("query", &"Arc<dyn OrderQueryServiceTrait>")
            .field("command", &"Arc<dyn OrderCommandServiceTrait>")
            .finish()
    }
}

impl OrderService {
    pub async fn new(deps: OrderServiceDeps) -> Result<Self> {
        let OrderServiceDeps {
            query,
            command,
            product_query,
            kafka,
            metrics,
            registry,
            cache_store,
        } = deps;

        let query_service = Arc::new(
            OrderQueryService::new(
                query.clone(),
                metrics.clone(),
                registry.clone(),
                cache_store.clone(),
            )
            .await,
        ) as DynOrderQueryService;

        let command_deps = OrderCommandServiceDeps {
            product_query: product_query.clone(),
            command: command.clone(),
            query: query.clone(),
            kafka: kafka.clone(),
            metrics: metrics.clone(),
            registry: registry.clone(),
        };

        let command_service =
            Arc::new(OrderCommandService::new(command_deps).await) as DynOrderCommandService;

        Ok(Self {
            query: query_service,
            command: command_service,
        })
    }
}
