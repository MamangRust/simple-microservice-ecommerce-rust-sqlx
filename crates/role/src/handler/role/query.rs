use crate::{
    abstract_trait::role::service::DynRoleQueryService, domain::requests::role::FindAllRole,
};
use genproto::role::{
    ApiResponsePaginationRole, ApiResponsePaginationRoleDeleteAt, ApiResponseRole,
    ApiResponsesRole, FindAllRoleRequest, FindByIdRoleRequest, FindByIdUserRoleRequest,
    FindByNameRequest, role_query_service_server::RoleQueryService,
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct RoleQueryServiceImpl {
    pub query: DynRoleQueryService,
}

impl RoleQueryServiceImpl {
    pub fn new(query: DynRoleQueryService) -> Self {
        Self { query }
    }
}

#[tonic::async_trait]
impl RoleQueryService for RoleQueryServiceImpl {
    async fn find_all_role(
        &self,
        request: Request<FindAllRoleRequest>,
    ) -> Result<Response<ApiResponsePaginationRole>, Status> {
        info!("Handling gRPC request: FindAll Roles");

        let req = request.into_inner();

        let domain_req = FindAllRole {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .query
            .find_all(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::role::RoleResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationRole {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {len} Roles");

        Ok(Response::new(reply))
    }

    async fn find_by_id_role(
        &self,
        request: Request<FindByIdRoleRequest>,
    ) -> Result<Response<ApiResponseRole>, Status> {
        info!("Handling gRPC request: Find Role by ID");

        let req = request.into_inner();

        let api_response = self
            .query
            .find_by_id(req.role_id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRole {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Successfully fetched Role with ID: {}", req.role_id);
        Ok(Response::new(reply))
    }

    async fn find_by_name(
        &self,
        request: Request<FindByNameRequest>,
    ) -> Result<Response<ApiResponseRole>, Status> {
        info!("Handling gRPC request: Find Role by Name");

        let req = request.into_inner();

        let api_response = self
            .query
            .find_by_name(req.name.clone())
            .await
            .map_err(AppErrorGrpc::from)?;

        let reply = ApiResponseRole {
            status: "success".into(),
            message: api_response.message,
            data: Some(api_response.data.into()),
        };

        info!("Successfully fetched Role with Name: {}", req.name.clone());
        Ok(Response::new(reply))
    }

    async fn find_by_user_id(
        &self,
        request: Request<FindByIdUserRoleRequest>,
    ) -> Result<Response<ApiResponsesRole>, Status> {
        info!("Handling gRPC request: Find Role by ID");

        let req = request.into_inner();

        let api_response = self
            .query
            .find_by_user_id(req.user_id)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::role::RoleResponse> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let reply = ApiResponsesRole {
            status: "success".into(),
            message: api_response.message,
            data,
        };

        info!("Successfully fetched Role with ID: {}", req.user_id);
        Ok(Response::new(reply))
    }

    async fn find_by_active(
        &self,
        request: Request<FindAllRoleRequest>,
    ) -> Result<Response<ApiResponsePaginationRoleDeleteAt>, Status> {
        info!("Handling gRPC request: Find active Roles");

        let req = request.into_inner();

        let domain_req = FindAllRole {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .query
            .find_active(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::role::RoleResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationRoleDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {len} active Roles");
        Ok(Response::new(reply))
    }

    async fn find_by_trashed(
        &self,
        request: Request<FindAllRoleRequest>,
    ) -> Result<Response<ApiResponsePaginationRoleDeleteAt>, Status> {
        info!("Handling gRPC request: Find trashed Roles");

        let req = request.into_inner();

        let domain_req = FindAllRole {
            page: req.page,
            page_size: req.page_size,
            search: req.search,
        };

        let api_response = self
            .query
            .find_trashed(&domain_req)
            .await
            .map_err(AppErrorGrpc::from)?;

        let data: Vec<genproto::role::RoleResponseDeleteAt> = api_response
            .data
            .into_iter()
            .map(|item| item.into())
            .collect();

        let len = data.len();

        let reply = ApiResponsePaginationRoleDeleteAt {
            status: "success".into(),
            message: api_response.message,
            data,
            pagination: Some(api_response.pagination.into()),
        };

        info!("Successfully fetched {} trashed Roles", len);
        Ok(Response::new(reply))
    }
}
