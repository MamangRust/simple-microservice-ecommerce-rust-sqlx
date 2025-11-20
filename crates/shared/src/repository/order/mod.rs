mod command;
mod query;

use std::sync::Arc;

use self::command::OrderCommandRepository;
use self::query::OrderQueryRepository;

use crate::{
    abstract_trait::{DynOrderCommandRepository, DynOrderQueryRepository},
    config::ConnectionPool,
};

#[derive(Clone)]
pub struct OrderRepository {
    pub query: DynOrderQueryRepository,
    pub command: DynOrderCommandRepository,
}

impl OrderRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let query = Arc::new(OrderQueryRepository::new(pool.clone())) as DynOrderQueryRepository;

        let command =
            Arc::new(OrderCommandRepository::new(pool.clone())) as DynOrderCommandRepository;

        Self { query, command }
    }
}
