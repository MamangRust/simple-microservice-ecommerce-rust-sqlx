use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub grpc_port: u16,
    pub metric_port: u16,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub run_migrations: bool,
    pub port: u16,
    pub product: ServiceConfig,
    pub order: ServiceConfig,
    pub kafka_broker: String,
    pub db_max_conn: u32,
    pub db_min_conn: u32,
}
impl Config {
    pub fn init() -> Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").context("Missing environment variable: DATABASE_URL")?;
        let jwt_secret =
            std::env::var("JWT_SECRET").context("Missing environment variable: JWT_SECRET")?;
        let run_migrations_str = std::env::var("RUN_MIGRATIONS")
            .context("Missing environment variable: RUN_MIGRATIONS")?;
        let port_str = std::env::var("PORT").context("Missing environment variable: PORT")?;

        let kafka_broker = std::env::var("KAFKA").context("Missing environment variable: KAFKA")?;

        let run_migrations = match run_migrations_str.as_str() {
            "true" => true,
            "false" => false,
            other => {
                return Err(anyhow!(
                    "RUN_MIGRATIONS must be 'true' or 'false', got '{}'",
                    other
                ));
            }
        };

        let db_max_conn: u32 = std::env::var("DB_MAX_CONNECTION")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>()
            .context("Unable to parse DB_MAX_CONNECTION as u32")?;

        let db_min_conn: u32 = std::env::var("DB_MIN_CONNECTION")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u32>()
            .context("Unable to parse DB_MIN_CONNECTION as u32")?;

        let port = port_str
            .parse::<u16>()
            .context("PORT must be a valid u16 integer")?;

        // order
        let order_grpc_port = std::env::var("ORDER_GRPC_PORT")
            .context("Missing environment variable: ORDER_GRPC_PORT")?
            .parse::<u16>()
            .context("ORDER_GRPC_PORT must be a valid u16 integer")?;

        let order_metric_port = std::env::var("ORDER_METRIC_PORT")
            .context("Missing environment variable: ORDER_METRIC_PORT")?
            .parse::<u16>()
            .context("ORDER_METRIC_PORT must be a valid u16 integer")?;

        // product
        let product_grpc_port = std::env::var("PRODUCT_GRPC_PORT")
            .context("Missing environment variable: PRODUCT_GRPC_PORT")?
            .parse::<u16>()
            .context("PRODUCT_GRPC_PORT must be a valid u16 integer")?;

        let product_metric_port = std::env::var("PRODUCT_METRIC_PORT")
            .context("Missing environment variable: PRODUCT_METRIC_PORT")?
            .parse::<u16>()
            .context("PRODUCT_METRIC_PORT must be a valid u16 integer")?;

        Ok(Self {
            database_url,
            jwt_secret,
            run_migrations,
            port,
            product: ServiceConfig {
                grpc_port: product_grpc_port,
                metric_port: product_metric_port,
            },
            order: ServiceConfig {
                grpc_port: order_grpc_port,
                metric_port: order_metric_port,
            },
            kafka_broker,
            db_max_conn,
            db_min_conn,
        })
    }
}
