use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateRefreshToken {
    #[serde(rename = "user_id")]
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[validate(length(min = 1))]
    pub token: String,

    #[serde(rename = "expires_at")]
    #[validate(length(min = 1))]
    pub expired_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateRefreshToken {
    #[serde(rename = "user_id")]
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[validate(length(min = 1))]
    pub token: String,

    #[serde(rename = "expires_at")]
    #[validate(length(min = 1))]
    pub expired_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequest {
    #[serde(rename = "refresh_token")]
    #[validate(length(min = 1))]
    pub refresh_token: String,
}
