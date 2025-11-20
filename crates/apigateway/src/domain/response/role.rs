use genproto::role::{
    RoleResponse as RoleResponseProto, RoleResponseDeleteAt as RoleResponseDeleteAtProto,
};
use serde::{Deserialize, Serialize};
use shared::utils::parse_datetime;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct RoleResponse {
    pub role_id: i32,
    pub role_name: String,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

// proto to response
impl From<RoleResponseProto> for RoleResponse {
    fn from(value: RoleResponseProto) -> Self {
        RoleResponse {
            role_id: value.id,
            role_name: value.name,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

// response to proto
impl From<RoleResponse> for RoleResponseProto {
    fn from(value: RoleResponse) -> Self {
        RoleResponseProto {
            id: value.role_id,
            name: value.role_name,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct RoleResponseDeleteAt {
    pub role_id: i32,
    pub role_name: String,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<String>,
}

// proto to response
impl From<RoleResponseDeleteAtProto> for RoleResponseDeleteAt {
    fn from(value: RoleResponseDeleteAtProto) -> Self {
        RoleResponseDeleteAt {
            role_id: value.id,
            role_name: value.name,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
            deleted_at: value.deleted_at.as_deref().and_then(parse_datetime),
        }
    }
}

// response to proto
impl From<RoleResponseDeleteAt> for RoleResponseDeleteAtProto {
    fn from(value: RoleResponseDeleteAt) -> Self {
        RoleResponseDeleteAtProto {
            id: value.role_id,
            name: value.role_name,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
            deleted_at: Some(value.deleted_at.unwrap_or_default()),
        }
    }
}
