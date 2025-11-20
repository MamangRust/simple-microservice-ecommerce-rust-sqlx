use shared::utils::parse_datetime;

use genproto::product::ProductResponse as ProductResponseProto;
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
