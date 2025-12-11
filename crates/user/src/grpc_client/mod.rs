use crate::config::grpc_config::GrpcClientConfig;
use anyhow::{Context, Result};
use genproto::{
    role::role_query_service_client::RoleQueryServiceClient,
    user_role::user_role_service_client::UserRoleServiceClient,
};
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

pub mod role;
pub mod user_role;

#[derive(Clone)]
pub struct GrpcClients {
    pub role_client: RoleQueryServiceClient<Channel>,
    pub user_role_client: UserRoleServiceClient<Channel>,
}

impl GrpcClients {
    pub async fn init(config: GrpcClientConfig) -> Result<Self> {
        let role_channel = Self::connect(config.role, "role-service").await?;

        Ok(Self {
            role_client: RoleQueryServiceClient::new(
                role_channel.clone(),
            ),
            user_role_client: UserRoleServiceClient::new(
                role_channel.clone(),
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
