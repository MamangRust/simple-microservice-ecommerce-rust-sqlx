use crate::{
    abstract_trait::OrderCommandRepositoryTrait,
    config::ConnectionPool,
    domain::requests::{CreateOrderRequest, UpdateOrderRequest},
    errors::RepositoryError,
    model::Order as OrderModel,
};
use async_trait::async_trait;
use tracing::{error, info};

pub struct OrderCommandRepository {
    db: ConnectionPool,
}

impl OrderCommandRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrderCommandRepositoryTrait for OrderCommandRepository {
    async fn create_order(
        &self,
        order: &CreateOrderRequest,
        total: i64,
    ) -> Result<OrderModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderModel,
            r#"
            INSERT INTO orders (product_id, quantity, total, created_at, updated_at)
            VALUES ($1, $2, $3, current_timestamp, current_timestamp)
            RETURNING order_id, product_id, quantity, total, created_at, updated_at, deleted_at
            "#,
            order.product_id,
            order.quantity,
            total,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to create order for product {}: {:?}",
                order.product_id, err
            );
            RepositoryError::from(err)
        })?;

        info!(
            "✅ Created order ID {} for product {}",
            result.order_id, result.product_id
        );
        Ok(result)
    }

    async fn update_order(
        &self,
        order: &UpdateOrderRequest,
        total: i64,
    ) -> Result<OrderModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderModel,
            r#"
            UPDATE orders
            SET product_id = $2,
                quantity   = $3,
                total = $4,
                updated_at = current_timestamp
            WHERE order_id = $1
            RETURNING order_id, product_id, quantity, total, created_at, updated_at, deleted_at
            "#,
            order.id,
            order.product_id,
            order.quantity,
            total
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!("❌ Failed to update order ID {}: {:?}", order.id, err);
            RepositoryError::from(err)
        })?;

        info!("🔄 Updated order ID {}", result.order_id);
        Ok(result)
    }

    async fn trash_order(&self, id: i32) -> Result<OrderModel, RepositoryError> {
        info!("🗑️ Trashing order: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let order = sqlx::query_as!(
            OrderModel,
            r#"
            UPDATE orders
            SET deleted_at = CURRENT_TIMESTAMP
            WHERE order_id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to trash order {}: {:?}", id, e);
            RepositoryError::from(e)
        })?;

        Ok(order)
    }

    async fn restore_order(&self, id: i32) -> Result<OrderModel, RepositoryError> {
        info!("🔄 Restoring order: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let order = sqlx::query_as!(
            OrderModel,
            r#"
            UPDATE orders
            SET deleted_at = NULL
            WHERE order_id = $1 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to restore order {}: {:?}", id, e);
            RepositoryError::from(e)
        })?;

        Ok(order)
    }

    async fn delete_order(&self, id: i32) -> Result<(), RepositoryError> {
        info!("❌ Hard deleting order: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM orders
            WHERE order_id = $1 AND deleted_at IS NOT NULL
            "#,
            id
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to delete order {}: {:?}", id, e);
            RepositoryError::from(e)
        })?;

        Ok(())
    }

    async fn restore_all_orders(&self) -> Result<(), RepositoryError> {
        info!("🔄 Restoring all trashed orders");

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            UPDATE orders SET deleted_at = NULL WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to restore all orders: {:?}", e);
            RepositoryError::from(e)
        })?;

        Ok(())
    }

    async fn delete_all_orders(&self) -> Result<(), RepositoryError> {
        info!("❌ Hard deleting all trashed orders");

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM orders WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to delete all trashed orders: {:?}", e);
            RepositoryError::from(e)
        })?;

        Ok(())
    }
}
