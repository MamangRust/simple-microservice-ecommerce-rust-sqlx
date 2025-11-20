mod command;
mod query;

pub use self::command::{DynProductCommandRepository, ProductCommandRepositoryTrait};
pub use self::query::{DynProductQueryRepository, ProductQueryRepositoryTrait};
