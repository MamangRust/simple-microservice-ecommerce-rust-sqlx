use crate::{
    abstract_trait::order_item::repository::OrderItemCommandRepositoryTrait,
    domain::requests::order_item::{CreateOrderItemRecordRequest, UpdateOrderItemRecordRequest},
    model::order_item::OrderItem as OrderItemModel,
};
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError};
use tracing::{error, info};

pub struct OrderItemCommandRepository {
    db: ConnectionPool,
}

impl OrderItemCommandRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrderItemCommandRepositoryTrait for OrderItemCommandRepository {
    async fn create_order_item(
        &self,
        req: &CreateOrderItemRecordRequest,
    ) -> Result<OrderItemModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderItemModel,
            r#"
            INSERT INTO order_items (order_id, product_id, quantity, price, created_at, updated_at)
            VALUES ($1, $2, $3, $4, current_timestamp, current_timestamp)
            RETURNING order_item_id, order_id, product_id, quantity, price, created_at, updated_at, deleted_at
            "#,
            req.order_id,
            req.product_id,
            req.quantity,
            req.price
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to create order item for order {}: {:?}",
                req.order_id, err
            );
            RepositoryError::from(err)
        })?;

        info!(
            "✅ Created order item {} for order {}",
            result.order_item_id, result.order_id
        );
        Ok(result)
    }

    async fn update_order_item(
        &self,
        req: &UpdateOrderItemRecordRequest,
    ) -> Result<OrderItemModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderItemModel,
            r#"
            UPDATE order_items
            SET order_id   = $2,
                product_id = $3,
                quantity   = $4,
                price      = $5,
                updated_at = current_timestamp
            WHERE order_item_id = $1
            RETURNING order_item_id, order_id, product_id, quantity, price, created_at, updated_at, deleted_at
            "#,
            req.order_item_id,
            req.order_id,
            req.product_id,
            req.quantity,
            req.price,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to update order item {}: {:?}",
                req.order_item_id, err
            );
            RepositoryError::from(err)
        })?;

        info!("🔄 Updated order item {}", result.order_item_id);
        Ok(result)
    }

    async fn trashed_order_item(
        &self,
        order_item_id: i32,
    ) -> Result<OrderItemModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderItemModel,
            r#"
            UPDATE order_items
            SET deleted_at = current_timestamp,
                updated_at = current_timestamp
            WHERE order_item_id = $1
            RETURNING order_item_id, order_id, product_id, quantity, price, created_at, updated_at, deleted_at
            "#,
            order_item_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!("❌ Failed to soft delete order item {}: {:?}", order_item_id, err);
            RepositoryError::from(err)
        })?;

        info!("🗑️ Soft deleted order item {}", order_item_id);
        Ok(result)
    }

    async fn restore_order_item(
        &self,
        order_item_id: i32,
    ) -> Result<OrderItemModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderItemModel,
            r#"
            UPDATE order_items
            SET deleted_at = NULL,
                updated_at = current_timestamp
            WHERE order_item_id = $1
            RETURNING order_item_id, order_id, product_id, quantity, price, created_at, updated_at, deleted_at
            "#,
            order_item_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!("❌ Failed to restore order item {}: {:?}", order_item_id, err);
            RepositoryError::from(err)
        })?;

        info!("♻️ Restored order item {}", order_item_id);
        Ok(result)
    }

    async fn delete_order_item_permanent(
        &self,
        order_item_id: i32,
    ) -> Result<bool, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM order_items WHERE order_item_id = $1
            "#,
            order_item_id
        )
        .execute(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to permanently delete order item {}: {:?}",
                order_item_id, err
            );
            RepositoryError::from(err)
        })?;

        info!("🔥 Permanently deleted order item {}", order_item_id);
        Ok(true)
    }

    async fn restore_all_order_item(&self) -> Result<bool, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            UPDATE order_items
            SET deleted_at = NULL,
                updated_at = current_timestamp
            WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(|err| {
            error!("❌ Failed to restore all order items: {:?}", err);
            RepositoryError::from(err)
        })?;

        info!("♻️ Restored all soft-deleted order items");
        Ok(true)
    }

    async fn delete_all_order_item_permanent(&self) -> Result<bool, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM order_items
            WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to delete all trashed order items permanently: {:?}",
                err
            );
            RepositoryError::from(err)
        })?;

        info!("🔥 Permanently deleted all trashed order items");
        Ok(true)
    }
}
