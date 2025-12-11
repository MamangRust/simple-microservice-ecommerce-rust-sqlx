pub mod product;

use std::time::Duration;
use crate::config::grpc_config::GrpcClientConfig;
use anyhow::{Context, Result};
use genproto::product::product_query_service_client::ProductQueryServiceClient;
use tonic::transport::{Channel, Endpoint};

#[derive(Clone)]
pub struct GrpcClients {
    pub product_query_client: ProductQueryServiceClient<Channel>,
}

impl GrpcClients {
    pub async fn init(config: GrpcClientConfig) -> Result<Self> {
        let product_channel = Self::connect(config.product, "product-service").await?;

        Ok(Self {
            product_query_client: ProductQueryServiceClient::new(
                product_channel.clone(),
            ),
        })
    }

    async fn connect(addr: String, service: &str) -> Result<Channel> {
        let endpoint = Endpoint::from_shared(addr.clone())
            .with_context(|| format!("Invalid gRPC address for {service}: {addr}"))?;

        let configured_endpoint = endpoint
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_interval(Duration::from_secs(5))
            .initial_connection_window_size(1_048_576)
            .initial_stream_window_size(1_048_576);

        configured_endpoint
            .connect()
            .await
            .with_context(|| format!("Failed to connect to {service} at {addr}"))
    }
}
