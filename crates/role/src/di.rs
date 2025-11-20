use crate::{
    repository::{
        role::{RoleCommandRepository, RoleQueryRepository},
        user_role::UserRoleRepository,
    },
    service::{
        role::{RoleCommandService, RoleQueryService},
        user_role::UserRoleCommandService,
    },
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
    pub role_query: RoleQueryService,
    pub role_command: RoleCommandService,
    pub user_role_command: UserRoleCommandService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("role_query", &"RoleQueryService")
            .field("role_command", &"RoleCommandService")
            .field("user_role_command", &"UserRoleCommandService")
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

        let role_query_repo = Arc::new(RoleQueryRepository::new(pool.clone()));
        let role_command_repo = Arc::new(RoleCommandRepository::new(pool.clone()));
        let user_role_repo = Arc::new(UserRoleRepository::new(pool.clone()));

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let role_query = RoleQueryService::new(
            role_query_repo.clone(),
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await;

        let role_command =
            RoleCommandService::new(role_command_repo.clone(), metrics.clone(), registry.clone())
                .await;

        let user_role_command =
            UserRoleCommandService::new(user_role_repo.clone(), metrics.clone(), registry.clone())
                .await;

        Ok(Self {
            role_query,
            role_command,
            user_role_command,
        })
    }
}
