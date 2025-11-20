use crate::domain::{
    requests::product::{CreateProductRequest, FindAllProducts, UpdateProductRequest},
    response::{
        api::{ApiResponse, ApiResponsePagination},
        product::{ProductResponse, ProductResponseDeleteAt},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use shared::errors::HttpError;
use std::sync::Arc;

pub type DynProductGrpcClient = Arc<dyn ProductGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait ProductGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponse>>, HttpError>;
    async fn find_active(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, HttpError>;
    async fn find_trashed(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, HttpError>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, HttpError>;
    async fn create_product(
        &self,
        req: &CreateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, HttpError>;
    async fn update_product(
        &self,
        req: &UpdateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, HttpError>;
    async fn trash_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, HttpError>;
    async fn restore_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, HttpError>;
    async fn delete_product(&self, id: i32) -> Result<ApiResponse<()>, HttpError>;
    async fn restore_all_product(&self) -> Result<ApiResponse<()>, HttpError>;
    async fn delete_all_product(&self) -> Result<ApiResponse<()>, HttpError>;
}
