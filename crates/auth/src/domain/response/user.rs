use genproto::user::{
    UserResponse as UserResponseProto, UserResponseWithPassword as UserResponseWithPasswordProto,
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
pub struct UserResponseWithPassword {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub password: String,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

impl From<UserResponseWithPasswordProto> for UserResponseWithPassword {
    fn from(value: UserResponseWithPasswordProto) -> Self {
        UserResponseWithPassword {
            id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            password: value.password,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<UserResponseWithPassword> for UserResponseWithPasswordProto {
    fn from(value: UserResponseWithPassword) -> Self {
        UserResponseWithPasswordProto {
            id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            password: value.password,
            created_at: value.created_at.unwrap_or_default(),
            updated_at: value.updated_at.unwrap_or_default(),
        }
    }
}
