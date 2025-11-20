use crate::{
    abstract_trait::ResetTokenQueryRepositoryTrait, config::ConnectionPool,
    errors::RepositoryError, model::ResetToken as ResetTokenModel,
};
use async_trait::async_trait;
use tracing::{error, info};

pub struct ResetTokenQueryRepository {
    db: ConnectionPool,
}

impl ResetTokenQueryRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ResetTokenQueryRepositoryTrait for ResetTokenQueryRepository {
    async fn find_by_token(&self, token: &str) -> Result<Option<ResetTokenModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        match sqlx::query_as!(
            ResetTokenModel,
            r#"
            SELECT reset_token_id, user_id, token, expired_date
            FROM reset_tokens
            WHERE token = $1
            "#,
            token
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(model)) => {
                info!("✅ Found reset token for token={}", token);
                Ok(Some(model))
            }
            Ok(None) => {
                info!("🔍 No reset token found for token={}", token);
                Ok(None)
            }
            Err(e) => {
                error!("❌ Query failed for reset token lookup: {:?}", e);
                Err(RepositoryError::from(e))
            }
        }
    }
}
