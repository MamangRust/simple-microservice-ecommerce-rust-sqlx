use genproto::user::{
    UserResponse as UserResponseProto, UserResponseDeleteAt as UserResponseDeleteAtProto,
};
use serde::{Deserialize, Serialize};
use shared::utils::parse_datetime;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct UserResponse {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

impl From<UserResponseProto> for UserResponse {
    fn from(value: UserResponseProto) -> Self {
        UserResponse {
            id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<UserResponse> for UserResponseProto {
    fn from(value: UserResponse) -> Self {
        UserResponseProto {
            id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct UserResponseDeleteAt {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<String>,
}

impl From<UserResponseDeleteAtProto> for UserResponseDeleteAt {
    fn from(value: UserResponseDeleteAtProto) -> Self {
        UserResponseDeleteAt {
            id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
            deleted_at: value.deleted_at.as_deref().and_then(parse_datetime),
        }
    }
}

impl From<UserResponseDeleteAt> for UserResponseDeleteAtProto {
    fn from(value: UserResponseDeleteAt) -> Self {
        UserResponseDeleteAtProto {
            id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
            deleted_at: Some(value.deleted_at.unwrap_or_default()),
        }
    }
}
