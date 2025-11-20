use crate::{
    abstract_trait::grpc_client::DynProductGrpcClient,
    grpc_client::{GrpcClients, product::ProductGrpcClientService},
    repository::{
        order::{OrderCommandRepository, OrderQueryRepository},
        order_item::{command::OrderItemCommandRepository, query::OrderItemQueryRepository},
    },
    service::{
        order::{OrderCommandService, OrderCommandServiceDeps, OrderQueryService},
        order_item::OrderItemQueryService,
    },
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynKafka,
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
    utils::Metrics,
};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DependenciesInject {
    pub order_query: OrderQueryService,
    pub order_command: OrderCommandService,
    pub order_item_query: OrderItemQueryService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("order_query", &"OrderQueryService")
            .field("order_command", &"OrderCommandService")
            .field("order_item_query", &"OrderItemQueryService")
            .finish()
    }
}

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub redis: RedisClient,
}

impl DependenciesInject {
    pub async fn new(deps: DependenciesInjectDeps, clients: GrpcClients) -> Result<Self> {
        let DependenciesInjectDeps {
            kafka,
            pool,
            metrics,
            registry,
            redis,
        } = deps;

        let order_query_repo = Arc::new(OrderQueryRepository::new(pool.clone()));
        let order_command_repo = Arc::new(OrderCommandRepository::new(pool.clone()));
        let order_item_query_repo = Arc::new(OrderItemQueryRepository::new(pool.clone()));
        let order_item_command_repo = Arc::new(OrderItemCommandRepository::new(pool.clone()));

        let product_client: DynProductGrpcClient =
            Arc::new(ProductGrpcClientService::new(clients.product_query_client).await);

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let order_query = OrderQueryService::new(
            order_query_repo.clone(),
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await;

        let order_command_deps = OrderCommandServiceDeps {
            product_client,
            order_item_query: order_item_query_repo.clone(),
            order_item_command: order_item_command_repo,
            query: order_query_repo.clone(),
            command: order_command_repo,
            kafka,
            metrics: metrics.clone(),
            registry: registry.clone(),
        };

        let order_command = OrderCommandService::new(order_command_deps).await;

        let order_item_query = OrderItemQueryService::new(
            order_item_query_repo.clone(),
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await;

        Ok(Self {
            order_query,
            order_command,
            order_item_query,
        })
    }
}
