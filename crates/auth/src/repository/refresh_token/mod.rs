mod command;
mod query;

use crate::abstract_trait::refresh_token::{
    DynRefreshTokenCommandRepository, DynRefreshTokenQueryRepository,
};
use shared::config::ConnectionPool;
use std::sync::Arc;

use self::command::RefreshTokenCommandRepository;
use self::query::RefreshTokenQueryRepository;

#[derive(Clone)]
pub struct RefreshTokenRepository {
    pub query: DynRefreshTokenQueryRepository,
    pub command: DynRefreshTokenCommandRepository,
}

impl RefreshTokenRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let query = Arc::new(RefreshTokenQueryRepository::new(pool.clone()))
            as DynRefreshTokenQueryRepository;

        let command = Arc::new(RefreshTokenCommandRepository::new(pool.clone()))
            as DynRefreshTokenCommandRepository;

        Self { query, command }
    }
}
