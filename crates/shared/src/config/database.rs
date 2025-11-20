use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub type ConnectionPool = Pool<Postgres>;

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn new_pool(connection_string: &str) -> anyhow::Result<ConnectionPool> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to create database connection pool: {}", err))?;

        Ok(pool)
    }
}
