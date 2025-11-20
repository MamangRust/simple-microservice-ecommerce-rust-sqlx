use crate::{
    abstract_trait::user::DynUserGrpcClient,
    domain::{
        requests::user::{FindAllUsers, UpdateUserRequest},
        response::{
            api::{ApiResponse, ApiResponsePagination},
            user::{UserResponse, UserResponseDeleteAt},
        },
    },
};
use crate::{
    middleware::{jwt, validate::SimpleValidatedJson},
    state::AppState,
};
use axum::{
    Json,
    extract::{Extension, Path, Query},
    middleware,
    response::IntoResponse,
    routing::{delete, get, put},
};
use serde_json::json;
use shared::errors::HttpError;
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "User",
    security(("bearer_auth" = [])),
    params(FindAllUsers),
    responses(
        (status = 200, description = "List of users", body = ApiResponsePagination<Vec<UserResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_users(
    Extension(service): Extension<DynUserGrpcClient>,
    Query(params): Query<FindAllUsers>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_all(&params).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/users/active",
    tag = "User",
    security(("bearer_auth" = [])),
    params(FindAllUsers),
    responses(
        (status = 200, description = "List of active users", body = ApiResponsePagination<Vec<UserResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_active_users(
    Extension(service): Extension<DynUserGrpcClient>,
    Query(params): Query<FindAllUsers>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_active(&params).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/users/trashed",
    tag = "User",
    security(("bearer_auth" = [])),
    params(FindAllUsers),
    responses(
        (status = 200, description = "List of soft-deleted users", body = ApiResponsePagination<Vec<UserResponseDeleteAt>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_trashed_users(
    Extension(service): Extension<DynUserGrpcClient>,
    Query(params): Query<FindAllUsers>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_trashed(&params).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "User",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "User ID")),
    responses(
        (status = 200, description = "User details", body = ApiResponse<UserResponse>),
        (status = 404, description = "User not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_user(
    Extension(service): Extension<DynUserGrpcClient>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_by_id(id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "User",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = ApiResponse<UserResponse>),
        (status = 404, description = "User not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_user(
    Extension(service): Extension<DynUserGrpcClient>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateUserRequest>,
) -> Result<impl IntoResponse, HttpError> {
    body.user_id = Some(id);
    let response = service.update_user(&body).await?;
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/users/trash/{id}",
    tag = "User",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "User ID")),
    responses(
        (status = 200, description = "User soft-deleted", body = ApiResponse<UserResponseDeleteAt>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trash_user_handler(
    Extension(service): Extension<DynUserGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.trash_user(id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/users/restore/{id}",
    tag = "User",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "User ID")),
    responses(
        (status = 200, description = "User restored", body = ApiResponse<UserResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_user_handler(
    Extension(service): Extension<DynUserGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.restore_user(id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/users/delete/{id}",
    tag = "User",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "User ID")),
    responses(
        (status = 200, description = "User permanently deleted", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_user(
    Extension(service): Extension<DynUserGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_user(id).await?;
    Ok(Json(json!({
        "status": "success",
        "message": "User deleted permanently"
    })))
}

#[utoipa::path(
    put,
    path = "/api/users/restore-all",
    tag = "User",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed users restored", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_all_user_handler(
    Extension(service): Extension<DynUserGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.restore_all_user().await?;
    Ok(Json(json!({
        "status": "success",
        "message": "All users restored successfully"
    })))
}

#[utoipa::path(
    delete,
    path = "/api/users/delete-all",
    tag = "User",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed users permanently deleted", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_all_user_handler(
    Extension(service): Extension<DynUserGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_all_user().await?;
    Ok(Json(json!({
        "status": "success",
        "message": "All trashed users deleted permanently"
    })))
}

pub fn user_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/users", get(get_users))
        .route("/api/users/active", get(get_active_users))
        .route("/api/users/trashed", get(get_trashed_users))
        .route("/api/users/{id}", get(get_user))
        .route("/api/users/{id}", put(update_user))
        .route("/api/users/trash/{id}", delete(trash_user_handler))
        .route("/api/users/restore/{id}", put(restore_user_handler))
        .route("/api/users/restore-all", put(restore_all_user_handler))
        .route("/api/users/delete/{id}", delete(delete_user))
        .route("/api/users/delete-all", delete(delete_all_user_handler))
        .route_layer(middleware::from_fn(jwt::auth))
        .layer(Extension(app_state.di_container.user_clients.clone()))
        .layer(Extension(app_state.jwt_config.clone()))
}
