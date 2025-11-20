use crate::model::product::Product as ProductModel;
use shared::utils::parse_datetime;

use genproto::product::{
    ProductResponse as ProductResponseProto,
    ProductResponseDeleteAt as ProductResponseDeleteAtProto,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ProductResponse {
    pub id: i32,
    pub name: String,
    pub price: i64,
    pub stock: i32,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

// dari model to response
impl From<ProductModel> for ProductResponse {
    fn from(value: ProductModel) -> Self {
        ProductResponse {
            id: value.product_id,
            name: value.name,
            price: value.price,
            stock: value.stock,
            created_at: value.created_at.map(|dt| dt.to_string()),
            updated_at: value.updated_at.map(|dt| dt.to_string()),
        }
    }
}

// dari proto to response
impl From<ProductResponseProto> for ProductResponse {
    fn from(value: ProductResponseProto) -> Self {
        ProductResponse {
            id: value.id,
            name: value.name,
            price: value.price,
            stock: value.stock,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

// response to proto
impl From<ProductResponse> for ProductResponseProto {
    fn from(value: ProductResponse) -> Self {
        ProductResponseProto {
            id: value.id,
            name: value.name,
            price: value.price,
            stock: value.stock,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ProductResponseDeleteAt {
    pub id: i32,
    pub name: String,
    pub price: i64,
    pub stock: i32,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<String>,
}

// dari model to response
impl From<ProductModel> for ProductResponseDeleteAt {
    fn from(value: ProductModel) -> Self {
        ProductResponseDeleteAt {
            id: value.product_id,
            name: value.name,
            price: value.price,
            stock: value.stock,
            created_at: value.created_at.map(|dt| dt.to_string()),
            updated_at: value.updated_at.map(|dt| dt.to_string()),
            deleted_at: value.deleted_at.map(|dt| dt.to_string()),
        }
    }
}

// proto to response
impl From<ProductResponseDeleteAtProto> for ProductResponseDeleteAt {
    fn from(value: ProductResponseDeleteAtProto) -> Self {
        ProductResponseDeleteAt {
            id: value.id,
            name: value.name,
            price: value.price,
            stock: value.stock,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
            deleted_at: value.deleted_at.as_deref().and_then(parse_datetime),
        }
    }
}

// response to proto
impl From<ProductResponseDeleteAt> for ProductResponseDeleteAtProto {
    fn from(value: ProductResponseDeleteAt) -> Self {
        ProductResponseDeleteAtProto {
            id: value.id,
            name: value.name,
            price: value.price,
            stock: value.stock,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
            deleted_at: Some(value.deleted_at.unwrap_or_default()),
        }
    }
}
