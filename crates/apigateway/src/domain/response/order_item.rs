use genproto::order_item::{
    OrderItemResponse as OrderItemResponseProto,
    OrderItemResponseDeleteAt as OrderItemResposeDeleteAtProto,
};
use serde::{Deserialize, Serialize};
use shared::utils::parse_datetime;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct OrderItemResponse {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub quantity: i32,
    pub price: i32,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

impl From<OrderItemResponseProto> for OrderItemResponse {
    fn from(value: OrderItemResponseProto) -> Self {
        OrderItemResponse {
            id: value.id,
            order_id: value.order_id,
            product_id: value.product_id,
            quantity: value.quantity,
            price: value.price,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<OrderItemResponse> for OrderItemResponseProto {
    fn from(value: OrderItemResponse) -> Self {
        OrderItemResponseProto {
            id: value.id,
            order_id: value.order_id,
            product_id: value.product_id,
            quantity: value.quantity,
            price: value.price,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct OrderItemResponseDeleteAt {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub quantity: i32,
    pub price: i32,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<String>,
}

impl From<OrderItemResposeDeleteAtProto> for OrderItemResponseDeleteAt {
    fn from(value: OrderItemResposeDeleteAtProto) -> Self {
        OrderItemResponseDeleteAt {
            id: value.id,
            order_id: value.order_id,
            product_id: value.product_id,
            quantity: value.quantity,
            price: value.price,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
            deleted_at: value.deleted_at.as_deref().and_then(parse_datetime),
        }
    }
}

impl From<OrderItemResponseDeleteAt> for OrderItemResposeDeleteAtProto {
    fn from(value: OrderItemResponseDeleteAt) -> Self {
        OrderItemResposeDeleteAtProto {
            id: value.id,
            order_id: value.order_id,
            product_id: value.product_id,
            quantity: value.quantity,
            price: value.price,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
            deleted_at: Some(value.deleted_at.unwrap_or_default()),
        }
    }
}
