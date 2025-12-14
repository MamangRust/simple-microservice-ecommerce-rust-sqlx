use crate::abstract_trait::session::DynSessionMiddleware;
use crate::{
    abstract_trait::product::DynProductGrpcClient,
    domain::{
        requests::product::{CreateProductRequest, FindAllProducts, UpdateProductRequest},
        response::{
            api::{ApiResponse, ApiResponsePagination},
            product::{ProductResponse, ProductResponseDeleteAt},
        },
    },
};
use crate::{
    middleware::{
        jwt::auth_middleware, session::session_middleware,
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
    path = "/api/products",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(FindAllProducts),
    responses(
        (status = 200, description = "List of products", body = ApiResponsePagination<Vec<ProductResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_products(
    Extension(service): Extension<DynProductGrpcClient>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllProducts>,
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
    path = "/api/products/active",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(FindAllProducts),
    responses(
        (status = 200, description = "List of active products", body = ApiResponsePagination<Vec<ProductResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_active_products(
    Extension(service): Extension<DynProductGrpcClient>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllProducts>,
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
    path = "/api/products/trashed",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(FindAllProducts),
    responses(
        (status = 200, description = "List of soft-deleted products", body = ApiResponsePagination<Vec<ProductResponseDeleteAt>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_trashed_products(
    Extension(service): Extension<DynProductGrpcClient>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
    Query(params): Query<FindAllProducts>,
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
    path = "/api/products/{id}",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product details", body = ApiResponse<ProductResponse>),
        (status = 404, description = "Product not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_product(
    Extension(service): Extension<DynProductGrpcClient>,
    Path(id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(session): Extension<DynSessionMiddleware>,
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

    let response = service.find_by_id(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/products",
    tag = "Product",
    security(("bearer_auth" = [])),
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created", body = ApiResponse<ProductResponse>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_product(
    Extension(service): Extension<DynProductGrpcClient>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateProductRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.create_product(&body).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/products/{id}",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Product ID")),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Product updated", body = ApiResponse<ProductResponse>),
        (status = 404, description = "Product not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_product(
    Extension(service): Extension<DynProductGrpcClient>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateProductRequest>,
) -> Result<impl IntoResponse, HttpError> {
    body.id = Some(id);
    let response = service.update_product(&body).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/api/products/trash/{id}",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product soft-deleted", body = ApiResponse<ProductResponseDeleteAt>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trash_product_handler(
    Extension(service): Extension<DynProductGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.trash_product(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/products/restore/{id}",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product restored", body = ApiResponse<ProductResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_product_handler(
    Extension(service): Extension<DynProductGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.restore_product(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/api/products/delete/{id}",
    tag = "Product",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product permanently deleted", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_product(
    Extension(service): Extension<DynProductGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_product(id).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Product deleted permanently"
        })),
    ))
}

#[utoipa::path(
    put,
    path = "/api/products/restore-all",
    tag = "Product",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed products restored", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_all_product_handler(
    Extension(service): Extension<DynProductGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.restore_all_product().await?;
    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "All products restored successfully"
        })),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/products/delete-all",
    tag = "Product",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed products permanently deleted", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_all_product_handler(
    Extension(service): Extension<DynProductGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_all_product().await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "All trashed products deleted permanently"
        })),
    ))
}

pub fn product_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/products", get(get_products))
        .route("/api/products/active", get(get_active_products))
        .route("/api/products/trashed", get(get_trashed_products))
        .route("/api/products/{id}", get(get_product))
        .route("/api/products", post(create_product))
        .route("/api/products/{id}", put(update_product))
        .route("/api/products/trash/{id}", delete(trash_product_handler))
        .route("/api/products/restore/{id}", put(restore_product_handler))
        .route(
            "/api/products/restore-all",
            put(restore_all_product_handler),
        )
        .route("/api/products/delete/{id}", delete(delete_product))
        .route(
            "/api/products/delete-all",
            delete(delete_all_product_handler),
        )
        .route_layer(middleware::from_fn(session_middleware))
        .route_layer(middleware::from_fn(auth_middleware))
        .layer(Extension(app_state.di_container.product_clients.clone()))
        .layer(Extension(app_state.di_container.role_clients.clone()))
        .layer(Extension(app_state.session.clone()))
        .layer(Extension(app_state.jwt_config.clone()))
}
