use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct AuthRequest {
    #[validate(email)]
    #[serde(rename = "email")]
    pub email: String,

    #[validate(length(min = 6))]
    #[serde(rename = "password")]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema, Clone)]
pub struct RegisterRequest {
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
}
