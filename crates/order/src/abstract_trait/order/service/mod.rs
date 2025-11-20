mod command;
mod query;

pub use self::command::{DynOrderCommandService, OrderCommandServiceTrait};
pub use self::query::{DynOrderQueryService, OrderQueryServiceTrait};
