mod command;
mod query;

pub use self::command::{DynUserCommandRepository, UserCommandRepositoryTrait};
pub use self::query::{DynUserQueryRepository, UserQueryRepositoryTrait};
