use crate::{
    abstract_trait::grpc_client::{role::DynRoleGrpcClient, user_role::DynUserRoleGrpcClient},
    grpc_client::{GrpcClients, role::RoleGrpcClientService, user_role::UserRoleGrpcClientService},
    repository::{command::UserCommandRepository, query::UserQueryRepository},
    service::{
        command::{UserCommandService, UserCommandServiceDeps},
        query::UserQueryService,
    },
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynHashing,
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
    utils::Metrics,
};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub hash: DynHashing,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub redis: RedisClient,
}

#[derive(Clone)]
pub struct DependenciesInject {
    pub user_query: UserQueryService,
    pub user_command: UserCommandService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("user_query", &"UserQueryService")
            .field("user_command", &"UserCommandService")
            .finish()
    }
}

impl DependenciesInject {
    pub async fn new(deps: DependenciesInjectDeps, clients: GrpcClients) -> Result<Self> {
        let DependenciesInjectDeps {
            hash,
            pool,
            metrics,
            registry,
            redis,
        } = deps;

        let user_query_repo = Arc::new(UserQueryRepository::new(pool.clone()));
        let user_command_repo = Arc::new(UserCommandRepository::new(pool.clone()));

        let role_client: DynRoleGrpcClient =
            Arc::new(RoleGrpcClientService::new(clients.role_client).await);
        let user_role_client: DynUserRoleGrpcClient =
            Arc::new(UserRoleGrpcClientService::new(clients.user_role_client).await);

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let user_query = UserQueryService::new(
            user_query_repo.clone(),
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await;

        let user_command_deps = UserCommandServiceDeps {
            hash,
            role_client,
            user_role_client,
            query: user_query_repo.clone(),
            command: user_command_repo,
            metrics: metrics.clone(),
            registry: registry.clone(),
        };

        let user_command = UserCommandService::new(user_command_deps).await;

        Ok(Self {
            user_query,
            user_command,
        })
    }
}
