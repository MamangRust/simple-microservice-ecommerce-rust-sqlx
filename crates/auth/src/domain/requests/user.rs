use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateUserVerifiedRequest {
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[serde(rename = "is_verified")]
    pub is_verified: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateUserPasswordRequest {
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[validate(length(min = 6))]
    #[serde(rename = "password")]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[serde(rename = "firstname")]
    pub first_name: String,

    #[serde(rename = "lastname")]
    pub last_name: String,

    #[validate(email)]
    #[serde(rename = "email")]
    pub email: String,

    #[validate(length(min = 6))]
    #[serde(rename = "password")]
    pub password: String,

    #[validate(length(min = 6))]
    #[serde(rename = "confirm_password")]
    pub confirm_password: String,

    #[serde(rename = "verified_code")]
    pub verified_code: String,

    #[serde(rename = "is_verified")]
    pub is_verified: bool,
}
