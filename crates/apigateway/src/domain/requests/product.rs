use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema, IntoParams)]
pub struct FindAllProducts {
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

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    #[schema(example = "Smartphone")]
    pub name: String,

    #[validate(range(min = 1, message = "Price must be greater than zero"))]
    #[schema(example = 99999)]
    pub price: i64,

    #[validate(range(min = 0, message = "Stock cannot be negative"))]
    #[schema(example = 100)]
    pub stock: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateProductRequest {
    pub id: Option<i32>,

    #[validate(length(min = 1, message = "Name is required"))]
    #[schema(example = "Smartphone")]
    pub name: String,

    #[validate(range(min = 1, message = "Price must be greater than zero"))]
    #[schema(example = 99999)]
    pub price: i64,

    #[validate(range(min = 0, message = "Stock cannot be negative"))]
    #[schema(example = 100)]
    pub stock: i32,
}
