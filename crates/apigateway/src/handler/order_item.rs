use crate::{
    abstract_trait::order_item::DynOrderItemGrpcClient,
    domain::{
        requests::order_item::FindAllOrderItems,
        response::{
            api::{ApiResponse, ApiResponsePagination},
            order_item::{OrderItemResponse, OrderItemResponseDeleteAt},
        },
    },
};
use crate::{middleware::jwt, state::AppState};
use axum::{
    Json,
    extract::{Extension, Path, Query},
    middleware,
    response::IntoResponse,
    routing::get,
};
use shared::errors::HttpError;
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/order-items",
    tag = "Order Item",
    security(("bearer_auth" = [])),
    params(FindAllOrderItems),
    responses(
        (status = 200, description = "List of order items", body = ApiResponsePagination<Vec<OrderItemResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_order_items(
    Extension(service): Extension<DynOrderItemGrpcClient>,
    Query(params): Query<FindAllOrderItems>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_all(&params).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/order-items/active",
    tag = "Order Item",
    security(("bearer_auth" = [])),
    params(FindAllOrderItems),
    responses(
        (status = 200, description = "List of active order items", body = ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_active_order_items(
    Extension(service): Extension<DynOrderItemGrpcClient>,
    Query(params): Query<FindAllOrderItems>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_by_active(&params).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/order-items/trashed",
    tag = "Order Item",
    security(("bearer_auth" = [])),
    params(FindAllOrderItems),
    responses(
        (status = 200, description = "List of soft-deleted order items", body = ApiResponsePagination<Vec<OrderItemResponseDeleteAt>>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_trashed_order_items(
    Extension(service): Extension<DynOrderItemGrpcClient>,
    Query(params): Query<FindAllOrderItems>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_by_trashed(&params).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/order-items/{order_item}",
    tag = "Order Item",
    security(("bearer_auth" = [])),
    params(
        ("order_id" = i32, Path, description = "ID of the order to fetch items for")
    ),
    responses(
        (status = 200, description = "List of items for a specific order", body = ApiResponse<Vec<OrderItemResponse>>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found or has no items"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_items_by_order_id(
    Extension(service): Extension<DynOrderItemGrpcClient>,
    Path(order_id): Path<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.find_order_item_by_order(order_id).await?;
    Ok(Json(response))
}

pub fn order_item_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/order-items", get(get_order_items))
        .route("/api/order-items/active", get(get_active_order_items))
        .route("/api/order-items/trashed", get(get_trashed_order_items))
        .route("/api/order-items/{order_item}", get(get_items_by_order_id))
        .route_layer(middleware::from_fn(jwt::auth))
        .layer(Extension(app_state.di_container.order_clients.clone()))
        .layer(Extension(app_state.jwt_config.clone()))
}
