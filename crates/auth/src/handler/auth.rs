use crate::{
    abstract_trait::auth::{
        DynIdentityService, DynLoginService, DynPasswordResetService, DynRegisterService,
    },
    domain::requests::{
        auth::{AuthRequest as DomainLoginRequest, RegisterRequest as DomainRegisterRequest},
        reset_token::CreateResetPasswordRequest,
    },
};
use genproto::{
    auth::{
        ApiResponseForgotPassword, ApiResponseGetMe, ApiResponseLogin, ApiResponseRefreshToken,
        ApiResponseRegister, ApiResponseResetPassword, ApiResponseVerifyCode,
        ForgotPasswordRequest, GetMeRequest, LoginRequest, RefreshTokenRequest,
        ResetPasswordRequest, VerifyCodeRequest, auth_service_server::AuthService,
    },
    common::RegisterRequest,
};
use shared::errors::AppErrorGrpc;
use std::fmt;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct AuthGrpcServiceImpl {
    pub identity_service: DynIdentityService,
    pub login_service: DynLoginService,
    pub register_service: DynRegisterService,
    pub password_reset_service: DynPasswordResetService,
}

impl fmt::Debug for AuthGrpcServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthGrpcServiceImpl")
            .field("identity_service", &"DynIdentityService")
            .field("login_service", &"DynLoginService")
            .field("register_service", &"DynRegisterService")
            .field("password_reset_service", &"DynPasswordResetService")
            .finish()
    }
}

#[derive(Clone)]
pub struct AuthGrpcServiceDeps {
    pub identity_service: DynIdentityService,
    pub login_service: DynLoginService,
    pub register_service: DynRegisterService,
    pub password_reset_service: DynPasswordResetService,
}

impl AuthGrpcServiceImpl {
    pub fn new(deps: AuthGrpcServiceDeps) -> Self {
        let AuthGrpcServiceDeps {
            identity_service,
            login_service,
            register_service,
            password_reset_service,
        } = deps;

        Self {
            identity_service,
            login_service,
            register_service,
            password_reset_service,
        }
    }
}

#[tonic::async_trait]
impl AuthService for AuthGrpcServiceImpl {
    async fn verify_code(
        &self,
        request: Request<VerifyCodeRequest>,
    ) -> Result<Response<ApiResponseVerifyCode>, Status> {
        info!("Verifying code");

        let req = request.into_inner();

        let api_response = self
            .password_reset_service
            .verify_code(&req.code)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseVerifyCode {
            status: "sucess".into(),
            message: api_response.message,
        };

        info!("Verify code successfully");

        Ok(Response::new(reply))
    }
    async fn forgot_password(
        &self,
        request: Request<ForgotPasswordRequest>,
    ) -> Result<Response<ApiResponseForgotPassword>, Status> {
        info!(
            "Handling forgot password for email: {}",
            request.get_ref().email
        );

        let req = request.into_inner();

        let api_response = self
            .password_reset_service
            .forgot(&req.email)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseForgotPassword {
            status: "success".into(),
            message: api_response.message,
        };

        info!("Forgot password email sent successfully");
        Ok(Response::new(reply))
    }

    async fn reset_password(
        &self,
        request: Request<ResetPasswordRequest>,
    ) -> Result<Response<ApiResponseResetPassword>, Status> {
        info!("Resetting password for user");

        let req = request.into_inner();

        let domain_req = CreateResetPasswordRequest {
            reset_token: req.reset_token,
            password: req.password,
            confirm_password: req.confirm_password,
        };

        let api_response = self
            .password_reset_service
            .reset_password(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseResetPassword {
            status: "success".into(),
            message: api_response.message,
        };

        info!("Password reset successfully");
        Ok(Response::new(reply))
    }

    async fn register_user(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<ApiResponseRegister>, Status> {
        info!("Registering user {}", request.get_ref().email);

        let req = request.into_inner();

        let domain_req = DomainRegisterRequest {
            first_name: req.firstname,
            last_name: req.lastname,
            email: req.email,
            password: req.password,
            confirm_password: req.confirm_password,
        };

        let api_response = self
            .register_service
            .register(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRegister {
            status: api_response.status,
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("User registered successfully");
        Ok(Response::new(reply))
    }

    async fn login_user(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<ApiResponseLogin>, Status> {
        info!("Logging in user {}", request.get_ref().email);

        let req = request.into_inner();

        let domain_req = DomainLoginRequest {
            email: req.email,
            password: req.password,
        };

        let api_response = self
            .login_service
            .login(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseLogin {
            status: api_response.status,
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("User logged in successfully");
        Ok(Response::new(reply))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<ApiResponseRefreshToken>, Status> {
        info!("Refreshing token");

        let req = request.into_inner();

        let api_response = self
            .identity_service
            .refresh_token(&req.refresh_token)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRefreshToken {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Token refreshed successfully");
        Ok(Response::new(reply))
    }

    async fn get_me(
        &self,
        request: Request<GetMeRequest>,
    ) -> Result<Response<ApiResponseGetMe>, Status> {
        info!("Getting user profile");

        let req = request.into_inner();

        let api_response = self
            .identity_service
            .get_me(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseGetMe {
            status: "success".into(),
            message: "User fetched successfully".into(),
            data: api_response.data.map(Into::into),
        };

        info!("User fetched successfully");
        Ok(Response::new(reply))
    }
}
