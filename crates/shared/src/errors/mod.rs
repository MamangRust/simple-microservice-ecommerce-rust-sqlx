mod error;
mod grpc;
mod http;
mod repository;
mod service;

pub use self::error::ErrorResponse;
pub use self::grpc::AppErrorGrpc;
pub use self::http::HttpError;
pub use self::repository::RepositoryError;
pub use self::service::ServiceError;

use tonic::Status;

pub fn grpc_status_to_service_error(status: Status) -> ServiceError {
    let app_error = AppErrorGrpc::from(status);
    match app_error {
        AppErrorGrpc::Service(service_err) => service_err,
        AppErrorGrpc::Unhandled(msg) => ServiceError::Internal(msg),
    }
}
