use crate::{
    abstract_trait::user::repository::UserCommandRepositoryTrait,
    domain::requests::user::{
        CreateUserRequest, UpdateUserPasswordRequest, UpdateUserRequest, UpdateUserVerifiedRequest,
    },
    model::user::User as UserModel,
};
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError};

pub struct UserCommandRepository {
    db: ConnectionPool,
}

impl UserCommandRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserCommandRepositoryTrait for UserCommandRepository {
    async fn create_user(&self, req: &CreateUserRequest) -> Result<UserModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let user = sqlx::query_as!(
            UserModel,
            r#"
        INSERT INTO users (
            firstname,
            lastname,
            email,
            password,
            verification_code,
            is_verified,
            created_at,
            updated_at
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        RETURNING
            user_id,
            firstname,
            lastname,
            email,
            password,
            verification_code,
            is_verified,
            created_at,
            updated_at,
            deleted_at
        "#,
            req.first_name,
            req.last_name,
            req.email,
            req.password,
            req.verified_code,
            req.is_verified
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(user)
    }

    async fn update_user(&self, req: &UpdateUserRequest) -> Result<UserModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let user = sqlx::query_as!(
            UserModel,
            r#"
            UPDATE users
            SET firstname = $2,
                lastname = $3,
                email = $4,
                password = $5,
                updated_at = current_timestamp
            WHERE user_id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
            req.user_id,
            req.first_name,
            req.last_name,
            req.email,
            req.password
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(user)
    }

    async fn update_isverifed(
        &self,
        req: &UpdateUserVerifiedRequest,
    ) -> Result<UserModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let user = sqlx::query_as!(
            UserModel,
            r#"
            UPDATE users
            SET is_verified = $2,
                updated_at = current_timestamp
            WHERE user_id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
            req.user_id,
            req.is_verified
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(user)
    }

    async fn update_password(
        &self,
        req: &UpdateUserPasswordRequest,
    ) -> Result<UserModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let user = sqlx::query_as!(
            UserModel,
            r#"
            UPDATE users
            SET password = $2,
                updated_at = current_timestamp
            WHERE user_id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
            req.user_id,
            req.password
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(user)
    }

    async fn trash_user(&self, id: i32) -> Result<UserModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let user = sqlx::query_as!(
            UserModel,
            r#"
            UPDATE users
            SET deleted_at = current_timestamp
            WHERE user_id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(user)
    }

    async fn restore_user(&self, id: i32) -> Result<UserModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let user = sqlx::query_as!(
            UserModel,
            r#"
            UPDATE users
            SET deleted_at = NULL
            WHERE user_id = $1 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(user)
    }

    async fn delete_user(&self, id: i32) -> Result<(), RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM users WHERE user_id = $1 AND deleted_at IS NOT NULL
            "#,
            id
        )
        .execute(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(())
    }

    async fn restore_all_user(&self) -> Result<(), RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            UPDATE users SET deleted_at = NULL WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(())
    }

    async fn delete_all_user(&self) -> Result<(), RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM users WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(())
    }
}
