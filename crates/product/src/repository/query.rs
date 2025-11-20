use crate::{
    abstract_trait::product::repository::ProductQueryRepositoryTrait,
    domain::requests::product::FindAllProducts, model::product::Product as ProductModel,
};
use async_trait::async_trait;
use shared::{config::ConnectionPool, errors::RepositoryError};
use tracing::{error, info};

#[derive(Clone)]
pub struct ProductQueryRepository {
    db: ConnectionPool,
}

impl ProductQueryRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProductQueryRepositoryTrait for ProductQueryRepository {
    async fn find_all(
        &self,
        req: &FindAllProducts,
    ) -> Result<(Vec<ProductModel>, i64), RepositoryError> {
        info!("🔍 Fetching all products with search: {:?}", req.search);

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
                p.product_id,
                p.name,
                p.price,
                p.stock,
                p.created_at,
                p.updated_at,
                p.deleted_at,
                COUNT(*) OVER() AS total_count
            FROM products p
            WHERE ($1::TEXT IS NULL OR p.name ILIKE '%' || $1 || '%')
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch products: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let products = rows
            .into_iter()
            .map(|r| ProductModel {
                product_id: r.product_id,
                name: r.name,
                price: r.price,
                stock: r.stock,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((products, total))
    }

    async fn find_active(
        &self,
        req: &FindAllProducts,
    ) -> Result<(Vec<ProductModel>, i64), RepositoryError> {
        info!("🟢 Fetching active products with search: {:?}", req.search);

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
                p.product_id,
                p.name,
                p.price,
                p.stock,
                p.created_at,
                p.updated_at,
                p.deleted_at,
                COUNT(*) OVER() AS total_count
            FROM products p
            WHERE p.deleted_at IS NULL
              AND ($1::TEXT IS NULL OR p.name ILIKE '%' || $1 || '%')
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Error fetching active products: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let products = rows
            .into_iter()
            .map(|r| ProductModel {
                product_id: r.product_id,
                name: r.name,
                price: r.price,
                stock: r.stock,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((products, total))
    }

    async fn find_trashed(
        &self,
        req: &FindAllProducts,
    ) -> Result<(Vec<ProductModel>, i64), RepositoryError> {
        info!("🗑️ Fetching trashed products with search: {:?}", req.search);

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
                p.product_id,
                p.name,
                p.price,
                p.stock,
                p.created_at,
                p.updated_at,
                p.deleted_at,
                COUNT(*) OVER() AS total_count
            FROM products p
            WHERE p.deleted_at IS NOT NULL
              AND ($1::TEXT IS NULL OR p.name ILIKE '%' || $1 || '%')
            ORDER BY p.deleted_at DESC
            LIMIT $2 OFFSET $3
            "#,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch trashed products: {:?}", e);
            RepositoryError::from(e)
        })?;

        let total = rows
            .first()
            .map(|r| r.total_count.unwrap_or(0))
            .unwrap_or(0);

        let products = rows
            .into_iter()
            .map(|r| ProductModel {
                product_id: r.product_id,
                name: r.name,
                price: r.price,
                stock: r.stock,
                created_at: r.created_at,
                updated_at: r.updated_at,
                deleted_at: r.deleted_at,
            })
            .collect();

        Ok((products, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<ProductModel>, RepositoryError> {
        info!("🆔 Fetching product by ID: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            ProductModel,
            r#"
            SELECT
                product_id,
                name,
                price,
                stock,
                created_at,
                updated_at,
                deleted_at
            FROM products
            WHERE product_id = $1
            "#,
            id
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(RepositoryError::from)?;

        Ok(result)
    }
}
