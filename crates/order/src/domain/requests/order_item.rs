use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct FindAllOrderItems {
    #[validate(length(min = 1))]
    pub search: String,

    #[validate(range(min = 1))]
    pub page: i32,

    #[validate(range(min = 1, max = 100))]
    #[serde(rename = "page_size")]
    pub page_size: i32,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateOrderItemRecordRequest {
    #[validate(range(min = 1))]
    #[serde(rename = "order_id")]
    pub order_id: i32,

    #[validate(range(min = 1))]
    #[serde(rename = "product_id")]
    pub product_id: i32,

    #[validate(range(min = 1))]
    pub quantity: i32,

    #[validate(range(min = 1))]
    pub price: i32,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateOrderItemRecordRequest {
    #[validate(range(min = 1))]
    #[serde(rename = "order_item_id")]
    pub order_item_id: i32,

    #[validate(range(min = 1))]
    #[serde(rename = "order_id")]
    pub order_id: i32,

    #[validate(range(min = 1))]
    #[serde(rename = "product_id")]
    pub product_id: i32,

    #[validate(range(min = 1))]
    pub quantity: i32,

    #[validate(range(min = 1))]
    pub price: i32,
}
