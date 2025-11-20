use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct FindAllOrderItems {
    #[validate(length(min = 1))]
    pub search: String,

    #[validate(range(min = 1))]
    pub page: i32,

    #[validate(range(min = 1, max = 100))]
    #[serde(rename = "page_size")]
    pub page_size: i32,
}
