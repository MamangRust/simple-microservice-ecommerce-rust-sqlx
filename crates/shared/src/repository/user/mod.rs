mod command;
mod query;

use self::command::UserCommandRepository;
use self::query::UserQueryRepository;

use crate::{
    abstract_trait::{DynUserCommandRepository, DynUserQueryRepository},
    config::ConnectionPool,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct UserRepository {
    pub query: DynUserQueryRepository,
    pub command: DynUserCommandRepository,
}

impl UserRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let query = Arc::new(UserQueryRepository::new(pool.clone())) as DynUserQueryRepository;
        let command =
            Arc::new(UserCommandRepository::new(pool.clone())) as DynUserCommandRepository;

        Self { query, command }
    }
}
