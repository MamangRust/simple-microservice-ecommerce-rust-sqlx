use crate::{
    abstract_trait::refresh_token::RefreshTokenQueryRepositoryTrait,
    models::refresh_token::RefreshToken as RefreshTokenModel,
};
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError};

pub struct RefreshTokenQueryRepository {
    db: ConnectionPool,
}

impl RefreshTokenQueryRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RefreshTokenQueryRepositoryTrait for RefreshTokenQueryRepository {
    async fn find_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<Option<RefreshTokenModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            RefreshTokenModel,
            r#"
            SELECT refresh_token_id, user_id, token, expiration, created_at, updated_at, deleted_at
            FROM refresh_tokens
            WHERE user_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }

    async fn find_by_token(
        &self,
        token: String,
    ) -> Result<Option<RefreshTokenModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            RefreshTokenModel,
            r#"
            SELECT refresh_token_id, user_id, token, expiration, created_at, updated_at, deleted_at
            FROM refresh_tokens
            WHERE token = $1 AND deleted_at IS NULL
            "#,
            token
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }
}
