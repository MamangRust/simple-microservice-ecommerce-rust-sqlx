use crate::{
    abstract_trait::{session::DynSessionMiddleware, user::DynUserGrpcClient},
    domain::{
        requests::user::{FindAllUsers, UpdateUserRequest},
        response::{
            api::{ApiResponse, ApiResponsePagination},
            user::{UserResponse, UserResponseDeleteAt},
        },
    },
};
use crate::{
    middleware::{
        jwt::auth_middleware, rate_limit::rate_limit_middleware, session::session_middleware,
        validate::SimpleValidatedJson,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
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
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllUsers>,
) -> Result<impl IntoResponse, HttpError> {
    let key = format!("session:{user_id}");

    let current_session = session
        .get_session(&key)
        .await
        .ok_or_else(|| HttpError::Unauthorized("Session expired or not found".to_string()))?;

    if !current_session
        .roles
        .iter()
        .any(|r| r == "ROLE_ADMIN" || r == "ROLE_MODERATOR")
    {
        return Err(HttpError::Forbidden(
            "Access denied. Required role: ADMIN or MODERATOR".to_string(),
        ));
    }

    let response = service.find_all(&params).await?;
    Ok((StatusCode::OK, Json(response)))
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
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllUsers>,
) -> Result<impl IntoResponse, HttpError> {
    let key = format!("session:{user_id}");

    let current_session = session
        .get_session(&key)
        .await
        .ok_or_else(|| HttpError::Unauthorized("Session expired or not found".to_string()))?;

    if !current_session
        .roles
        .iter()
        .any(|r| r == "ROLE_ADMIN" || r == "ROLE_MODERATOR")
    {
        return Err(HttpError::Forbidden(
            "Access denied. Required role: ADMIN or MODERATOR".to_string(),
        ));
    }

    let response = service.find_active(&params).await?;
    Ok((StatusCode::OK, Json(response)))
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
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllUsers>,
) -> Result<impl IntoResponse, HttpError> {
    let key = format!("session:{user_id}");

    let current_session = session
        .get_session(&key)
        .await
        .ok_or_else(|| HttpError::Unauthorized("Session expired or not found".to_string()))?;

    if !current_session
        .roles
        .iter()
        .any(|r| r == "ROLE_ADMIN" || r == "ROLE_MODERATOR")
    {
        return Err(HttpError::Forbidden(
            "Access denied. Required role: ADMIN or MODERATOR".to_string(),
        ));
    }

    let response = service.find_trashed(&params).await?;
    Ok((StatusCode::OK, Json(response)))
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
    Extension(_user_id): Extension<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_by_id(id).await?;
    Ok((StatusCode::OK, Json(response)))
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
    Ok((StatusCode::OK, Json(response)))
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
    Ok((StatusCode::OK, Json(response)))
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
    Ok((StatusCode::OK, Json(response)))
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

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "User deleted permanently"
        })),
    ))
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

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "All users restored successfully"
        })),
    ))
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

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "All users trashed successfully"
        })),
    ))
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
        .route_layer(middleware::from_fn(session_middleware))
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(rate_limit_middleware))
        .layer(Extension(app_state.di_container.user_clients.clone()))
        .layer(Extension(app_state.di_container.role_clients.clone()))
        .layer(Extension(app_state.rate_limit.clone()))
        .layer(Extension(app_state.session.clone()))
        .layer(Extension(app_state.jwt_config.clone()))
}
