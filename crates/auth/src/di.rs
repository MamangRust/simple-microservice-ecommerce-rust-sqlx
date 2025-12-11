use crate::{
    abstract_trait::{auth::DynTokenService, grpc_client::user::DynUserGrpcClient},
    grpc_client::{GrpcClients, user::UserGrpcClientService},
    repository::{refresh_token::RefreshTokenRepository, reset_token::ResetTokenRepository},
    service::{
        forgot::{PasswordResetService, PasswordResetServiceDeps},
        identity::{IdentityService, IdentityServiceDeps},
        login::{LoginService, LoginServiceDeps},
        register::{RegisterService, RegisterServiceDeps},
        token::TokenService,
    },
};
use anyhow::{Result, Context};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::{DynHashing, DynJwtService, DynKafka},
    cache::CacheStore,
    config::{ConnectionPool, RedisClient},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct DependenciesInject {
    pub login_service: LoginService,
    pub register_service: RegisterService,
    pub identity_service: IdentityService,
    pub password_reset_service: PasswordResetService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("login_service", &"LoginService")
            .field("register_service", &"RegisterService")
            .field("identity_service", &"IdentityService")
            .field("password_reset_service", &"PasswordResetService")
            .finish()
    }
}

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub hash: DynHashing,
    pub jwt_config: DynJwtService,
    pub kafka: DynKafka,
    pub redis: RedisClient,
}

impl DependenciesInject {
    pub async fn new(deps: DependenciesInjectDeps, clients: GrpcClients, registry: &mut Registry) -> Result<Self> {
        let DependenciesInjectDeps {
            hash,
            pool,
            jwt_config,
            kafka,
            redis,
        } = deps;

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let refresh_token = RefreshTokenRepository::new(pool.clone());
        let reset_token = ResetTokenRepository::new(pool.clone());

        let user_client: DynUserGrpcClient = Arc::new(
            UserGrpcClientService::new(
                clients.user_query_client.clone(),
                clients.user_command_client.clone(),
            ),
        );

        let register_deps = RegisterServiceDeps {
            user_client: user_client.clone(),
            kafka: kafka.clone(),
            cache_store: cache.clone(),
        };

        let register_service = RegisterService::new(register_deps, registry).context("failed initialize register")?;

        let token_service = Arc::new(TokenService::new(
            jwt_config.clone(),
            refresh_token.command.clone(),
        )) as DynTokenService;

        let login_deps = LoginServiceDeps {
            hash,
            token_service: token_service.clone(),
            user_client: user_client.clone(),
            cache_store: cache.clone(),
        };

        let login_service = LoginService::new(login_deps, registry).context("failed initialize login")?;

        let identity_deps = IdentityServiceDeps {
            refresh_token_command: refresh_token.command.clone(),
            jwt: jwt_config,
            token_service: token_service.clone(),
            user_client: user_client.clone(),
            cache_store: cache.clone(),
        };

        let identity_service = IdentityService::new(identity_deps, registry).context("failed initialize identity")?;

        let password_deps = PasswordResetServiceDeps {
            reset_token_query: reset_token.query,
            reset_token_command: reset_token.command,
            user_client: user_client.clone(),
            kafka: kafka.clone(),
            cache_store: cache.clone(),
        };

        let password_reset_service = PasswordResetService::new(password_deps, registry).context("failed iniliazlie password reset")?;

        Ok(Self {
            login_service,
            register_service,
            identity_service,
            password_reset_service,
        })
    }
}
