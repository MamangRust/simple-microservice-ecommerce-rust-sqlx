use crate::{
    abstract_trait::user::service::DynUserCommandService,
    domain::requests::user::{
        CreateUserRequest as DomainCreateUserRequest,
        UpdateUserPasswordRequest as DomainUpdatePasswordRequest,
        UpdateUserRequest as DomainUpdateUserRequest,
        UpdateUserVerifiedRequest as DomainUpdateVerifiedRequest,
    },
};
use genproto::{
    common::CreateUserRequest,
    user::{
        ApiResponseUser, ApiResponseUserAll, ApiResponseUserDelete, ApiResponseUserDeleteAt,
        FindByIdUserRequest, UpdateUserPasswordRequest, UpdateUserRequest,
        UpdateUserVerifiedRequest, user_command_service_server::UserCommandService,
    },
};
use shared::errors::AppErrorGrpc;
use std::fmt;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct UserCommandGrpcServiceImpl {
    pub user_command_service: DynUserCommandService,
}

impl fmt::Debug for UserCommandGrpcServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserCommandGrpcServiceImpl")
            .field("user_Command_service", &"DynUserCommandService")
            .finish()
    }
}

impl UserCommandGrpcServiceImpl {
    pub fn new(user: DynUserCommandService) -> Self {
        Self {
            user_command_service: user,
        }
    }
}

#[tonic::async_trait]
impl UserCommandService for UserCommandGrpcServiceImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Creating new User");

        let req = request.into_inner();

        let domain_req = DomainCreateUserRequest {
            first_name: req.firstname,
            last_name: req.lastname,
            email: req.email,
            password: req.password,
            confirm_password: req.confirm_password,
            is_verified: req.is_verified,
            verified_code: req.verified_code,
        };

        let api_response = self
            .user_command_service
            .create_user(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.clone().into()),
        };

        info!(
            "User created successfully with ID: {}",
            api_response.data.id
        );
        Ok(Response::new(reply))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Updating User");

        let req = request.into_inner();

        let domain_req = DomainUpdateUserRequest {
            user_id: Some(req.id),
            first_name: req.firstname,
            last_name: req.lastname,
            email: req.email,
            password: req.password,
            confirm_password: req.confirm_password,
        };

        let api_response = self
            .user_command_service
            .update_user(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("User updated successfully: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn update_user_is_verified(
        &self,
        request: Request<UpdateUserVerifiedRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Updating User Verification Status");

        let req = request.into_inner();

        let domain_req = DomainUpdateVerifiedRequest {
            user_id: req.user_id,
            is_verified: req.is_verified,
        };

        let api_response = self
            .user_command_service
            .update_user_is_verified(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!(
            "User verification status updated successfully: ID={}, is_verified={}",
            req.user_id, req.is_verified
        );
        Ok(Response::new(reply))
    }

    async fn update_user_password(
        &self,
        request: Request<UpdateUserPasswordRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Updating User Password");

        let req = request.into_inner();

        let domain_req = DomainUpdatePasswordRequest {
            user_id: req.user_id,
            password: req.password,
        };

        let api_response = self
            .user_command_service
            .update_user_password(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("User password updated successfully: ID={}", req.user_id);
        Ok(Response::new(reply))
    }

    async fn trashed_user(
        &self,
        request: Request<FindByIdUserRequest>,
    ) -> Result<Response<ApiResponseUserDeleteAt>, Status> {
        info!("Soft deleting User");

        let req = request.into_inner();

        let api_response = self
            .user_command_service
            .trash_user(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("User soft deleted: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn restore_user(
        &self,
        request: Request<FindByIdUserRequest>,
    ) -> Result<Response<ApiResponseUserDeleteAt>, Status> {
        info!("Restoring User");

        let req = request.into_inner();

        let api_response = self
            .user_command_service
            .restore_user(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("User restored: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn delete_user_permanent(
        &self,
        request: Request<FindByIdUserRequest>,
    ) -> Result<Response<ApiResponseUserDelete>, Status> {
        info!("Permanently deleting User");

        let req = request.into_inner();

        let api_response = self
            .user_command_service
            .delete_user(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserDelete {
            status: "success".into(),
            message: api_response.message,
        };

        info!("User permanently deleted: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn restore_all_user(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseUserAll>, Status> {
        info!("Restoring all soft-deleted Users");

        let _req = request.into_inner();

        let api_response = self
            .user_command_service
            .restore_all_user()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All Users restored successfully");
        Ok(Response::new(reply))
    }

    async fn delete_all_user_permanent(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseUserAll>, Status> {
        info!("Permanently deleting all soft-deleted Users");

        let _req = request.into_inner();

        let api_response = self
            .user_command_service
            .delete_all_user()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All Users permanently deleted");
        Ok(Response::new(reply))
    }
}
