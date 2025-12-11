use crate::{
    abstract_trait::{role::DynRoleGrpcClient, session::DynSessionMiddleware},
    domain::{
        requests::role::{CreateRoleRequest, FindAllRole, UpdateRoleRequest},
        response::{
            api::{ApiResponse, ApiResponsePagination},
            role::{RoleResponse, RoleResponseDeleteAt},
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
    routing::{delete, get, post, put},
};
use serde_json::json;
use shared::errors::HttpError;
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/roles",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(FindAllRole),
    responses(
        (status = 200, description = "List of roles", body = ApiResponsePagination<Vec<RoleResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_roles(
    Extension(service): Extension<DynRoleGrpcClient>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllRole>,
) -> Result<impl IntoResponse, HttpError> {
    let key = format!("session:{user_id}");

    let current_session = session
        .get_session(&key)
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
    path = "/api/roles/active",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(FindAllRole),
    responses(
        (status = 200, description = "List of active roles", body = ApiResponsePagination<Vec<RoleResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_active_roles(
    Extension(service): Extension<DynRoleGrpcClient>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllRole>,
) -> Result<impl IntoResponse, HttpError> {
    let key = format!("session:{user_id}");

    let current_session = session
        .get_session(&key)
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
    path = "/api/roles/trashed",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(FindAllRole),
    responses(
        (status = 200, description = "List of soft-deleted roles", body = ApiResponsePagination<Vec<RoleResponseDeleteAt>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_trashed_roles(
    Extension(service): Extension<DynRoleGrpcClient>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllRole>,
) -> Result<impl IntoResponse, HttpError> {
    let key = format!("session:{user_id}");

    let current_session = session
        .get_session(&key)
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
    path = "/api/roles/{id}",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Role ID")),
    responses(
        (status = 200, description = "Role details", body = ApiResponse<RoleResponse>),
        (status = 404, description = "Role not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_role(
    Extension(service): Extension<DynRoleGrpcClient>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_by_id(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/roles",
    tag = "Role",
    security(("bearer_auth" = [])),
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created", body = ApiResponse<RoleResponse>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_role(
    Extension(service): Extension<DynRoleGrpcClient>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateRoleRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.create_role(&body).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/roles/{id}",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Role ID")),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated", body = ApiResponse<RoleResponse>),
        (status = 404, description = "Role not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_role(
    Extension(service): Extension<DynRoleGrpcClient>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateRoleRequest>,
) -> Result<impl IntoResponse, HttpError> {
    body.id = Some(id);
    let response = service.update_role(&body).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/api/roles/trash/{id}",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Role ID")),
    responses(
        (status = 200, description = "Role soft-deleted", body = ApiResponse<RoleResponseDeleteAt>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trash_role_handler(
    Extension(service): Extension<DynRoleGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.trash_role(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/roles/restore/{id}",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Role ID")),
    responses(
        (status = 200, description = "Role restored", body = ApiResponse<RoleResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_role_handler(
    Extension(service): Extension<DynRoleGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.restore_role(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/api/roles/delete/{id}",
    tag = "Role",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Role ID")),
    responses(
        (status = 200, description = "Role permanently deleted", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_role(
    Extension(service): Extension<DynRoleGrpcClient>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_ole(id).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Role deleted permanently"
        })),
    ))
}

#[utoipa::path(
    put,
    path = "/api/roles/restore-all",
    tag = "Role",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed roles restored", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_all_role_handler(
    Extension(service): Extension<DynRoleGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.restore_all_role().await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
           "message": "All roles restored successfully"
        })),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/roles/delete-all",
    tag = "Role",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed roles permanently deleted", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_all_role_handler(
    Extension(service): Extension<DynRoleGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_all_role().await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
           "message": "All trashed roles deleted permanently"
        })),
    ))
}

pub fn roles_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/roles", get(get_roles))
        .route("/api/roles/active", get(get_active_roles))
        .route("/api/roles/trashed", get(get_trashed_roles))
        .route("/api/roles/{id}", get(get_role))
        .route("/api/roles", post(create_role))
        .route("/api/roles/{id}", put(update_role))
        .route("/api/roles/trash/{id}", delete(trash_role_handler))
        .route("/api/roles/restore/{id}", put(restore_role_handler))
        .route("/api/roles/restore-all", put(restore_all_role_handler))
        .route("/api/roles/delete/{id}", delete(delete_role))
        .route("/api/roles/delete-all", delete(delete_all_role_handler))
        .route_layer(middleware::from_fn(session_middleware))
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(rate_limit_middleware))
        .layer(Extension(app_state.di_container.role_clients.clone()))
        .layer(Extension(app_state.rate_limit.clone()))
        .layer(Extension(app_state.session.clone()))
        .layer(Extension(app_state.jwt_config.clone()))
}
