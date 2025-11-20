use anyhow::{Context, Result};

#[derive(Clone)]
pub struct GrpcClientConfig {
    pub role: String,
}

impl GrpcClientConfig {
    pub fn init() -> Result<Self> {
        let role = std::env::var("GRPC_ROLE_ADDR")
            .context("Missing environment variable: GRPC_ROLE_ADDR")?;

        Ok(Self { role })
    }
}
