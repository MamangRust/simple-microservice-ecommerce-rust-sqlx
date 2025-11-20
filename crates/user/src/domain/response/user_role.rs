use genproto::user_role::UserRoleResponse as UserRoleResponseProto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRoleResponse {
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[serde(rename = "role_id")]
    pub role_id: i32,
}

impl From<UserRoleResponseProto> for UserRoleResponse {
    fn from(value: UserRoleResponseProto) -> Self {
        UserRoleResponse {
            user_id: value.userid,
            role_id: value.roleid,
        }
    }
}
