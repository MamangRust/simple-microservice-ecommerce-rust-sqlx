use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserRoleRequest {
    #[serde(rename = "user_id")]
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[serde(rename = "role_id")]
    #[validate(range(min = 1))]
    pub role_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RemoveUserRoleRequest {
    #[serde(rename = "user_id")]
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[serde(rename = "role_id")]
    #[validate(range(min = 1))]
    pub role_id: i32,
}
