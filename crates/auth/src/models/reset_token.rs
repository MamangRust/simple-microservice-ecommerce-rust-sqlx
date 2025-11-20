use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ResetToken {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub expiry_date: NaiveDateTime,
}
