use crate::model::order::Order as OrderModel;
use genproto::order::{
    OrderResponse as OrderResponseProto, OrderResponseDeleteAt as OrderResponseDeleteAtProto,
};
use serde::{Deserialize, Serialize};
use shared::utils::parse_datetime;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct OrderResponse {
    pub id: i32,
    pub user_id: i32,
    pub total_price: i32,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

// model to response
impl From<OrderModel> for OrderResponse {
    fn from(value: OrderModel) -> Self {
        OrderResponse {
            id: value.order_id,
            user_id: value.user_id,
            total_price: value.total_price,
            created_at: value.created_at.map(|dt| dt.to_string()),
            updated_at: value.updated_at.map(|dt| dt.to_string()),
        }
    }
}

// proto to response
impl From<OrderResponseProto> for OrderResponse {
    fn from(value: OrderResponseProto) -> Self {
        OrderResponse {
            id: value.id,
            user_id: value.user_id,
            total_price: value.total_price,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

// response to proto
impl From<OrderResponse> for OrderResponseProto {
    fn from(value: OrderResponse) -> Self {
        OrderResponseProto {
            id: value.id,
            user_id: value.user_id,
            total_price: value.total_price,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct OrderResponseDeleteAt {
    pub id: i32,
    pub user_id: i32,
    pub total_price: i32,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<String>,
}

// model to response
impl From<OrderModel> for OrderResponseDeleteAt {
    fn from(value: OrderModel) -> Self {
        OrderResponseDeleteAt {
            id: value.order_id,
            user_id: value.user_id,
            total_price: value.total_price,
            created_at: value.created_at.map(|dt| dt.to_string()),
            updated_at: value.updated_at.map(|dt| dt.to_string()),
            deleted_at: value.deleted_at.map(|dt| dt.to_string()),
        }
    }
}

// proto to response
impl From<OrderResponseDeleteAtProto> for OrderResponseDeleteAt {
    fn from(value: OrderResponseDeleteAtProto) -> Self {
        OrderResponseDeleteAt {
            id: value.id,
            user_id: value.user_id,
            total_price: value.total_price,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
            deleted_at: value.deleted_at.as_deref().and_then(parse_datetime),
        }
    }
}

// response to proto
impl From<OrderResponseDeleteAt> for OrderResponseDeleteAtProto {
    fn from(value: OrderResponseDeleteAt) -> Self {
        OrderResponseDeleteAtProto {
            id: value.id,
            user_id: value.user_id,
            total_price: value.total_price,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
            deleted_at: Some(value.deleted_at.unwrap_or_default()),
        }
    }
}
