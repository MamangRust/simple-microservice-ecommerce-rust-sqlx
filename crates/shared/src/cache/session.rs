use chrono::Duration;
use redis::{Commands, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
}
    
#[derive(Clone)]
pub struct SessionStore {
    pub redis: Arc<redis::Client>,
}

impl SessionStore {
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

    pub fn create_session(&self, session_id: &str, session: &Session, ttl: Duration) -> bool {
        let json_data = match serde_json::to_string(session) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize session: {:?}", e);
                return false;
            }
        };

        let conn = self.get_conn();
        if let Some(mut conn) = conn {
            let result: redis::RedisResult<()> = redis::pipe()
                .cmd("SET")
                .arg(session_id)
                .arg(&json_data)
                .ignore()
                .cmd("EXPIRE")
                .arg(session_id)
                .arg(ttl.num_seconds() as usize)
                .query(&mut conn);

            match result {
                Ok(_) => {
                    debug!("Session created for session_id: {}", session_id);
                    true
                }
                Err(e) => {
                    error!("Failed to create session: {:?}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let mut conn = self.get_conn()?;
        let result: redis::RedisResult<Option<String>> = conn.get(session_id);

        match result {
            Ok(Some(data)) => match serde_json::from_str::<Session>(&data) {
                Ok(session) => {
                    debug!("Session retrieved for session_id: {}", session_id);
                    Some(session)
                }
                Err(e) => {
                    error!("Failed to deserialize session: {:?}", e);
                    None
                }
            },
            Ok(None) => {
                debug!("Session not found: {}", session_id);
                None
            }
            Err(e) => {
                error!("Redis get error for session {}: {:?}", session_id, e);
                None
            }
        }
    }

    pub fn delete_session(&self, session_id: &str) -> bool {
        if let Some(mut conn) = self.get_conn() {
            if let Err(e) = redis::cmd("DEL").arg(session_id).query::<()>(&mut conn) {
                error!("Failed to delete session {}: {:?}", session_id, e);
                false
            } else {
                debug!("Session deleted: {}", session_id);
                true
            }
        } else {
            false
        }
    }

    pub fn refresh_session(&self, session_id: &str, ttl: Duration) -> bool {
        if let Some(mut conn) = self.get_conn() {
            if let Err(e) = redis::cmd("EXPIRE")
                .arg(session_id)
                .arg(ttl.num_seconds() as usize)
                .query::<()>(&mut conn)
            {
                error!("Failed to refresh session TTL {}: {:?}", session_id, e);
                false
            } else {
                debug!("Session TTL refreshed for session_id: {}", session_id);
                true
            }
        } else {
            false
        }
    }
}
