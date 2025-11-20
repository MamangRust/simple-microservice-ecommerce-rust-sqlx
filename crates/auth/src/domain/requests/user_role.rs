use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateUserRoleRequest {
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[serde(rename = "role_id")]
    pub role_id: i32,
}
