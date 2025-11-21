use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
}
