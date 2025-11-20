use crate::{
    domain::{
        requests::{CreateProductRequest, FindAllProducts, UpdateProductRequest},
        responses::{ApiResponse, ApiResponsePagination, ProductResponse, ProductResponseDeleteAt},
    },
    errors::AppErrorHttp,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type DynProductGrpcClient = Arc<dyn ProductGrpcClientTrait + Send + Sync>;

#[async_trait]
pub trait ProductGrpcClientTrait {
    async fn find_all(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponse>>, AppErrorHttp>;
    async fn find_active(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, AppErrorHttp>;
    async fn find_trashed(
        &self,
        req: &FindAllProducts,
    ) -> Result<ApiResponsePagination<Vec<ProductResponseDeleteAt>>, AppErrorHttp>;
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, AppErrorHttp>;
    async fn create_product(
        &self,
        req: &CreateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, AppErrorHttp>;
    async fn update_product(
        &self,
        req: &UpdateProductRequest,
    ) -> Result<ApiResponse<ProductResponse>, AppErrorHttp>;
    async fn trash_product(
        &self,
        id: i32,
    ) -> Result<ApiResponse<ProductResponseDeleteAt>, AppErrorHttp>;
    async fn restore_product(&self, id: i32) -> Result<ApiResponse<ProductResponse>, AppErrorHttp>;
    async fn delete_product(&self, id: i32) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn restore_all_product(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
    async fn delete_all_product(&self) -> Result<ApiResponse<()>, AppErrorHttp>;
}
