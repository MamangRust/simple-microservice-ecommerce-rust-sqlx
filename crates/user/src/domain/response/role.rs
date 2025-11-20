use genproto::role::RoleResponse as RoleResponseProto;
use serde::{Deserialize, Serialize};
use shared::utils::parse_datetime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleResponse {
    pub id: i32,
    pub name: String,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

impl From<RoleResponseProto> for RoleResponse {
    fn from(value: RoleResponseProto) -> Self {
        RoleResponse {
            id: value.id,
            name: value.name,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<RoleResponse> for RoleResponseProto {
    fn from(value: RoleResponse) -> Self {
        RoleResponseProto {
            id: value.id,
            name: value.name,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}
