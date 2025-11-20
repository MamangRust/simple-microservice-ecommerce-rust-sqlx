use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ResetToken {
    pub reset_token_id: i32,
    pub user_id: i32,
    pub token: String,
    pub expired_date: NaiveDateTime,
}
