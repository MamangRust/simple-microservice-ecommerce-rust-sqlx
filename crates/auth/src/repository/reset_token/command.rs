use crate::{
    abstract_trait::reset_token::ResetTokenCommandRepositoryTrait,
    domain::requests::reset_token::CreateResetTokenRequest,
    models::reset_token::ResetToken as ResetTokenModel,
};
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError, utils::parse_expiration_datetime};
use tracing::{error, info};

pub struct ResetTokenCommandRepository {
    db: ConnectionPool,
}

impl ResetTokenCommandRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ResetTokenCommandRepositoryTrait for ResetTokenCommandRepository {
    async fn create_reset_token(
        &self,
        request: &CreateResetTokenRequest,
    ) -> Result<ResetTokenModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let expired_at = match parse_expiration_datetime(&request.expired_at) {
            Ok(datetime) => datetime,
            Err(e) => {
                error!("❌ Failed to parse expiration datetime: {}", e);
                return Err(RepositoryError::Custom("Invalid datetime format".into()));
            }
        };

        let result = sqlx::query_as!(
            ResetTokenModel,
            r#"
            INSERT INTO reset_tokens (user_id, token, expiry_date)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, token, expiry_date
            "#,
            request.user_id,
            request.reset_token,
            expired_at
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Error inserting reset token for user_id {}: {:?}",
                request.user_id, err
            );
            RepositoryError::from(err)
        })?;

        info!(
            "✅ Created reset token for user_id {} with token {}",
            result.user_id, result.token
        );

        Ok(result)
    }

    async fn delete_reset_token(&self, user_id: i32) -> Result<(), RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let result = sqlx::query!(
            r#"
            DELETE FROM reset_tokens
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Error deleting reset token for user_id {}: {:?}",
                user_id, err
            );
            RepositoryError::from(err)
        })?;

        if result.rows_affected() == 0 {
            error!("⚠️ No reset token found for user_id {}", user_id);
            return Err(RepositoryError::NotFound);
        }

        info!("🗑️ Deleted reset token for user_id {}", user_id);

        Ok(())
    }
}
