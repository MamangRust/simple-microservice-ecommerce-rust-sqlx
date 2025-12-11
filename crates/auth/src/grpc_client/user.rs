use crate::{
    abstract_trait::grpc_client::user::UserGrpcClientTrait,
    domain::{
        requests::user::{CreateUserRequest, UpdateUserPasswordRequest, UpdateUserVerifiedRequest},
        response::{
            api::ApiResponse,
            user::{UserResponse, UserResponseWithPassword},
        },
    },
};
use anyhow::Result;
use async_trait::async_trait;
use genproto::{
    common::CreateUserRequest as CreateUserProtoRequest,
    user::{
        FindByEmailUserRequest, FindByIdUserRequest,
        UpdateUserPasswordRequest as UpdateUserPasswordProtoRequest,
        UpdateUserVerifiedRequest as UpdateUserVerifiedProtoRequest, VerifyCodeRequest,
        user_command_service_client::UserCommandServiceClient,
        user_query_service_client::UserQueryServiceClient,
    },
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, transport::Channel};

pub struct UserGrpcClientService {
    query_client: UserQueryServiceClient<Channel>,
    command_client: UserCommandServiceClient<Channel>,
}

impl UserGrpcClientService {
    pub fn new(
        query_client: UserQueryServiceClient<Channel>,
        command_client: UserCommandServiceClient<Channel>,
    ) -> Self {
        Self {
            query_client,
            command_client,
        }
    }
}

#[async_trait]
impl UserGrpcClientTrait for UserGrpcClientService {
    async fn create_user(
        &self,
        req: CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc> {
        let req = Request::new(CreateUserProtoRequest {
            firstname: req.first_name,
            lastname: req.last_name,
            email: req.email,
            password: req.password,
            confirm_password: req.confirm_password,
            verified_code: req.verified_code,
            is_verified: req.is_verified,
        });

        let mut client = self.command_client.clone();

        let response = client.create_user(req).await.map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponse = user_data.into();

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "OK".to_string(),
            data: domain_user,
        })
    }

    async fn update_user_is_verified(
        &self,
        req: UpdateUserVerifiedRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc> {
        let req = Request::new(UpdateUserVerifiedProtoRequest {
            user_id: req.user_id,
            is_verified: req.is_verified,
        });

        let mut client = self.command_client.clone();

        let response = client
            .update_user_is_verified(req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponse = user_data.into();

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "OK".to_string(),
            data: domain_user,
        })
    }

    async fn update_user_password(
        &self,
        req: UpdateUserPasswordRequest,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc> {
        let req = Request::new(UpdateUserPasswordProtoRequest {
            user_id: req.user_id,
            password: req.password,
        });

        let mut client = self.command_client.clone();

        let response = client
            .update_user_password(req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponse = user_data.into();

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "OK".to_string(),
            data: domain_user,
        })
    }

    async fn find_verification_code(
        &self,
        code: String,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc> {
        let request = Request::new(VerifyCodeRequest { code });

        let mut client = self.query_client.clone();

        let response = client
            .find_verification_code(request)
            .await
            .map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponse = user_data.into();

        Ok(ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        })
    }

    async fn find_by_email(
        &self,
        email: String,
    ) -> Result<ApiResponse<UserResponse>, AppErrorGrpc> {
        let request = Request::new(FindByEmailUserRequest { email });

        let mut client = self.query_client.clone();

        let response = client
            .find_by_email(request)
            .await
            .map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponse = user_data.into();

        Ok(ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        })
    }

    async fn find_by_email_and_verify(
        &self,
        email: String,
    ) -> Result<ApiResponse<UserResponseWithPassword>, AppErrorGrpc> {
        let request = Request::new(FindByEmailUserRequest { email });

        let mut client = self.query_client.clone();

        let response = client
            .find_by_email_and_verify(request)
            .await
            .map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponseWithPassword = user_data.into();

        Ok(ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        })
    }

    async fn find_by_id(&self, id: i32) -> Result<ApiResponse<UserResponse>, AppErrorGrpc> {
        let request = Request::new(FindByIdUserRequest { id });

        let mut client = self.query_client.clone();

        let response = client
            .find_by_id(request)
            .await
            .map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User data is missing in gRPC response".into())
        })?;

        let domain_user: UserResponse = user_data.into();

        Ok(ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        })
    }
}
