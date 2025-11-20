use crate::{
    abstract_trait::order_item::repository::OrderItemQueryRepositoryTrait,
    domain::requests::order_item::FindAllOrderItems,
    model::order_item::OrderItem as OrderItemModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError};
use tracing::{error, info};

#[derive(Clone)]
pub struct OrderItemQueryRepository {
    db: ConnectionPool,
}

impl OrderItemQueryRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrderItemQueryRepositoryTrait for OrderItemQueryRepository {
    async fn find_order_item_by_order(
        &self,
        order_id: i32,
    ) -> Result<Vec<OrderItemModel>, RepositoryError> {
        info!("📦 Fetching order items for order_id: {}", order_id);

        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let rows = sqlx::query!(
            r#"
        SELECT
            order_item_id,
            order_id,
            product_id,
            quantity,
            price,
            created_at,
            updated_at,
            deleted_at
        FROM order_items
        WHERE order_id = $1
        ORDER BY created_at DESC
        "#,
            order_id
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!(
                "❌ Failed to fetch order items for order_id {}: {:?}",
                order_id, e
            );
            RepositoryError::from(e)
        })?;

        let order_items = rows
            .into_iter()
            .map(|r| OrderItemModel {
                order_item_id: r.order_item_id,
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                price: r.price,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok(order_items)
    }

    async fn calculate_total_price(&self, order_id: i32) -> Result<i32, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let row = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(quantity * price), 0)::int AS total_price
            FROM order_items
            WHERE order_id = $1
              AND deleted_at IS NULL
            "#,
            order_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(row.total_price.unwrap_or(0))
    }

    async fn find_all_order_items(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<(Vec<OrderItemModel>, i64), RepositoryError> {
        info!("📦 Fetching order items with search {:?}", req.search);

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
            order_item_id,
            order_id,
            product_id,
            quantity,
            price,
            created_at,
            updated_at,
            deleted_at,
            COUNT(*) OVER() AS total_count
        FROM order_items
        WHERE ($1::TEXT IS NULL OR product_id::TEXT ILIKE '%' || $1 || '%')
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch order items: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let order_items = rows
            .into_iter()
            .map(|r| OrderItemModel {
                order_item_id: r.order_item_id,
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                price: r.price,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((order_items, total))
    }

    async fn find_by_active(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<(Vec<OrderItemModel>, i64), RepositoryError> {
        info!("📦 Fetching active order items");

        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let limit = req.page_size as i64;
        let offset = ((req.page - 1).max(0) * req.page_size) as i64;

        let rows = sqlx::query!(
            r#"
        SELECT
            order_item_id,
            order_id,
            product_id,
            quantity,
            price,
            created_at,
            updated_at,
            deleted_at,
            COUNT(*) OVER() AS total_count
        FROM order_items
        WHERE deleted_at IS NULL
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch active order items: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let order_items = rows
            .into_iter()
            .map(|r| OrderItemModel {
                order_item_id: r.order_item_id,
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                price: r.price,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((order_items, total))
    }

    async fn find_by_trashed(
        &self,
        req: &FindAllOrderItems,
    ) -> Result<(Vec<OrderItemModel>, i64), RepositoryError> {
        info!("📦 Fetching trashed order items");

        let mut conn = self.db.acquire().await.map_err(|e| {
            error!("❌ Failed to acquire DB connection: {:?}", e);
            RepositoryError::from(e)
        })?;

        let limit = req.page_size as i64;
        let offset = ((req.page - 1).max(0) * req.page_size) as i64;

        let rows = sqlx::query!(
            r#"
        SELECT
            order_item_id,
            order_id,
            product_id,
            quantity,
            price,
            created_at,
            updated_at,
            deleted_at,
            COUNT(*) OVER() AS total_count
        FROM order_items
        WHERE deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        LIMIT $1 OFFSET $2
        "#,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch trashed order items: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let order_items = rows
            .into_iter()
            .map(|r| OrderItemModel {
                order_item_id: r.order_item_id,
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                price: r.price,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((order_items, total))
    }
}
