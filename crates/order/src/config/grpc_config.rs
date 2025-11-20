use anyhow::{Context, Result};

#[derive(Clone)]
pub struct GrpcClientConfig {
    pub product: String,
}

impl GrpcClientConfig {
    pub fn init() -> Result<Self> {
        let product = std::env::var("GRPC_PRODUCT_ADDR")
            .context("Missing environment variable: GRPC_PRODUCT_ADDR")?;

        Ok(Self { product })
    }
}
