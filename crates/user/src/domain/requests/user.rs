use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct FindAllUsers {
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
pub struct RegisterRequest {
    pub first_name: String,
    pub last_name: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 6))]
    pub password: String,

    #[validate(length(min = 6))]
    pub confirm_password: String,

    pub verified_code: String,
    pub is_verified: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct CreateUserRequest {
    #[serde(rename = "firstname")]
    pub first_name: String,

    #[serde(rename = "lastname")]
    pub last_name: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 6))]
    pub password: String,

    #[validate(length(min = 6))]
    pub confirm_password: String,

    pub verified_code: String,
    pub is_verified: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct UpdateUserRequest {
    pub user_id: Option<i32>,

    #[serde(rename = "firstname")]
    pub first_name: String,

    #[serde(rename = "lastname")]
    pub last_name: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 6))]
    pub password: String,

    #[validate(length(min = 6))]
    pub confirm_password: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserVerifiedRequest {
    #[validate(range(min = 1))]
    pub user_id: i32,

    pub is_verified: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserPasswordRequest {
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[validate(length(min = 6))]
    pub password: String,
}
