use crate::abstract_trait::auth::{
    DynIdentityService, DynLoginService, DynPasswordResetService, DynRegisterService,
};
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
use anyhow::{Context, Result};
use shared::{
    abstract_trait::{DynHashing, DynJwtService, DynKafka},
    cache::CacheStore,
    config::{ConnectionPool, RedisPool},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct DependenciesInject {
    pub login_service: DynLoginService,
    pub register_service: DynRegisterService,
    pub identity_service: DynIdentityService,
    pub password_reset_service: DynPasswordResetService,
}

impl fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("login_service", &"DynLoginService")
            .field("register_service", &"DynRegisterService")
            .field("identity_service", &"DynIdentityService")
            .field("password_reset_service", &"DynPasswordResetService")
            .finish()
    }
}

#[derive(Clone)]
pub struct DependenciesInjectDeps {
    pub pool: ConnectionPool,
    pub hash: DynHashing,
    pub jwt_config: DynJwtService,
    pub kafka: DynKafka,
    pub redis: RedisPool,
}

impl DependenciesInject {
    pub async fn new(deps: DependenciesInjectDeps, clients: GrpcClients) -> Result<Self> {
        let DependenciesInjectDeps {
            hash,
            pool,
            jwt_config,
            kafka,
            redis,
        } = deps;

        let cache = Arc::new(CacheStore::new(redis.pool.clone()));

        let refresh_token = RefreshTokenRepository::new(pool.clone());
        let reset_token = ResetTokenRepository::new(pool.clone());

        let user_client: DynUserGrpcClient = Arc::new(UserGrpcClientService::new(
            clients.user_query_client.clone(),
            clients.user_command_client.clone(),
        ));

        let register_deps = RegisterServiceDeps {
            user_client: user_client.clone(),
            kafka: kafka.clone(),
            cache_store: cache.clone(),
        };

        let register_service =
            Arc::new(RegisterService::new(register_deps).context("failed initialize register")?)
                as DynRegisterService;

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

        let login_service =
            Arc::new(LoginService::new(login_deps).context("failed initialize login")?)
                as DynLoginService;

        let identity_deps = IdentityServiceDeps {
            refresh_token_command: refresh_token.command.clone(),
            jwt: jwt_config,
            token_service: token_service.clone(),
            user_client: user_client.clone(),
            cache_store: cache.clone(),
        };

        let identity_service =
            Arc::new(IdentityService::new(identity_deps).context("failed initialize identity")?)
                as DynIdentityService;

        let password_deps = PasswordResetServiceDeps {
            reset_token_query: reset_token.query,
            reset_token_command: reset_token.command,
            user_client: user_client.clone(),
            kafka: kafka.clone(),
            cache_store: cache.clone(),
        };

        let password_reset_service = Arc::new(
            PasswordResetService::new(password_deps).context("failed iniliazlie password reset")?,
        ) as DynPasswordResetService;

        Ok(Self {
            login_service,
            register_service,
            identity_service,
            password_reset_service,
        })
    }
}
