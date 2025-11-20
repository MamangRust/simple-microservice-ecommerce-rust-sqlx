mod command;
mod query;

pub use self::command::{
    DynProductCommandRepository, DynProductCommandService, ProductCommandRepositoryTrait,
    ProductCommandServiceTrait,
};
pub use self::query::{
    DynProductQueryRepository, DynProductQueryService, ProductQueryRepositoryTrait,
    ProductQueryServiceTrait,
};
