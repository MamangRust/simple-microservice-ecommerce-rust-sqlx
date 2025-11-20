mod command;
mod query;

use std::sync::Arc;

use crate::abstract_trait::reset_token::{
    DynResetTokenCommandRepository, DynResetTokenQueryRepository,
};
use shared::config::ConnectionPool;

use self::command::ResetTokenCommandRepository;
use self::query::ResetTokenQueryRepository;

#[derive(Clone)]
pub struct ResetTokenRepository {
    pub query: DynResetTokenQueryRepository,
    pub command: DynResetTokenCommandRepository,
}

impl ResetTokenRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let query =
            Arc::new(ResetTokenQueryRepository::new(pool.clone())) as DynResetTokenQueryRepository;

        let command = Arc::new(ResetTokenCommandRepository::new(pool.clone()))
            as DynResetTokenCommandRepository;

        Self { query, command }
    }
}
