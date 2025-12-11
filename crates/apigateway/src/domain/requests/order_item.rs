use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, IntoParams)]
pub struct FindAllOrderItems {
    #[serde(default = "default_page")]
    pub page: i32,

    #[serde(default = "default_page_size")]
    pub page_size: i32,

    #[serde(default)]
    pub search: String,
}

fn default_page() -> i32 {
    1
}

fn default_page_size() -> i32 {
    10
}
