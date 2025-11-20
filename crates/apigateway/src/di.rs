use anyhow::Result;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    abstract_trait::{
        auth::DynAuthGrpcClient, order::DynOrderGrpcClient, product::DynProductGrpcClient,
        role::DynRoleGrpcClient, user::DynUserGrpcClient,
    },
    service::{
        AuthGrpcClientService, GrpcClients, OrderGrpcClientService, ProductGrpcClientService,
        RoleGrpcClientService, UserGrpcClientService,
    },
};
use shared::utils::Metrics;

#[derive(Clone)]
pub struct DependenciesInject {
    pub auth_clients: DynAuthGrpcClient,
    pub role_clients: DynRoleGrpcClient,
    pub user_clients: DynUserGrpcClient,
    pub product_clients: DynProductGrpcClient,
    pub order_clients: DynOrderGrpcClient,
}

impl std::fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("auth_service", &"DynAuthService")
            .field("role_service", &"DynRoleService")
            .field("user_service", &"DynUserService")
            .field("product_service", &"DynProductService")
            .field("order_service", &"DynOrderService")
            .finish()
    }
}

impl DependenciesInject {
    pub async fn new(
        clients: GrpcClients,
        metrics: Arc<Mutex<Metrics>>,
        registry: Arc<Mutex<Registry>>,
    ) -> Result<Self> {
        let auth_clients: DynAuthGrpcClient = Arc::new(
            AuthGrpcClientService::new(clients.auth, metrics.clone(), registry.clone()).await,
        );

        let role_clients: DynRoleGrpcClient = Arc::new(
            RoleGrpcClientService::new(
                clients.role_query,
                clients.role_command,
                metrics.clone(),
                registry.clone(),
            )
            .await,
        );

        let user_clients: DynUserGrpcClient = Arc::new(
            UserGrpcClientService::new(
                clients.user_query,
                clients.user_command,
                metrics.clone(),
                registry.clone(),
            )
            .await,
        );

        let product_clients: DynProductGrpcClient = Arc::new(
            ProductGrpcClientService::new(
                clients.product_query,
                clients.product_command,
                metrics.clone(),
                registry.clone(),
            )
            .await,
        );

        let order_clients: DynOrderGrpcClient = Arc::new(
            OrderGrpcClientService::new(
                clients.order_query,
                clients.order_command,
                metrics.clone(),
                registry.clone(),
            )
            .await,
        );

        Ok(Self {
            auth_clients,
            role_clients,
            user_clients,
            product_clients,
            order_clients,
        })
    }
}
