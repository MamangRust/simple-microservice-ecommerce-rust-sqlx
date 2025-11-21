use crate::{
    abstract_trait::auth::DynAuthGrpcClient,
    domain::{
        requests::{
            auth::{AuthRequest, RegisterRequest},
            reset_token::CreateResetPasswordRequest,
            verify_code::VerifyCodeQuery,
        },
        response::{api::ApiResponse, token::TokenResponse, user::UserResponse},
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
    Extension, Json,
    extract::Query,
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use shared::errors::HttpError;
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

pub async fn health_checker_handler() -> Result<impl IntoResponse, HttpError> {
    const MESSAGE: &str = "JWT Authentication in Rust using Axum, Postgres, and SQLX";

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "success",
            "message": MESSAGE
        })),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<TokenResponse>),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "Auth"
)]
pub async fn login_user_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    SimpleValidatedJson(body): SimpleValidatedJson<AuthRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.login_user(&body).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<UserResponse>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn register_user_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    SimpleValidatedJson(body): SimpleValidatedJson<RegisterRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.register_user(&body).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/auth/forgot-password",
    request_body(content = String, description = "Email to reset password", content_type = "application/json"),
    responses(
        (status = 200, description = "Reset link sent", body = ApiResponse<bool>),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Auth"
)]
pub async fn forgot_password_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    Json(email): Json<String>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.forgot(&email).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/auth/reset-password",
    request_body = CreateResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful", body = ApiResponse<bool>),
        (status = 400, description = "Invalid code or request")
    ),
    tag = "Auth"
)]
pub async fn reset_password_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateResetPasswordRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.reset_password(&body).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/auth/verify-code",
    params(
        ("verify_code" = String, Query, description = "Verification code to verify")
    ),
    responses(
        (status = 200, description = "Code verified", body = ApiResponse<bool>),
        (status = 400, description = "Invalid code")
    ),
    tag = "Auth"
)]
pub async fn verify_code_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    Query(query): Query<VerifyCodeQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.verify_code(&query.verify_code).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body(content = String, description = "Refresh token", content_type = "application/json"),
    responses(
        (status = 200, description = "Token refreshed", body = ApiResponse<TokenResponse>),
        (status = 401, description = "Invalid or expired refresh token")
    ),
    tag = "Auth"
)]
pub async fn refresh_token_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    Json(token): Json<String>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.refresh_token(&token).await?;
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Get Me user", body = ApiResponse<UserResponse>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Auth",
)]
pub async fn get_me_handler(
    Extension(service): Extension<DynAuthGrpcClient>,
    Extension(user_id): Extension<i32>,
) -> Result<impl IntoResponse, HttpError> {
    let response = service.get_me(user_id).await?;
    Ok((StatusCode::OK, Json(response)))
}

pub fn auth_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    let public_routes = OpenApiRouter::new()
        .route("/api/auth/register", post(register_user_handler))
        .route("/api/auth/login", post(login_user_handler))
        .route("/api/auth/verify-code", get(verify_code_handler))
        .route("/api/healthchecker", get(health_checker_handler))
        .layer(Extension(app_state.di_container.auth_clients.clone()));

    let private_routes = OpenApiRouter::new()
        .route("/api/auth/me", get(get_me_handler))
        .route("/api/auth/forgot-password", post(forgot_password_handler))
        .route("/api/auth/reset-password", post(reset_password_handler))
        .route("/api/auth/refresh", post(refresh_token_handler))
        .route_layer(middleware::from_fn(session_middleware))
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(rate_limit_middleware))
        .layer(Extension(app_state.di_container.auth_clients.clone()))
        .layer(Extension(app_state.di_container.role_clients.clone()))
        .layer(Extension(app_state.rate_limit.clone()))
        .layer(Extension(app_state.session.clone()))
        .layer(Extension(app_state.jwt_config.clone()));

    public_routes.merge(private_routes).with_state(app_state)
}
