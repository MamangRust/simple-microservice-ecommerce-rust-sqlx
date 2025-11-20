use crate::{
    abstract_trait::{DynHashing, DynJwtService, DynKafka, DynUserRoleCommandRepository},
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
    repository::{
        OrderRepository, ProductRepository, RefreshTokenRepository, ResetTokenRepository,
        RoleRepository, UserRepository, UserRoleRepository,
    },
    service::{
        AuthService, AuthServiceDeps, OrderService, OrderServiceDeps, ProductService, RoleService,
        UserService,
    },
    utils::Metrics,
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DependenciesInject {
    pub auth_service: AuthService,
    pub role_service: RoleService,
    pub user_service: UserService,
    pub product_service: ProductService,
    pub order_service: OrderService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("auth_service", &"<AuthService>")
            .field("role_service", &"<RoleService>")
            .field("user_service", &"<UserService>")
            .field("product_service", &"<ProductService>")
            .field("order_service", &"<OrderService>")
            .finish()
    }
}

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub hash: DynHashing,
    pub jwt_config: DynJwtService,
    pub kafka: DynKafka,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub redis: RedisClient,
}

impl DependenciesInject {
    pub async fn new(deps: DependenciesInjectDeps) -> Result<Self> {
        let DependenciesInjectDeps {
            pool,
            hash,
            jwt_config,
            kafka,
            metrics,
            registry,
            redis,
        } = deps;

        let refresh_token = RefreshTokenRepository::new(pool.clone());
        let reset_token = ResetTokenRepository::new(pool.clone());
        let user_role =
            Arc::new(UserRoleRepository::new(pool.clone())) as DynUserRoleCommandRepository;

        let user_repository = UserRepository::new(pool.clone());
        let role_repository = RoleRepository::new(pool.clone());
        let product_repository = ProductRepository::new(pool.clone());
        let order_repository = OrderRepository::new(pool.clone());

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let user_service = UserService::new(
            user_repository.query.clone(),
            user_repository.command.clone(),
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await?;

        let auth_deps = AuthServiceDeps {
            kafka: kafka.clone(),
            hash,
            jwt: jwt_config,
            role: role_repository.clone().query,
            user_role,
            reset_query: reset_token.query,
            reset_command: reset_token.command,
            user_query: user_repository.query,
            user_command: user_repository.command,
            refresh_command: refresh_token.command,
            metrics: metrics.clone(),
            registry: registry.clone(),
            cache: cache.clone(),
        };

        let auth_service = AuthService::new(auth_deps).await?;

        let role_service = RoleService::new(
            role_repository.clone().query,
            role_repository.command,
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await?;

        let product_service = ProductService::new(
            product_repository.query.clone(),
            product_repository.command,
            metrics.clone(),
            registry.clone(),
            cache.clone(),
        )
        .await?;

        let order_deps = OrderServiceDeps {
            query: order_repository.query.clone(),
            command: order_repository.command.clone(),
            product_query: product_repository.query.clone(),
            kafka: kafka.clone(),
            metrics: metrics.clone(),
            registry: registry.clone(),
            cache_store: cache.clone(),
        };

        let order_service = OrderService::new(order_deps).await?;

        Ok(Self {
            auth_service,
            role_service,
            user_service,
            product_service,
            order_service,
        })
    }
}
