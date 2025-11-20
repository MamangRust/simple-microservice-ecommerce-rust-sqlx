mod command;
mod query;

pub use self::command::{
    DynRoleCommandRepository, DynRoleCommandService, RoleCommandRepositoryTrait,
    RoleCommandServiceTrait,
};
pub use self::query::{
    DynRoleQueryRepository, DynRoleQueryService, RoleQueryRepositoryTrait, RoleQueryServiceTrait,
};
