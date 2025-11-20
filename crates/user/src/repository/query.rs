use crate::{
    abstract_trait::user::repository::UserQueryRepositoryTrait,
    domain::requests::user::FindAllUsers, model::user::User as UserModel,
};
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError};
use tracing::{error, info};

#[derive(Clone)]
pub struct UserQueryRepository {
    db: ConnectionPool,
}

impl UserQueryRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserQueryRepositoryTrait for UserQueryRepository {
    async fn find_all(&self, req: &FindAllUsers) -> Result<(Vec<UserModel>, i64), RepositoryError> {
        info!("🔍 Fetching all users with search: {:?}", req.search);

        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let limit = req.page_size as i64;
        let offset = ((req.page - 1).max(0) * req.page_size) as i64;

        let search_pattern = if req.search.trim().is_empty() {
            None
        } else {
            Some(req.search.as_str())
        };

        let rows = sqlx::query!(
            r#"
            SELECT
                user_id, firstname, lastname, email,
                password, verification_code, is_verified,
                created_at, updated_at, deleted_at,
                COUNT(*) OVER() AS total_count
            FROM users
            WHERE deleted_at IS NULL
            AND (
                $1::TEXT IS NULL OR
                firstname ILIKE '%' || $1 || '%' OR
                lastname ILIKE '%' || $1 || '%' OR
                email ILIKE '%' || $1 || '%'
            )
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch users: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let users = rows
            .into_iter()
            .map(|r| UserModel {
                user_id: r.user_id,
                firstname: r.firstname,
                lastname: r.lastname,
                email: r.email,
                password: r.password,
                verification_code: r.verification_code,
                is_verified: r.is_verified,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((users, total))
    }

    async fn find_active(
        &self,
        req: &FindAllUsers,
    ) -> Result<(Vec<UserModel>, i64), RepositoryError> {
        info!("🟢 Fetching active users with search: {:?}", req.search);

        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ DB connection failed: {:?}", e);
            RepositoryError::from(e)
        })?;

        let limit = req.page_size as i64;
        let offset = ((req.page - 1).max(0) * req.page_size) as i64;

        let search_pattern = if req.search.trim().is_empty() {
            None
        } else {
            Some(req.search.as_str())
        };

        let rows = sqlx::query!(
            r#"
            SELECT
                user_id, firstname, lastname, email,
                password, verification_code, is_verified,
                created_at, updated_at, deleted_at,
                COUNT(*) OVER() AS total_count
            FROM users
            WHERE deleted_at IS NULL
            AND (
                $1::TEXT IS NULL OR
                firstname ILIKE '%' || $1 || '%' OR
                lastname ILIKE '%' || $1 || '%' OR
                email ILIKE '%' || $1 || '%'
            )
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Error fetching active users: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let users = rows
            .into_iter()
            .map(|r| UserModel {
                user_id: r.user_id,
                firstname: r.firstname,
                lastname: r.lastname,
                email: r.email,
                password: r.password,
                verification_code: r.verification_code,
                is_verified: r.is_verified,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((users, total))
    }
    async fn find_trashed(
        &self,
        req: &FindAllUsers,
    ) -> Result<(Vec<UserModel>, i64), RepositoryError> {
        info!("🗑️ Fetching trashed users with search: {:?}", req.search);

        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let limit = req.page_size as i64;
        let offset = ((req.page - 1).max(0) * req.page_size) as i64;

        let search_pattern = if req.search.trim().is_empty() {
            None
        } else {
            Some(req.search.as_str())
        };

        let rows = sqlx::query!(
            r#"
            SELECT
                user_id, firstname, lastname, email,
                password, verification_code, is_verified,
                created_at, updated_at, deleted_at,
                COUNT(*) OVER() AS total_count
            FROM users
            WHERE deleted_at IS NOT NULL
              AND (
                $1::TEXT IS NULL OR
                firstname ILIKE '%' || $1 || '%' OR
                lastname ILIKE '%' || $1 || '%' OR
                email ILIKE '%' || $1 || '%'
              )
            ORDER BY deleted_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch trashed users: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let users = rows
            .into_iter()
            .map(|r| UserModel {
                user_id: r.user_id,
                firstname: r.firstname,
                lastname: r.lastname,
                email: r.email,
                password: r.password,
                verification_code: r.verification_code,
                is_verified: r.is_verified,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((users, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<UserModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            UserModel,
            r#"
            SELECT * FROM users WHERE user_id = $1 AND deleted_at IS NULL
            "#,
            id
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }

    async fn find_by_email(&self, email: String) -> Result<Option<UserModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            UserModel,
            r#"
            SELECT * FROM users WHERE email = $1 AND deleted_at IS NULL
            "#,
            email
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }

    async fn find_by_email_and_verify(
        &self,
        email: String,
    ) -> Result<Option<UserModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            UserModel,
            r#"
            SELECT * FROM users
            WHERE email = $1 AND deleted_at IS NULL AND is_verified = true
            "#,
            email
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }

    async fn find_verify_code(&self, code: String) -> Result<Option<UserModel>, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            UserModel,
            r#"
            SELECT * FROM users WHERE verification_code = $1
            "#,
            code
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }
}
