use crate::abstract_trait::rate_limit::RateLimitMiddlewareTrait;
use async_trait::async_trait;
use redis::{Commands, Connection};
use std::sync::Arc;
use tracing::{debug, error};

#[derive(Clone)]
pub struct RateLimiter {
    pub redis: Arc<redis::Client>,
}

impl RateLimiter {
    pub fn new(redis: redis::Client) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }

    fn get_conn(&self) -> Option<Connection> {
        match self.redis.get_connection() {
            Ok(conn) => Some(conn),
            Err(e) => {
                error!("Failed to get Redis connection: {:?}", e);
                None
            }
        }
    }
}

#[async_trait]
impl RateLimitMiddlewareTrait for RateLimiter {
    fn check_rate_limit(&self, key: &str, max_requests: u32, window_seconds: u32) -> (bool, u32) {
        let mut conn = match self.get_conn() {
            Some(conn) => conn,
            None => return (false, 0),
        };

        let current: u32 = conn.get(key).unwrap_or(0);

        if current >= max_requests {
            debug!("Rate limit exceeded for key: {key}");
            return (false, current);
        }

        let _ = redis::pipe()
            .atomic()
            .cmd("INCR")
            .arg(key)
            .ignore()
            .cmd("EXPIRE")
            .arg(key)
            .arg(window_seconds)
            .ignore()
            .query::<()>(&mut conn);

        (true, current + 1)
    }

    fn get_remaining(&self, key: &str, max_requests: u32) -> u32 {
        let conn = match self.get_conn() {
            Some(conn) => conn,
            None => return 0,
        };

        let mut conn = conn;
        let current: u32 = conn.get(key).unwrap_or(0);
        max_requests.saturating_sub(current)
    }

    fn reset_limit(&self, key: &str) -> bool {
        if let Some(mut conn) = self.get_conn() {
            redis::cmd("DEL").arg(key).query::<()>(&mut conn).is_ok()
        } else {
            false
        }
    }
}
