mod command;
mod query;

use self::command::RoleCommandService;
use self::query::RoleQueryService;
use crate::{
    abstract_trait::{
        DynRoleCommandRepository, DynRoleCommandService, DynRoleQueryRepository,
        DynRoleQueryService,
    },
    cache::CacheStore,
    utils::Metrics,
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RoleService {
    pub query: DynRoleQueryService,
    pub command: DynRoleCommandService,
}

impl fmt::Debug for RoleService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RoleService")
            .field("query", &"Arc<dyn RoleQueryServiceTrait>")
            .field("command", &"Arc<dyn RoleCommandServiceTrait>")
            .finish()
    }
}

impl RoleService {
    pub async fn new(
        query: DynRoleQueryRepository,
        command: DynRoleCommandRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let query_service = Arc::new(
            RoleQueryService::new(
                query,
                metrics.clone(),
                registry.clone(),
                cache_store.clone(),
            )
            .await,
        ) as DynRoleQueryService;
        let command_service =
            Arc::new(RoleCommandService::new(command, metrics.clone(), registry.clone()).await)
                as DynRoleCommandService;

        Ok(Self {
            query: query_service,
            command: command_service,
        })
    }
}
