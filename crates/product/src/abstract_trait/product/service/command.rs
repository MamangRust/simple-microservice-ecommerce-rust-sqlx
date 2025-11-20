use crate::domain::{
    requests::product::{CreateProductRequest, UpdateProductRequest},
    response::{
        api::ApiResponse,
        product::{ProductResponse, ProductResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::ServiceError;
use std::sync::Arc;

pub type DynProductCommandService = Arc<dyn ProductCommandServiceTrait + Send + Sync>;

#[async_trait]
pub trait ProductCommandServiceTrait {
    async fn create_product(
        &self,
        req: &CreateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError>;
    async fn update_product(
        &self,
        req: &UpdateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError>;
    async fn increasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError>;
    async fn decreasing_stock(
        &self,
        product_id: i32,
        qty: i32,
    ) -> Result<ApiResponse<ProductResponse>, ServiceError>;
    async fn trash_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, ServiceError>;
    async fn restore_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, ServiceError>;
    async fn delete_product(&self, id: i32) -> Result<ApiResponse<()>, ServiceError>;
    async fn restore_all_product(&self) -> Result<ApiResponse<()>, ServiceError>;
    async fn delete_all_product(&self) -> Result<ApiResponse<()>, ServiceError>;
}
