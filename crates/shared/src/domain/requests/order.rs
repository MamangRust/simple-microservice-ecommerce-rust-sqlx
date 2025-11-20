use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema, IntoParams)]
pub struct FindAllOrders {
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
pub struct CreateOrderRequest {
    #[validate(range(min = 1, message = "Product ID is required"))]
    #[schema(example = 1)]
    pub product_id: i32,

    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    #[schema(example = 3)]
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateOrderRequest {
    pub id: i32,

    #[validate(range(min = 1, message = "Product ID is required"))]
    #[schema(example = 1)]
    pub product_id: i32,

    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    #[schema(example = 3)]
    pub quantity: i32,
}
