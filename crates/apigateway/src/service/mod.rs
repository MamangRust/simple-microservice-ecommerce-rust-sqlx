mod auth;
mod order;
mod order_item;
mod product;
mod role;
mod user;

pub use self::auth::AuthGrpcClientService;
pub use self::order::OrderGrpcClientService;
pub use self::order_item::OrderItemGrpcClientService;
pub use self::product::ProductGrpcClientService;
pub use self::role::RoleGrpcClientService;
pub use self::user::UserGrpcClientService;

use crate::config::GrpcClientConfig;
use anyhow::{Context, Result};
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

use genproto::{
    auth::auth_service_client::AuthServiceClient,
    order::{
        order_command_service_client::OrderCommandServiceClient,
        order_query_service_client::OrderQueryServiceClient,
    },
    order_item::order_item_service_client::OrderItemServiceClient,
    product::{
        product_command_service_client::ProductCommandServiceClient,
        product_query_service_client::ProductQueryServiceClient,
    },
    role::{
        role_command_service_client::RoleCommandServiceClient,
        role_query_service_client::RoleQueryServiceClient,
    },
    user::{
        user_command_service_client::UserCommandServiceClient,
        user_query_service_client::UserQueryServiceClient,
    },
};

#[derive(Clone)]
pub struct GrpcClients {
    pub auth: AuthServiceClient<Channel>,

    // User
    pub user_command: UserCommandServiceClient<Channel>,
    pub user_query: UserQueryServiceClient<Channel>,

    // Role
    pub role_command: RoleCommandServiceClient<Channel>,
    pub role_query: RoleQueryServiceClient<Channel>,

    // Product
    pub product_command: ProductCommandServiceClient<Channel>,
    pub product_query: ProductQueryServiceClient<Channel>,

    // Order
    pub order_command: OrderCommandServiceClient<Channel>,
    pub order_query: OrderQueryServiceClient<Channel>,

    pub order_item: OrderItemServiceClient<Channel>,
}

impl GrpcClients {
    pub async fn init(config: GrpcClientConfig) -> Result<Self> {
        let auth_channel = Self::connect(config.auth, "auth-service").await?;
        let user_channel = Self::connect(config.user, "user-service").await?;
        let role_channel = Self::connect(config.role, "role-service").await?;
        let product_channel = Self::connect(config.product, "product-service").await?;
        let order_channel = Self::connect(config.order, "order-service").await?;

        Ok(Self {
            auth: AuthServiceClient::new(auth_channel),

            user_command: UserCommandServiceClient::new(user_channel.clone()),
            user_query: UserQueryServiceClient::new(user_channel),

            role_command: RoleCommandServiceClient::new(role_channel.clone()),
            role_query: RoleQueryServiceClient::new(role_channel),

            product_command: ProductCommandServiceClient::new(product_channel.clone()),
            product_query: ProductQueryServiceClient::new(product_channel),

            order_command: OrderCommandServiceClient::new(order_channel.clone()),
            order_query: OrderQueryServiceClient::new(order_channel.clone()),
            order_item: OrderItemServiceClient::new(order_channel),
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
