use crate::{
    abstract_trait::OrderQueryRepositoryTrait, config::ConnectionPool,
    domain::requests::FindAllOrders, errors::RepositoryError, model::Order as OrderModel,
};
use async_trait::async_trait;
use tracing::{error, info};

#[derive(Clone)]
pub struct OrderQueryRepository {
    db: ConnectionPool,
}

impl OrderQueryRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrderQueryRepositoryTrait for OrderQueryRepository {
    async fn find_all(
        &self,
        req: &FindAllOrders,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError> {
        info!("🔍 Fetching all orders with search: {:?}", req.search);

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
                o.order_id,
                o.product_id,
                o.quantity,
                o.total,
                o.created_at,
                o.updated_at,
                o.deleted_at,
                COUNT(*) OVER() AS total_count
            FROM orders o
            WHERE ($1::TEXT IS NULL OR o.product_id::TEXT ILIKE '%' || $1 || '%')
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch orders: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let orders = rows
            .into_iter()
            .map(|r| OrderModel {
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                total: r.total,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((orders, total))
    }

    async fn find_active(
        &self,
        req: &FindAllOrders,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError> {
        info!("🟢 Fetching active orders with search: {:?}", req.search);

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
                o.order_id,
                o.product_id,
                o.quantity,
                o.total,
                o.created_at,
                o.updated_at,
                o.deleted_at,
                COUNT(*) OVER() AS total_count
            FROM orders o
            WHERE o.deleted_at IS NULL
              AND ($1::TEXT IS NULL OR o.product_id::TEXT ILIKE '%' || $1 || '%')
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Error fetching active orders: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let orders = rows
            .into_iter()
            .map(|r| OrderModel {
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                total: r.total,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((orders, total))
    }

    async fn find_trashed(
        &self,
        req: &FindAllOrders,
    ) -> Result<(Vec<OrderModel>, i64), RepositoryError> {
        info!("🗑️ Fetching trashed orders with search: {:?}", req.search);

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
                o.order_id,
                o.product_id,
                o.quantity,
                o.total,
                o.created_at,
                o.updated_at,
                o.deleted_at,
                COUNT(*) OVER() AS total_count
            FROM orders o
            WHERE o.deleted_at IS NOT NULL
              AND ($1::TEXT IS NULL OR o.product_id::TEXT ILIKE '%' || $1 || '%')
            ORDER BY o.deleted_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch trashed orders: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let orders = rows
            .into_iter()
            .map(|r| OrderModel {
                order_id: r.order_id,
                product_id: r.product_id,
                quantity: r.quantity,
                total: r.total,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((orders, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<OrderModel>, RepositoryError> {
        info!("🆔 Fetching order by ID: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            OrderModel,
            r#"
            SELECT
                order_id,
                product_id,
                quantity,
                total,
                created_at,
                updated_at,
                deleted_at
            FROM orders
            WHERE order_id = $1
            "#,
            id
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }
}
