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
use tracing::info;
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
        let auth_channel = Self::connect(&config.auth, "auth-service").await?;
        let user_channel = Self::connect(&config.user, "user-service").await?;
        let role_channel = Self::connect(&config.role, "role-service").await?;
        let product_channel = Self::connect(&config.product, "product-service").await?;
        let order_channel = Self::connect(&config.order, "order-service").await?;

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

    async fn connect(addr: &str, service: &str) -> Result<Channel> {
        info!("Connecting (balanced) to {} at {}", service, addr);

        const POOL_SIZE: usize = 10;

        let mut endpoints = Vec::with_capacity(POOL_SIZE);

        for _ in 0..POOL_SIZE {
            let ep = Endpoint::from_shared(addr.to_string())
                .with_context(|| format!("Invalid gRPC address for {service}: {addr}"))?
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(15))
                .tcp_keepalive(Some(Duration::from_secs(120)))
                .keep_alive_while_idle(true)
                .keep_alive_timeout(Duration::from_secs(10))
                .http2_keep_alive_interval(Duration::from_secs(20))
                .initial_connection_window_size(4 * 1024 * 1024)
                .initial_stream_window_size(2 * 1024 * 1024)
                .concurrency_limit(500)
                .rate_limit(1500, Duration::from_secs(1))
                .tcp_nodelay(true);

            endpoints.push(ep);
        }

        let channel = Channel::balance_list(endpoints.into_iter());

        info!("Successfully created balanced channel for {}", service);
        Ok(channel)
    }
}
