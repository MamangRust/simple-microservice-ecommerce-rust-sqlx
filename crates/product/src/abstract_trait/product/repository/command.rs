use crate::{
    domain::requests::product::{CreateProductRequest, UpdateProductRequest},
    model::product::Product as ProductModel,
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::RepositoryError;
use std::sync::Arc;

pub type DynProductCommandRepository = Arc<dyn ProductCommandRepositoryTrait + Send + Sync>;

#[async_trait]
pub trait ProductCommandRepositoryTrait {
    async fn create_product(
        &self,
        req: &CreateProductRequest,
    ) -> Result<ProductModel, RepositoryError>;
    async fn update_product(
        &self,
        req: &UpdateProductRequest,
    ) -> Result<ProductModel, RepositoryError>;
    async fn increasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ProductModel, RepositoryError>;
    async fn decreasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ProductModel, RepositoryError>;
    async fn trash_product(&self, id: i32) -> Result<ProductModel, RepositoryError>;
    async fn restore_product(&self, id: i32) -> Result<ProductModel, RepositoryError>;
    async fn delete_product(&self, id: i32) -> Result<(), RepositoryError>;
    async fn restore_all_products(&self) -> Result<(), RepositoryError>;
    async fn delete_all_products(&self) -> Result<(), RepositoryError>;
}
