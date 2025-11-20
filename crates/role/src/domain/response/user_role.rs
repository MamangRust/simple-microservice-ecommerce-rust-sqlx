use crate::model::user_role::UserRole as UserRoleModel;
use genproto::user_role::UserRoleResponse as UserRoleResponseProto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRoleResponse {
    #[serde(rename = "user_id")]
    pub user_id: i32,

    #[serde(rename = "role_id")]
    pub role_id: i32,
}

impl From<UserRoleModel> for UserRoleResponse {
    fn from(value: UserRoleModel) -> Self {
        UserRoleResponse {
            user_id: value.user_id,
            role_id: value.role_id,
        }
    }
}

impl From<UserRoleResponseProto> for UserRoleResponse {
    fn from(value: UserRoleResponseProto) -> Self {
        UserRoleResponse {
            user_id: value.userid,
            role_id: value.roleid,
        }
    }
}

impl From<UserRoleResponse> for UserRoleResponseProto {
    fn from(value: UserRoleResponse) -> Self {
        UserRoleResponseProto {
            userid: value.user_id,
            roleid: value.role_id,
        }
    }
}
