use crate::{
    abstract_trait::user_role::service::DynUserRoleCommandService,
    domain::requests::user_role::CreateUserRoleRequest as DomainCreateUserRoleRequest,
};
use genproto::user_role::{
    ApiResponseUserRole, CreateUserRoleRequest, user_role_service_server::UserRoleService,
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct UserRoleServiceImpl {
    pub user_role: DynUserRoleCommandService,
}

impl UserRoleServiceImpl {
    pub fn new(user_role: DynUserRoleCommandService) -> Self {
        Self { user_role }
    }
}

#[tonic::async_trait]
impl UserRoleService for UserRoleServiceImpl {
    async fn assign_role(
        &self,
        request: Request<CreateUserRoleRequest>,
    ) -> Result<Response<ApiResponseUserRole>, Status> {
        info!("Assigning role to user");

        let req = request.into_inner();

        let domain_req = DomainCreateUserRoleRequest {
            user_id: req.userid,
            role_id: req.roleid,
        };

        let api_response = self
            .user_role
            .assign_role_to_user(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserRole {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!(
            "Role assigned successfully: user_id={}, role_id={}",
            req.userid, req.roleid
        );
        Ok(Response::new(reply))
    }

    async fn update_role(
        &self,
        request: Request<CreateUserRoleRequest>,
    ) -> Result<Response<ApiResponseUserRole>, Status> {
        info!("Updating user's role");

        let req = request.into_inner();

        let domain_req = DomainCreateUserRoleRequest {
            user_id: req.userid,
            role_id: req.roleid,
        };

        let api_response = self
            .user_role
            .update_role_to_user(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseUserRole {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!(
            "User role updated successfully: user_id={}, role_id={}",
            req.userid, req.roleid
        );
        Ok(Response::new(reply))
    }
}
