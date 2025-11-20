use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema, IntoParams)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, message = "First name must be at least 2 characters"))]
    pub firstname: String,

    #[validate(length(min = 2, message = "Last name must be at least 2 characters"))]
    pub lastname: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 6))]
    pub password: String,

    #[serde(rename = "confirm_password")]
    #[validate(length(min = 6))]
    pub confirm_password: String,

    pub is_verified: bool,

    pub verification_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRequest {
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[validate(length(min = 2, message = "First name must be at least 2 characters"))]
    pub firstname: String,

    #[validate(length(min = 2, message = "Last name must be at least 2 characters"))]
    pub lastname: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 6))]
    pub password: String,

    #[serde(rename = "confirm_password")]
    #[validate(length(min = 6))]
    pub confirm_password: String,
}
