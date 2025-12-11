use crate::errors::repository::RepositoryError;
use bcrypt::BcryptError;
use jsonwebtoken::errors::Error as JwtError;
use rdkafka::error::KafkaError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepositoryError),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Validation failed: {0:?}")]
    Validation(Vec<String>),

    #[error("Bcrypt error: {0}")]
    Bcrypt(#[from] BcryptError),

    #[error("JWT error: {0}")]
    Jwt(#[from] JwtError),

    #[error("Kafka error: {0}")]
    Kafka(String),

    #[error("Token has expired")]
    TokenExpired,

    #[error("Invalid Token")]
    InvalidTokenType,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<KafkaError> for ServiceError {
    fn from(error: KafkaError) -> Self {
        ServiceError::Kafka(error.to_string())
    }
}
