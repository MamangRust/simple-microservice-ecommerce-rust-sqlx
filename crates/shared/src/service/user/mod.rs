mod command;
mod query;

use self::command::UserCommandService;
use self::query::UserQueryService;
use crate::{
    abstract_trait::{
        DynUserCommandRepository, DynUserCommandService, DynUserQueryRepository,
        DynUserQueryService,
    },
    cache::CacheStore,
    utils::Metrics,
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct UserService {
    pub query: DynUserQueryService,
    pub command: DynUserCommandService,
}

impl fmt::Debug for UserService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserService")
            .field("query", &"Arc<dyn UserQueryServiceTrait>")
            .field("command", &"Arc<dyn UserCommandServiceTrait>")
            .finish()
    }
}

impl UserService {
    pub async fn new(
        query: DynUserQueryRepository,
        command: DynUserCommandRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
        cache_store: Arc<CacheStore>,
    ) -> Result<Self> {
        let query_service = Arc::new(
            UserQueryService::new(
                query,
                metrics.clone(),
                registry.clone(),
                cache_store.clone(),
            )
            .await,
        ) as DynUserQueryService;

        let command_service =
            Arc::new(UserCommandService::new(command, metrics.clone(), registry.clone()).await)
                as DynUserCommandService;

        Ok(Self {
            query: query_service,
            command: command_service,
        })
    }
}
