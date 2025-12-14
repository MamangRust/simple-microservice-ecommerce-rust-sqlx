use crate::abstract_trait::user::service::{DynUserCommandService, DynUserQueryService};
use crate::{
    abstract_trait::grpc_client::{role::DynRoleGrpcClient, user_role::DynUserRoleGrpcClient},
    grpc_client::{GrpcClients, role::RoleGrpcClientService, user_role::UserRoleGrpcClientService},
    repository::{command::UserCommandRepository, query::UserQueryRepository},
    service::{
        command::{UserCommandService, UserCommandServiceDeps},
        query::UserQueryService,
    },
};
use anyhow::{Context, Result};
use shared::{
    abstract_trait::DynHashing,
    cache::CacheStore,
    config::{ConnectionPool, RedisPool},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub hash: DynHashing,
    pub redis: RedisPool,
}

#[derive(Clone)]
pub struct DependenciesInject {
    pub user_query: DynUserQueryService,
    pub user_command: DynUserCommandService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("user_query", &"DynUserQueryService")
            .field("user_command", &"DynUserCommandService")
            .finish()
    }
}

impl DependenciesInject {
    pub fn new(deps: DependenciesInjectDeps, clients: GrpcClients) -> Result<Self> {
        let DependenciesInjectDeps { hash, pool, redis } = deps;

        let user_query_repo = Arc::new(UserQueryRepository::new(pool.clone()));
        let user_command_repo = Arc::new(UserCommandRepository::new(pool.clone()));

        let role_client: DynRoleGrpcClient =
            Arc::new(RoleGrpcClientService::new(clients.role_client));
        let user_role_client: DynUserRoleGrpcClient =
            Arc::new(UserRoleGrpcClientService::new(clients.user_role_client));

        let cache = Arc::new(CacheStore::new(redis.pool.clone()));

        let user_query = Arc::new(
            UserQueryService::new(user_query_repo.clone(), cache.clone())
                .context("failed intialize user query")?,
        ) as DynUserQueryService;

        let user_command_deps = UserCommandServiceDeps {
            hash,
            role_client,
            user_role_client,
            query: user_query_repo.clone(),
            command: user_command_repo,
        };

        let user_command = Arc::new(
            UserCommandService::new(user_command_deps).context("failed initialize user command")?,
        ) as DynUserCommandService;

        Ok(Self {
            user_query,
            user_command,
        })
    }
}
