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
use std::sync::Arc;
use tokio::sync::Mutex;
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
    pub auth: Arc<Mutex<AuthServiceClient<Channel>>>,

    // User
    pub user_command: Arc<Mutex<UserCommandServiceClient<Channel>>>,
    pub user_query: Arc<Mutex<UserQueryServiceClient<Channel>>>,

    // Role
    pub role_command: Arc<Mutex<RoleCommandServiceClient<Channel>>>,
    pub role_query: Arc<Mutex<RoleQueryServiceClient<Channel>>>,

    // Product
    pub product_command: Arc<Mutex<ProductCommandServiceClient<Channel>>>,
    pub product_query: Arc<Mutex<ProductQueryServiceClient<Channel>>>,

    // Order
    pub order_command: Arc<Mutex<OrderCommandServiceClient<Channel>>>,
    pub order_query: Arc<Mutex<OrderQueryServiceClient<Channel>>>,

    pub order_item: Arc<Mutex<OrderItemServiceClient<Channel>>>,
}

impl GrpcClients {
    pub async fn init(config: GrpcClientConfig) -> Result<Self> {
        let auth_channel = Self::connect(config.auth, "auth-service").await?;
        let user_channel = Self::connect(config.user, "user-service").await?;
        let role_channel = Self::connect(config.role, "role-service").await?;
        let product_channel = Self::connect(config.product, "product-service").await?;
        let order_channel = Self::connect(config.order, "order-service").await?;

        Ok(Self {
            auth: Arc::new(Mutex::new(AuthServiceClient::new(auth_channel))),

            user_command: Arc::new(Mutex::new(UserCommandServiceClient::new(
                user_channel.clone(),
            ))),
            user_query: Arc::new(Mutex::new(UserQueryServiceClient::new(user_channel))),

            role_command: Arc::new(Mutex::new(RoleCommandServiceClient::new(
                role_channel.clone(),
            ))),
            role_query: Arc::new(Mutex::new(RoleQueryServiceClient::new(role_channel))),

            product_command: Arc::new(Mutex::new(ProductCommandServiceClient::new(
                product_channel.clone(),
            ))),
            product_query: Arc::new(Mutex::new(ProductQueryServiceClient::new(product_channel))),

            order_command: Arc::new(Mutex::new(OrderCommandServiceClient::new(
                order_channel.clone(),
            ))),
            order_query: Arc::new(Mutex::new(OrderQueryServiceClient::new(
                order_channel.clone(),
            ))),
            order_item: Arc::new(Mutex::new(OrderItemServiceClient::new(
                order_channel.clone(),
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
