use crate::config::grpc_config::GrpcClientConfig;
use anyhow::{Context, Result};
use genproto::{
    role::role_query_service_client::RoleQueryServiceClient,
    user_role::user_role_service_client::UserRoleServiceClient,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};

pub mod role;
pub mod user_role;

#[derive(Clone)]
pub struct GrpcClients {
    pub role_client: Arc<Mutex<RoleQueryServiceClient<Channel>>>,
    pub user_role_client: Arc<Mutex<UserRoleServiceClient<Channel>>>,
}

impl GrpcClients {
    pub async fn init(config: GrpcClientConfig) -> Result<Self> {
        let role_channel = Self::connect(config.role, "role-service").await?;

        Ok(Self {
            role_client: Arc::new(Mutex::new(RoleQueryServiceClient::new(
                role_channel.clone(),
            ))),
            user_role_client: Arc::new(Mutex::new(UserRoleServiceClient::new(
                role_channel.clone(),
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
