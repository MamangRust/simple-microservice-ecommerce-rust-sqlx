use crate::{
    abstract_trait::grpc_client::{role::DynRoleGrpcClient, user_role::DynUserRoleGrpcClient},
    grpc_client::{GrpcClients, role::RoleGrpcClientService, user_role::UserRoleGrpcClientService},
    repository::{command::UserCommandRepository, query::UserQueryRepository},
    service::{
        command::{UserCommandService, UserCommandServiceDeps},
        query::UserQueryService,
    },
};
use anyhow::{Result, Context};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynHashing,
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub hash: DynHashing,
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
    pub fn new(deps: DependenciesInjectDeps, clients: GrpcClients, registry: &mut Registry) -> Result<Self> {
        let DependenciesInjectDeps {
            hash,
            pool,
            redis,
        } = deps;

        let user_query_repo = Arc::new(UserQueryRepository::new(pool.clone()));
        let user_command_repo = Arc::new(UserCommandRepository::new(pool.clone()));

        let role_client: DynRoleGrpcClient =
            Arc::new(RoleGrpcClientService::new(clients.role_client));
        let user_role_client: DynUserRoleGrpcClient =
            Arc::new(UserRoleGrpcClientService::new(clients.user_role_client));

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let user_query = UserQueryService::new(
            user_query_repo.clone(),
            registry,
            cache.clone(),
        ).context("failed intialize user query")?;

        let user_command_deps = UserCommandServiceDeps {
            hash,
            role_client,
            user_role_client,
            query: user_query_repo.clone(),
            command: user_command_repo,
        };

        let user_command = UserCommandService::new(user_command_deps, registry).context("failed initialize user command")?;

        Ok(Self {
            user_query,
            user_command,
        })
    }
}
