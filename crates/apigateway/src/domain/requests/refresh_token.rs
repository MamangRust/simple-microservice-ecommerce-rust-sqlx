use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequest {
    #[serde(rename = "refresh_token")]
    #[validate(length(min = 1))]
    pub refresh_token: String,
}
