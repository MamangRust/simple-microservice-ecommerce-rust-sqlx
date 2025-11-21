use crate::{
    abstract_trait::order::DynOrderGrpcClient,
    domain::{
        requests::order::{CreateOrderRequest, FindAllOrder, UpdateOrderRequest},
        response::{
            api::{ApiResponse, ApiResponsePagination},
            order::{OrderResponse, OrderResponseDeleteAt},
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
    path = "/api/orders",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(FindAllOrder),
    responses(
        (status = 200, description = "List of orders", body = ApiResponsePagination<Vec<OrderResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_orders(
    Extension(service): Extension<DynOrderGrpcClient>,
    Query(params): Query<FindAllOrder>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_all(&params).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/orders/active",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(FindAllOrder),
    responses(
        (status = 200, description = "List of active orders", body = ApiResponsePagination<Vec<OrderResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_active_orders(
    Extension(service): Extension<DynOrderGrpcClient>,
    Query(params): Query<FindAllOrder>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_active(&params).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/orders/trashed",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(FindAllOrder),
    responses(
        (status = 200, description = "List of soft-deleted orders", body = ApiResponsePagination<Vec<OrderResponseDeleteAt>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_trashed_orders(
    Extension(service): Extension<DynOrderGrpcClient>,
    Query(params): Query<FindAllOrder>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_trashed(&params).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/orders/{id}",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Order ID")),
    responses(
        (status = 200, description = "Order details", body = ApiResponse<OrderResponse>),
        (status = 404, description = "Order not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_order(
    Extension(service): Extension<DynOrderGrpcClient>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_by_id(id).await?;
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/orders",
    tag = "Order",
    security(("bearer_auth" = [])),
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created", body = ApiResponse<OrderResponse>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_order(
    Extension(service): Extension<DynOrderGrpcClient>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateOrderRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.create_order(&body).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/orders/{id}",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Order ID")),
    request_body = UpdateOrderRequest,
    responses(
        (status = 200, description = "Order updated", body = ApiResponse<OrderResponse>),
        (status = 404, description = "Order not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_order(
    Extension(service): Extension<DynOrderGrpcClient>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateOrderRequest>,
) -> Result<impl IntoResponse, HttpError> {
    body.order_id = Some(id);
    let response = service.update_order(&body).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/api/orders/trash/{id}",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Order ID")),
    responses(
        (status = 200, description = "Order soft-deleted", body = ApiResponse<OrderResponseDeleteAt>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trash_order_handler(
    Extension(service): Extension<DynOrderGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.trash_order(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/orders/restore/{id}",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Order ID")),
    responses(
        (status = 200, description = "Order restored", body = ApiResponse<OrderResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_order_handler(
    Extension(service): Extension<DynOrderGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.restore_order(id).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    delete,
    path = "/api/orders/delete/{id}",
    tag = "Order",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Order ID")),
    responses(
        (status = 200, description = "Order permanently deleted", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_order(
    Extension(service): Extension<DynOrderGrpcClient>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_order(id).await?;
    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Order deleted permanently"
        })),
    ))
}

#[utoipa::path(
    put,
    path = "/api/orders/restore-all",
    tag = "Order",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed orders restored", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn restore_all_order_handler(
    Extension(service): Extension<DynOrderGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.restore_all_order().await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
           "message": "All orders restored successfully"
        })),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/orders/delete-all",
    tag = "Order",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All trashed orders permanently deleted", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_all_order_handler(
    Extension(service): Extension<DynOrderGrpcClient>,
) -> Result<impl IntoResponse, HttpError> {
    service.delete_all_order().await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "success",
           "message": "All trashed orders deleted permanently"
        })),
    ))
}

pub fn order_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/orders", get(get_orders))
        .route("/api/orders/active", get(get_active_orders))
        .route("/api/orders/trashed", get(get_trashed_orders))
        .route("/api/orders/{id}", get(get_order))
        .route("/api/orders", post(create_order))
        .route("/api/orders/{id}", put(update_order))
        .route("/api/orders/trash/{id}", delete(trash_order_handler))
        .route("/api/orders/restore/{id}", put(restore_order_handler))
        .route("/api/orders/restore-all", put(restore_all_order_handler))
        .route("/api/orders/delete/{id}", delete(delete_order))
        .route("/api/orders/delete-all", delete(delete_all_order_handler))
        .route_layer(middleware::from_fn(session_middleware))
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(rate_limit_middleware))
        .layer(Extension(app_state.di_container.order_clients.clone()))
        .layer(Extension(app_state.di_container.role_clients.clone()))
        .layer(Extension(app_state.rate_limit.clone()))
        .layer(Extension(app_state.session.clone()))
        .layer(Extension(app_state.jwt_config.clone()))
}
