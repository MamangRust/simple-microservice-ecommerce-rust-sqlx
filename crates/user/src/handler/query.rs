use crate::{
    abstract_trait::user::service::DynUserQueryService, domain::requests::user::FindAllUsers,
};
use genproto::user::{
    ApiResponsePaginationUser, ApiResponsePaginationUserDeleteAt, ApiResponseUser,
    ApiResponseUserWithPassword, FindAllUserRequest, FindByEmailUserRequest, FindByIdUserRequest,
    UserResponse, UserResponseDeleteAt, VerifyCodeRequest,
    user_query_service_server::UserQueryService,
};
use shared::errors::AppErrorGrpc;
use std::fmt;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct UserQueryGrpcServiceImpl {
    pub user_query_service: DynUserQueryService,
}

impl fmt::Debug for UserQueryGrpcServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserQueryGrpcServiceImpl")
            .field("user_query_service", &"DynUserQueryService")
            .finish()
    }
}

impl UserQueryGrpcServiceImpl {
    pub fn new(user: DynUserQueryService) -> Self {
        Self {
            user_query_service: user,
        }
    }
}

#[tonic::async_trait]
impl UserQueryService for UserQueryGrpcServiceImpl {
    async fn find_all(
        &self,
        request: Request<FindAllUserRequest>,
    ) -> Result<Response<ApiResponsePaginationUser>, Status> {
        info!("Handling gRPC request: FindAll Users");

        let req = request.into_inner();

        let domain_req = FindAllUsers {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .user_query_service
            .find_all(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<UserResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationUser {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} Users", len);

        Ok(Response::new(reply))
    }

    async fn find_by_id(
        &self,
        request: Request<FindByIdUserRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Handling gRPC request: Find User by ID");

        let req = request.into_inner();

        let api_response = self
            .user_query_service
            .find_by_id(req.id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Successfully fetched User with ID: {}", req.id);
        Ok(Response::new(reply))
    }

    async fn find_by_email(
        &self,
        request: Request<FindByEmailUserRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Handling gRPC request: Find User by Email");

        let req = request.into_inner();

        let api_response = self
            .user_query_service
            .find_by_email(req.email.clone())
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!(
            "Successfully fetched User with Email: {}",
            req.email.clone()
        );
        Ok(Response::new(reply))
    }

    async fn find_by_email_and_verify(
        &self,
        request: Request<FindByEmailUserRequest>,
    ) -> Result<Response<ApiResponseUserWithPassword>, Status> {
        info!("Handling gRPC request: Find User by Email and Verify");

        let req = request.into_inner();

        let api_response = self
            .user_query_service
            .find_by_email_and_verify(req.email.clone())
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserWithPassword {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!(
            "Successfully fetched User with Email for verification: {}",
            req.email
        );
        Ok(Response::new(reply))
    }
    async fn find_verification_code(
        &self,
        request: Request<VerifyCodeRequest>,
    ) -> Result<Response<ApiResponseUser>, Status> {
        info!("Handling gRPC request: Find User by Verification Code");

        let req = request.into_inner();

        let api_response = self
            .user_query_service
            .find_by_verification_code(req.code.clone())
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUser {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!(
            "Successfully verified code for User with Code: {}",
            req.code.clone()
        );
        Ok(Response::new(reply))
    }

    async fn find_by_active(
        &self,
        request: Request<FindAllUserRequest>,
    ) -> Result<Response<ApiResponsePaginationUserDeleteAt>, Status> {
        info!("Handling gRPC request: Find active Users");

        let req = request.into_inner();

        let domain_req = FindAllUsers {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .user_query_service
            .find_active(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<UserResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationUserDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} active Users", len);
        Ok(Response::new(reply))
    }

    async fn find_by_trashed(
        &self,
        request: Request<FindAllUserRequest>,
    ) -> Result<Response<ApiResponsePaginationUserDeleteAt>, Status> {
        info!("Handling gRPC request: Find trashed Users");

        let req = request.into_inner();

        let domain_req = FindAllUsers {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .user_query_service
            .find_trashed(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<UserResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationUserDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} trashed Users", len);
        Ok(Response::new(reply))
    }
}
