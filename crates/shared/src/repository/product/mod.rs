mod command;
mod query;

use self::command::ProductCommandRepository;
use self::query::ProductQueryRepository;

use crate::{
    abstract_trait::{DynProductCommandRepository, DynProductQueryRepository},
    config::ConnectionPool,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct ProductRepository {
    pub query: DynProductQueryRepository,
    pub command: DynProductCommandRepository,
}

impl ProductRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let query =
            Arc::new(ProductQueryRepository::new(pool.clone())) as DynProductQueryRepository;

        let command =
            Arc::new(ProductCommandRepository::new(pool.clone())) as DynProductCommandRepository;

        Self { query, command }
    }
}
