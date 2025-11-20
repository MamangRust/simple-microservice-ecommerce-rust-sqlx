use crate::{
    abstract_trait::grpc_client::user_role::UserRoleGrpcClientTrait,
    domain::{
        requests::user_role::CreateUserRoleRequest,
        response::{api::ApiResponse, user_role::UserRoleResponse},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use genproto::user_role::{
    CreateUserRoleRequest as CreateUserRolePbRequest,
    user_role_service_client::UserRoleServiceClient as UserRoleServiceGrpcClient,
};
use shared::errors::AppErrorGrpc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, transport::Channel};

pub struct UserRoleGrpcClientService {
    client: Arc<Mutex<UserRoleServiceGrpcClient<Channel>>>,
}

impl UserRoleGrpcClientService {
    pub async fn new(client: Arc<Mutex<UserRoleServiceGrpcClient<Channel>>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl UserRoleGrpcClientTrait for UserRoleGrpcClientService {
    async fn assign_role(
        &self,
        req: CreateUserRoleRequest,
    ) -> Result<ApiResponse<UserRoleResponse>, AppErrorGrpc> {
        let req = Request::new(CreateUserRolePbRequest {
            userid: req.user_id,
            roleid: req.role_id,
        });

        let mut client = self.client.lock().await;

        let response = client.assign_role(req).await.map_err(AppErrorGrpc::from)?;

        let inner = response.into_inner();

        let user_data = inner.data.ok_or_else(|| {
            AppErrorGrpc::Unhandled("User Role data is missing in gRPC response".into())
        })?;

        let domain_user: UserRoleResponse = user_data.into();

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "OK".to_string(),
            data: domain_user,
        })
    }
}
