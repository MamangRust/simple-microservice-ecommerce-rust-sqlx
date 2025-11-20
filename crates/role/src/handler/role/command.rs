use crate::{
    abstract_trait::role::service::DynRoleCommandService,
    domain::requests::role::{
        CreateRoleRequest as DomainCreateRoleRequest, UpdateRoleRequest as DomainUpdateRoleRequest,
    },
};
use genproto::role::{
    ApiResponseRole, ApiResponseRoleAll, ApiResponseRoleDelete, ApiResponseRoleDeleteAt,
    CreateRoleRequest, FindByIdRoleRequest, UpdateRoleRequest,
    role_command_service_server::RoleCommandService,
};

use shared::errors::AppErrorGrpc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct RoleCommandServiceImpl {
    pub command: DynRoleCommandService,
}

impl RoleCommandServiceImpl {
    pub fn new(command: DynRoleCommandService) -> Self {
        Self { command }
    }
}

#[tonic::async_trait]
impl RoleCommandService for RoleCommandServiceImpl {
    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<ApiResponseRole>, Status> {
        info!("Creating new Role");

        let req = request.into_inner();

        let domain_req = DomainCreateRoleRequest { name: req.name };

        let api_response = self
            .command
            .create_role(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRole {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.clone().into()),
        };

        info!(
            "Role created successfully with ID: {}",
            api_response.data.role_id
        );
        Ok(Response::new(reply))
    }

    async fn update_role(
        &self,
        request: Request<UpdateRoleRequest>,
    ) -> Result<Response<ApiResponseRole>, Status> {
        info!("Updating Role");

        let req = request.into_inner();

        let domain_req = DomainUpdateRoleRequest {
            id: req.id,
            name: req.name,
        };

        let api_response = self
            .command
            .update_role(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRole {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Role updated successfully: ID={}", req.id);
        Ok(Response::new(reply))
    }

    async fn trashed_role(
        &self,
        request: Request<FindByIdRoleRequest>,
    ) -> Result<Response<ApiResponseRoleDeleteAt>, Status> {
        info!("Soft deleting Role");

        let req = request.into_inner();

        let api_response = self
            .command
            .trash_role(req.role_id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRoleDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Role soft deleted: ID={}", req.role_id);
        Ok(Response::new(reply))
    }

    async fn restore_role(
        &self,
        request: Request<FindByIdRoleRequest>,
    ) -> Result<Response<ApiResponseRoleDeleteAt>, Status> {
        info!("Restoring Role");

        let req = request.into_inner();

        let api_response = self
            .command
            .restore_role(req.role_id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRoleDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Role restored: ID={}", req.role_id);
        Ok(Response::new(reply))
    }

    async fn delete_role_permanent(
        &self,
        request: Request<FindByIdRoleRequest>,
    ) -> Result<Response<ApiResponseRoleDelete>, Status> {
        info!("Permanently deleting Role");

        let req = request.into_inner();

        let api_response = self
            .command
            .delete_ole(req.role_id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRoleDelete {
            status: "success".into(),
            message: api_response.message,
        };

        info!("Role permanently deleted: ID={}", req.role_id);
        Ok(Response::new(reply))
    }

    async fn restore_all_role(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseRoleAll>, Status> {
        info!("Restoring all soft-deleted Roles");

        let _req = request.into_inner();

        let api_response = self
            .command
            .restore_all_role()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRoleAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All Roles restored successfully");
        Ok(Response::new(reply))
    }

    async fn delete_all_role_permanent(
        &self,
        request: Request<()>,
    ) -> Result<Response<ApiResponseRoleAll>, Status> {
        info!("Permanently deleting all soft-deleted Roles");

        let _req = request.into_inner();

        let api_response = self
            .command
            .delete_all_role()
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRoleAll {
            status: "success".into(),
            message: api_response.message,
        };

        info!("All Roles permanently deleted");
        Ok(Response::new(reply))
    }
}
