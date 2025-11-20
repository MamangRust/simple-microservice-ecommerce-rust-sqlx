mod forgot;
mod identity;
mod login;
mod register;
mod token;

use self::forgot::PasswordResetService;
use self::identity::IdentityService;
use self::login::LoginService;
use self::register::RegisterService;
use self::token::TokenService;
use crate::{
    abstract_trait::{
        DynHashing, DynIdentityService, DynJwtService, DynKafka, DynLoginService,
        DynPasswordResetService, DynRefreshTokenCommandRepository, DynRegisterService,
        DynResetTokenCommandRepository, DynResetTokenQueryRepository, DynRoleQueryRepository,
        DynTokenService, DynUserCommandRepository, DynUserQueryRepository,
        DynUserRoleCommandRepository,
    },
    cache::CacheStore,
    service::auth::{
        forgot::PasswordResetServiceDeps, identity::IdentityServiceDeps, login::LoginServiceDeps,
        register::RegisterServiceDeps,
    },
    utils::Metrics,
};
use anyhow::Result;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AuthService {
    pub passreset: DynPasswordResetService,
    pub identity: DynIdentityService,
    pub login: DynLoginService,
    pub register: DynRegisterService,
    pub token: DynTokenService,
}

#[derive(Clone)]
pub struct AuthServiceDeps {
    pub kafka: DynKafka,
    pub hash: DynHashing,
    pub jwt: DynJwtService,
    pub role: DynRoleQueryRepository,
    pub user_role: DynUserRoleCommandRepository,
    pub reset_query: DynResetTokenQueryRepository,
    pub reset_command: DynResetTokenCommandRepository,
    pub user_query: DynUserQueryRepository,
    pub user_command: DynUserCommandRepository,
    pub refresh_command: DynRefreshTokenCommandRepository,
    pub cache: Arc<CacheStore>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
}

impl AuthService {
    pub async fn new(deps: AuthServiceDeps) -> Result<Self> {
        let token = Arc::new(TokenService::new(
            deps.jwt.clone(),
            deps.refresh_command.clone(),
        )) as DynTokenService;

        let password_deps = PasswordResetServiceDeps {
            reset_token_query: deps.reset_query.clone(),
            reset_token_command: deps.reset_command.clone(),
            user_query: deps.user_query.clone(),
            user_command: deps.user_command.clone(),
            kafka: deps.kafka.clone(),
            metrics: deps.metrics.clone(),
            registry: deps.registry.clone(),
            cache_store: deps.cache.clone(),
        };

        let passreset =
            Arc::new(PasswordResetService::new(password_deps).await) as DynPasswordResetService;

        let identity_deps = IdentityServiceDeps {
            refresh_token_command: deps.refresh_command.clone(),
            token: deps.jwt.clone(),
            token_service: token.clone(),
            user_query: deps.user_query.clone(),
            metrics: deps.metrics.clone(),
            registry: deps.registry.clone(),
            cache_store: deps.cache.clone(),
        };

        let identity = Arc::new(IdentityService::new(identity_deps).await) as DynIdentityService;

        let login_deps = LoginServiceDeps {
            hash: deps.hash.clone(),
            token_service: token.clone(),
            query: deps.user_query.clone(),
            metrics: deps.metrics.clone(),
            registry: deps.registry.clone(),
            cache_store: deps.cache.clone(),
        };

        let login = Arc::new(LoginService::new(login_deps).await) as DynLoginService;

        let register_deps = RegisterServiceDeps {
            query: deps.user_query.clone(),
            command: deps.user_command.clone(),
            role: deps.role.clone(),
            user_role: deps.user_role.clone(),
            hash: deps.hash.clone(),
            kafka: deps.kafka.clone(),
            metrics: deps.metrics.clone(),
            registry: deps.registry.clone(),
            cache_store: deps.cache.clone(),
        };

        let register = Arc::new(RegisterService::new(register_deps).await) as DynRegisterService;

        Ok(Self {
            passreset,
            identity,
            login,
            register,
            token,
        })
    }
}
