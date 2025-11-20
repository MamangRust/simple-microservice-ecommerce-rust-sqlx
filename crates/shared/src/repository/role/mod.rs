mod command;
mod query;

use self::command::RoleCommandRepository;
use self::query::RoleQueryRepository;

use std::sync::Arc;

use crate::{
    abstract_trait::{DynRoleCommandRepository, DynRoleQueryRepository},
    config::ConnectionPool,
};

#[derive(Clone)]
pub struct RoleRepository {
    pub query: DynRoleQueryRepository,
    pub command: DynRoleCommandRepository,
}

impl RoleRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let query = Arc::new(RoleQueryRepository::new(pool.clone())) as DynRoleQueryRepository;

        let command =
            Arc::new(RoleCommandRepository::new(pool.clone())) as DynRoleCommandRepository;

        Self { query, command }
    }
}
