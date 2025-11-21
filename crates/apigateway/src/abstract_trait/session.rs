use crate::domain::response::session::Session;
use async_trait::async_trait;
use chrono::Duration;
use std::sync::Arc;

pub type DynSessionMiddleware = Arc<dyn SessionLimitMiddlewareTrait + Send + Sync>;

#[async_trait]
pub trait SessionLimitMiddlewareTrait {
    fn create_session(&self, session_id: &str, session: &Session, ttl: Duration) -> bool;
    fn get_session(&self, session_id: &str) -> Option<Session>;
    fn delete_session(&self, session_id: &str) -> bool;
    fn refresh_session(&self, session_id: &str, ttl: Duration) -> bool;
}
