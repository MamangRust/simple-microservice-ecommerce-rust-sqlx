use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateResetPasswordRequest {
    #[serde(rename = "reset_token")]
    #[validate(length(min = 1))]
    pub reset_token: String,

    #[validate(length(min = 6))]
    pub password: String,

    #[serde(rename = "confirm_password")]
    #[validate(length(min = 6))]
    pub confirm_password: String,
}
