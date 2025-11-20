use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct FindAllUsers {
    #[validate(length(min = 1))]
    pub search: String,

    #[validate(range(min = 1))]
    pub page: i32,

    #[validate(range(min = 1, max = 100))]
    pub page_size: i32,
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
