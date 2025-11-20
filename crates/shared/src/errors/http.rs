use crate::errors::{
    error::ErrorResponse, grpc::AppErrorGrpc, repository::RepositoryError, service::ServiceError,
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum HttpError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Conflict(String),
    ServiceUnavailable(String),
    Internal(String),
}

impl From<AppErrorGrpc> for HttpError {
    fn from(err: AppErrorGrpc) -> Self {
        match err {
            AppErrorGrpc::Service(service_err) => match service_err {
                ServiceError::InvalidCredentials => {
                    HttpError::Unauthorized("Invalid credentials".to_string())
                }

                ServiceError::Validation(errors) => {
                    HttpError::BadRequest(format!("Validation failed: {errors:?}"))
                }

                ServiceError::Repo(repo_err) => match repo_err {
                    RepositoryError::NotFound => HttpError::NotFound("Not found".into()),
                    RepositoryError::Conflict(msg) => HttpError::Conflict(msg),
                    RepositoryError::AlreadyExists(msg) => HttpError::Conflict(msg),
                    RepositoryError::ForeignKey(msg) => {
                        HttpError::BadRequest(format!("Foreign key violation: {msg}"))
                    }
                    _ => HttpError::Internal("Repository error".into()),
                },

                ServiceError::Jwt(err) => HttpError::Unauthorized(format!("JWT error: {err}")),

                ServiceError::Kafka(err) => {
                    HttpError::ServiceUnavailable(format!("Kafka error: {err}"))
                }

                ServiceError::Internal(msg) | ServiceError::Custom(msg) => HttpError::Internal(msg),

                ServiceError::Bcrypt(_) => {
                    HttpError::Internal("Internal authentication error".into())
                }

                ServiceError::TokenExpired => HttpError::Unauthorized("Token expired".into()),

                ServiceError::InvalidTokenType => {
                    HttpError::Unauthorized("Invalid token type".into())
                }
            },

            AppErrorGrpc::Unhandled(msg) => HttpError::Internal(msg),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            HttpError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            HttpError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            HttpError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            HttpError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            HttpError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            HttpError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            status: "error".into(),
            message: msg,
        });

        (status, body).into_response()
    }
}
