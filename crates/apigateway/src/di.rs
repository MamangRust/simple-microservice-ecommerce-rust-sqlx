use anyhow::{Result, Context};
use prometheus_client::registry::Registry;
use std::sync::Arc;

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
    pub fn new(
        clients: GrpcClients,
        registry: &mut Registry,
    ) -> Result<Self> {
        let auth_clients: DynAuthGrpcClient = Arc::new(
            AuthGrpcClientService::new(clients.auth.clone(), registry)
                .context("Failed to initialize AuthGrpcClientService")?
        );

        let role_clients: DynRoleGrpcClient = Arc::new(
            RoleGrpcClientService::new(
                clients.role_query.clone(),
                clients.role_command.clone(),
                registry,
            )
                .context("Failed to initialize RoleGrpcClientService")?
        );

        let user_clients: DynUserGrpcClient = Arc::new(
            UserGrpcClientService::new(
                clients.user_query.clone(),
                clients.user_command.clone(),
                registry,
            )
                .context("Failed to initialize UserGrpcClientService")?
        );

        let product_clients: DynProductGrpcClient = Arc::new(
            ProductGrpcClientService::new(
                clients.product_query.clone(),
                clients.product_command.clone(),
                registry,
            )
                .context("Failed to initialize ProductGrpcClientService")?
        );

        let order_clients: DynOrderGrpcClient = Arc::new(
            OrderGrpcClientService::new(
                clients.order_query.clone(),
                clients.order_command.clone(),
                registry,
            )
                .context("Failed to initialize OrderGrpcClientService")?
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
