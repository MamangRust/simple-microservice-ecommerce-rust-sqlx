use crate::{
    abstract_trait::grpc_client::ProductGrpcClientTrait,
    domain::response::{api::ApiResponse, product::ProductResponse},
};
use anyhow::Result;
use async_trait::async_trait;
use genproto::product::{
    FindByIdProductRequest,
    product_query_service_client::ProductQueryServiceClient as ProductQueryServiceGrpcClient,
};
use shared::errors::AppErrorGrpc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, transport::Channel};

pub struct ProductGrpcClientService {
    client: Arc<Mutex<ProductQueryServiceGrpcClient<Channel>>>,
}

impl ProductGrpcClientService {
    pub async fn new(client: Arc<Mutex<ProductQueryServiceGrpcClient<Channel>>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ProductGrpcClientTrait for ProductGrpcClientService {
    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<ProductResponse>, AppErrorGrpc> {
        let req = Request::new(FindByIdProductRequest { id });

        let mut client = self.client.lock().await;

        let response = client.find_by_id(req).await.map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("Product Role data is missing in gRPC response".into())
        })?;

        let domain_user: ProductResponse = user_data.into();

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "OK".to_string(),
            data: domain_user,
        })
    }
}
