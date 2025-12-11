use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, IntoParams)]
pub struct FindAllOrder {
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

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateOrderRequest {
    #[validate(range(min = 1))]
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[validate(length(min = 1))]
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateOrderRequest {
    #[serde(rename = "order_id")]
    pub order_id: Option<i32>,

    #[validate(range(min = 1))]
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[validate(length(min = 1))]
    pub items: Vec<UpdateOrderItemRequest>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone, ToSchema)]
pub struct CreateOrderItemRequest {
    #[validate(range(min = 1))]
    #[serde(rename = "product_id")]
    pub product_id: i32,

    #[validate(range(min = 1))]
    pub quantity: i32,

    #[validate(range(min = 1))]
    pub price: i32,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone, ToSchema)]
pub struct UpdateOrderItemRequest {
    #[validate(range(min = 1))]
    #[serde(rename = "order_item_id")]
    pub order_item_id: i32,

    #[validate(range(min = 1))]
    #[serde(rename = "product_id")]
    pub product_id: i32,

    #[validate(range(min = 1))]
    pub quantity: i32,

    #[validate(range(min = 1))]
    pub price: i32,
}
