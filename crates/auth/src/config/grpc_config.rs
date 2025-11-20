use anyhow::{Context, Result};

#[derive(Clone)]
pub struct GrpcClientConfig {
    pub user: String,
    pub role: String,
}

impl GrpcClientConfig {
    pub fn init() -> Result<Self> {
        let user = std::env::var("GRPC_USER_ADDR")
            .context("Missing environment variable: GRPC_USER_ADDR")?;

        let role = std::env::var("GRPC_ROLE_ADDR")
            .context("Missing environment variable: GRPC_ROLE_ADDR")?;

        Ok(Self { user, role })
    }
}
