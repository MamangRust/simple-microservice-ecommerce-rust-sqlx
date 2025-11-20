use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1))]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateRoleRequest {
    pub id: Option<i32>,

    #[validate(length(min = 1))]
    pub name: String,
}
