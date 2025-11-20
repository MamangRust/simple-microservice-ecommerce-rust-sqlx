use crate::{
    abstract_trait::ProductCommandRepositoryTrait,
    config::ConnectionPool,
    domain::requests::{CreateProductRequest, UpdateProductRequest},
    errors::RepositoryError,
    model::Product as ProductModel,
};
use async_trait::async_trait;
use tracing::{error, info};

pub struct ProductCommandRepository {
    db: ConnectionPool,
}

impl ProductCommandRepository {
    pub fn new(db: ConnectionPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProductCommandRepositoryTrait for ProductCommandRepository {
    async fn create_product(
        &self,
        product: &CreateProductRequest,
    ) -> Result<ProductModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            ProductModel,
            r#"
            INSERT INTO products (name, price, stock, created_at, updated_at)
            VALUES ($1, $2, $3, current_timestamp, current_timestamp)
            RETURNING product_id, name, price, stock, created_at, updated_at, deleted_at
            "#,
            product.name,
            product.price,
            product.stock
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!("❌ Failed to create product {}: {:?}", product.name, err);
            RepositoryError::from(err)
        })?;

        info!(
            "✅ Created product ID {} ({})",
            result.product_id, result.name
        );
        Ok(result)
    }

    async fn update_product(
        &self,
        product: &UpdateProductRequest,
    ) -> Result<ProductModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            ProductModel,
            r#"
            UPDATE products
            SET name = $2,
                price = $3,
                stock = $4,
                updated_at = current_timestamp
            WHERE product_id = $1
            RETURNING product_id, name, price, stock, created_at, updated_at, deleted_at
            "#,
            product.id,
            product.name,
            product.price,
            product.stock
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!("❌ Failed to update product ID {}: {:?}", product.id, err);
            RepositoryError::from(err)
        })?;

        info!("🔄 Updated product ID {}", result.product_id);
        Ok(result)
    }

    async fn increasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ProductModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            ProductModel,
            r#"
        UPDATE products
        SET stock = stock + $1,
            updated_at = current_timestamp
        WHERE product_id = $2
        RETURNING product_id, name, price, stock, created_at, updated_at, deleted_at
        "#,
            qty,
            product_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to increase stock product {}: {:?}",
                product_id, err
            );
            RepositoryError::from(err)
        })?;

        info!(
            "✅ Increased stock product ID {} (new stock: {})",
            result.product_id, result.stock
        );
        Ok(result)
    }

    async fn decreasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ProductModel, RepositoryError> {
        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let result = sqlx::query_as!(
            ProductModel,
            r#"
        UPDATE products
        SET stock = stock - $1,
            updated_at = current_timestamp
        WHERE product_id = $2
        RETURNING product_id, name, price, stock, created_at, updated_at, deleted_at
        "#,
            qty,
            product_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| {
            error!(
                "❌ Failed to decrease stock product {}: {:?}",
                product_id, err
            );
            RepositoryError::from(err)
        })?;

        info!(
            "✅ Decreased stock product ID {} (new stock: {})",
            result.product_id, result.stock
        );
        Ok(result)
    }

    async fn trash_product(&self, id: i32) -> Result<ProductModel, RepositoryError> {
        info!("🗑️ Trashing product: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let product = sqlx::query_as!(
            ProductModel,
            r#"
            UPDATE products
            SET deleted_at = CURRENT_TIMESTAMP
            WHERE product_id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to trash product {}: {:?}", id, e);
            RepositoryError::from(e)
        })?;

        info!("✅ Product ID {} moved to trash", product.product_id);
        Ok(product)
    }

    async fn restore_product(&self, id: i32) -> Result<ProductModel, RepositoryError> {
        info!("🔄 Restoring product: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        let product = sqlx::query_as!(
            ProductModel,
            r#"
            UPDATE products
            SET deleted_at = NULL
            WHERE product_id = $1 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to restore product {}: {:?}", id, e);
            RepositoryError::from(e)
        })?;

        info!("✅ Product ID {} restored", product.product_id);
        Ok(product)
    }

    async fn delete_product(&self, id: i32) -> Result<(), RepositoryError> {
        info!("❌ Hard deleting product: {}", id);

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM products
            WHERE product_id = $1 AND deleted_at IS NOT NULL
            "#,
            id
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to hard-delete product {}: {:?}", id, e);
            RepositoryError::from(e)
        })?;

        info!("✅ Product ID {} permanently deleted", id);
        Ok(())
    }

    async fn restore_all_products(&self) -> Result<(), RepositoryError> {
        info!("🔄 Restoring all trashed products");

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            UPDATE products SET deleted_at = NULL WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to restore all products: {:?}", e);
            RepositoryError::from(e)
        })?;

        info!("✅ All products restored");
        Ok(())
    }

    async fn delete_all_products(&self) -> Result<(), RepositoryError> {
        info!("❌ Hard deleting all trashed products");

        let mut conn = self.db.acquire().await.map_err(RepositoryError::from)?;

        sqlx::query!(
            r#"
            DELETE FROM products WHERE deleted_at IS NOT NULL
            "#
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("❌ Failed to delete all trashed products: {:?}", e);
            RepositoryError::from(e)
        })?;

        info!("✅ All trashed products permanently deleted");
        Ok(())
    }
}
