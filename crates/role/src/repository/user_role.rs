use async_trait::async_trait;
use tracing::{error, info};

use crate::{
    abstract_trait::user_role::repository::UserRoleCommandRepositoryTrait,
    domain::requests::user_role::{CreateUserRoleRequest, RemoveUserRoleRequest},
    model::user_role::UserRole as UserRoleModel,
};
use shared::{config::ConnectionPool, errors::RepositoryError};

pub struct UserRoleRepository {
    db_pool: ConnectionPool,
}

impl UserRoleRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl UserRoleCommandRepositoryTrait for UserRoleRepository {
    async fn assign_role_to_user(
        &self,
        create_user_role_request: &CreateUserRoleRequest,
    ) -> Result<UserRoleModel, RepositoryError> {
        let mut conn = self.db_pool.acquire().await.map_err(|e| {
            error!("Failed to acquire DB connection: {}", e);
            RepositoryError::from(e)
        })?;

        match sqlx::query_as!(
            UserRoleModel,
            r#"
            INSERT INTO user_roles (user_id, role_id, created_at, updated_at)
            VALUES ($1, $2, current_timestamp, current_timestamp)
            RETURNING user_role_id, user_id, role_id, created_at, updated_at, deleted_at
            "#,
            create_user_role_request.user_id,
            create_user_role_request.role_id
        )
        .fetch_one(&mut *conn)
        .await
        {
            Ok(row) => {
                info!(
                    "Assigned role_id={} to user_id={} (user_role_id={})",
                    row.role_id, row.user_id, row.user_role_id
                );
                Ok(row)
            }
            Err(e) => {
                error!(
                    "Failed to assign role_id={} to user_id={}: {}",
                    create_user_role_request.role_id, create_user_role_request.user_id, e
                );
                Err(RepositoryError::from(e))
            }
        }
    }

    async fn update_role_to_user(
        &self,
        req: &CreateUserRoleRequest,
    ) -> Result<UserRoleModel, RepositoryError> {
        let mut conn = self.db_pool.acquire().await.map_err(|e| {
            error!("Failed to acquire DB connection: {}", e);
            RepositoryError::from(e)
        })?;

        match sqlx::query_as!(
            UserRoleModel,
            r#"
        UPDATE user_roles
        SET
            role_id = $2,
            updated_at = current_timestamp
        WHERE
            user_id = $1
            AND deleted_at IS NULL
        RETURNING
            user_role_id,
            user_id,
            role_id,
            created_at,
            updated_at,
            deleted_at
        "#,
            req.user_id,
            req.role_id
        )
        .fetch_one(&mut *conn)
        .await
        {
            Ok(row) => {
                info!(
                    "Updated role to role_id={} for user_id={} (user_role_id={})",
                    row.role_id, row.user_id, row.user_role_id
                );
                Ok(row)
            }
            Err(e) => {
                error!(
                    "Failed to update role_id={} for user_id={}: {}",
                    req.role_id, req.user_id, e
                );
                Err(RepositoryError::from(e))
            }
        }
    }

    async fn remove_role_from_user(
        &self,
        remove_user_role_request: &RemoveUserRoleRequest,
    ) -> Result<(), RepositoryError> {
        let mut conn = self.db_pool.acquire().await.map_err(|e| {
            error!("Failed to acquire DB connection: {}", e);
            RepositoryError::from(e)
        })?;

        let result = sqlx::query!(
            r#"
            DELETE FROM user_roles
            WHERE user_id = $1 AND role_id = $2
            "#,
            remove_user_role_request.user_id,
            remove_user_role_request.role_id
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!(
                "Failed to remove role_id={} from user_id={}: {}",
                remove_user_role_request.role_id, remove_user_role_request.user_id, e
            );
            RepositoryError::from(e)
        })?;

        if result.rows_affected() == 0 {
            error!(
                "No user_role found to remove for user_id={} and role_id={}",
                remove_user_role_request.user_id, remove_user_role_request.role_id
            );
            return Err(RepositoryError::NotFound);
        }

        info!(
            "Removed role_id={} from user_id={} (rows affected: {})",
            remove_user_role_request.role_id,
            remove_user_role_request.user_id,
            result.rows_affected()
        );

        Ok(())
    }
}
