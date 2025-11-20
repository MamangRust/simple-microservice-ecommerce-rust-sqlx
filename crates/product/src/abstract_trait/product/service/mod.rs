mod command;
mod query;

pub use self::command::{DynProductCommandService, ProductCommandServiceTrait};
pub use self::query::{DynProductQueryService, ProductQueryServiceTrait};
