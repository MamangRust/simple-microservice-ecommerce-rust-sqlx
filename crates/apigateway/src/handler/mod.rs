mod auth;
mod order;
mod order_item;
mod product;
mod role;
mod user;

use crate::state::AppState;
use anyhow::Result;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use prometheus_client::encoding::text::encode;
use shared::utils::shutdown_signal;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::limit::RequestBodyLimitLayer;
use utoipa::{Modify, OpenApi, openapi::security::SecurityScheme};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

pub use self::auth::auth_routes;
pub use self::order::order_routes;
pub use self::order_item::order_item_routes;
pub use self::product::product_routes;
pub use self::role::roles_routes;
pub use self::user::user_routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login_user_handler,
        auth::get_me_handler,
        auth::register_user_handler,
        auth::verify_code_handler,
        auth::forgot_password_handler,
        auth::reset_password_handler,
        auth::refresh_token_handler,

        user::get_users,
        user::get_active_users,
        user::get_trashed_users,
        user::get_user,
        user::update_user,
        user::trash_user_handler,
        user::restore_user_handler,
        user::delete_user,
        user::restore_all_user_handler,
        user::delete_all_user_handler,

        role::get_roles,
        role::get_active_roles,
        role::get_trashed_roles,
        role::get_role,
        role::create_role,
        role::update_role,
        role::trash_role_handler,
        role::restore_role_handler,
        role::delete_role,
        role::restore_all_role_handler,
        role::delete_all_role_handler,



        product::get_products,
        product::get_active_products,
        product::get_trashed_products,
        product::get_product,
        product::create_product,
        product::update_product,
        product::trash_product_handler,
        product::restore_product_handler,
        product::delete_product,
        product::restore_all_product_handler,
        product::delete_all_product_handler,


        order::get_orders,
        order::get_active_orders,
        order::get_trashed_orders,
        order::get_order,
        order::create_order,
        order::update_order,
        order::trash_order_handler,
        order::restore_order_handler,
        order::delete_order,
        order::restore_all_order_handler,
        order::delete_all_order_handler,

        order_item::get_order_items,
        order_item::get_active_order_items,
        order_item::get_trashed_order_items,
        order_item::get_items_by_order_id,

    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Role", description = "Role endpoints"),
        (name = "User", description = "User endpoints"),
        (name = "Product", description = "Product endpoints"),
        (name = "Order", description = "Order endpoints"),
        (name = "Order-item", description = "Order Item endpoints"),
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();

        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            )),
        );
    }
}

pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut buffer = String::new();

    let registry = state.registry.lock().await;

    if let Err(e) = encode(&mut buffer, &registry) {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Failed to encode metrics: {e}")))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(Body::from(buffer))
        .unwrap()
}

pub struct AppRouter;

impl AppRouter {
    pub async fn serve(port: u16, app_state: AppState) -> Result<()> {
        let shared_state = Arc::new(app_state);

        let api_router = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .route("/metrics", get(metrics_handler))
            .with_state(shared_state.clone())
            .merge(auth_routes(shared_state.clone()))
            .merge(user_routes(shared_state.clone()))
            .merge(roles_routes(shared_state.clone()))
            .merge(product_routes(shared_state.clone()))
            .merge(order_routes(shared_state.clone()))
            .merge(order_item_routes(shared_state.clone()));

        let router_with_layers = api_router
            .layer(DefaultBodyLimit::disable())
            .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024));

        let (app_router, api) = router_with_layers.split_for_parts();

        let app = app_router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(&addr).await?;

        println!("🚀 Server running on http://{}", listener.local_addr()?);
        println!("📚 API Documentation available at:");
        println!("   📖 Swagger UI: http://localhost:{port}/swagger-ui");
        println!("   📊 Metrics: http://localhost:{port}/metrics");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();

        Ok(())
    }
}
