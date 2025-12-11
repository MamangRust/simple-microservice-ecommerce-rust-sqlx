use crate::{
    abstract_trait::grpc_client::role::RoleGrpcClientTrait,
    domain::response::{api::ApiResponse, role::RoleResponse},
};
use anyhow::Result;
use async_trait::async_trait;
use genproto::role::{
    FindByNameRequest,
    role_query_service_client::RoleQueryServiceClient as RoleQueryServiceGrpcClient,
};
use shared::errors::AppErrorGrpc;
use tonic::{Request, transport::Channel};

pub struct RoleGrpcClientService {
    client: RoleQueryServiceGrpcClient<Channel>,
}

impl RoleGrpcClientService {
    pub async fn new(client: RoleQueryServiceGrpcClient<Channel>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl RoleGrpcClientTrait for RoleGrpcClientService {
    async fn find_by_name(&self, name: &str) -> Result<ApiResponse<RoleResponse>, AppErrorGrpc> {
        let req = Request::new(FindByNameRequest {
            name: name.to_string(),
        });

        let mut client = self.client.clone();

        let response = client.find_by_name(req).await.map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("Role data is missing in gRPC response".into())
        })?;

        let domain_user: RoleResponse = user_data.into();

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "OK".to_string(),
            data: domain_user,
        })
    }
}
