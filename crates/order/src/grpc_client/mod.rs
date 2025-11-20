pub mod product;

use crate::config::grpc_config::GrpcClientConfig;
use anyhow::{Context, Result};
use genproto::product::product_query_service_client::ProductQueryServiceClient;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};

#[derive(Clone)]
pub struct GrpcClients {
    pub product_query_client: Arc<Mutex<ProductQueryServiceClient<Channel>>>,
}

impl GrpcClients {
    pub async fn init(config: GrpcClientConfig) -> Result<Self> {
        let product_channel = Self::connect(config.product, "product-service").await?;

        Ok(Self {
            product_query_client: Arc::new(Mutex::new(ProductQueryServiceClient::new(
                product_channel.clone(),
            ))),
        })
    }

    async fn connect(addr: String, service: &str) -> Result<Channel> {
        let endpoint = Endpoint::from_shared(addr.clone())
            .with_context(|| format!("Invalid gRPC address for {service}: {addr}"))?;

        endpoint
            .connect()
            .await
            .with_context(|| format!("Failed to connect to {service} at {addr}"))
    }
}
