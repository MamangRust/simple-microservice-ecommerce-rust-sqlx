mod command;
mod query;

pub use self::command::{DynRoleCommandRepository, RoleCommandRepositoryTrait};
pub use self::query::{DynRoleQueryRepository, RoleQueryRepositoryTrait};
