use crate::{
    abstract_trait::{role::DynRoleGrpcClient, session::DynSessionMiddleware},
    domain::response::session::Session,
};
use axum::{
    Extension, Json,
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use chrono::Duration;
use shared::errors::ErrorResponse;

pub async fn session_middleware(
    Extension(role_client): Extension<DynRoleGrpcClient>,
    Extension(session_service): Extension<DynSessionMiddleware>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = match req.extensions().get::<i32>() {
        Some(id) => *id,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    status: "fail".to_string(),
                    message: "Missing user_id in request context".to_string(),
                }),
            ));
        }
    };

    let roles = match role_client.find_by_user_id(user_id).await {
        Ok(resp) => resp.data.into_iter().map(|r| r.role_name).collect(),
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    status: "fail".to_string(),
                    message: "Failed to fetch roles".to_string(),
                }),
            ));
        }
    };

    let session = Session {
        user_id: user_id.to_string(),
        email: "".to_owned(),
        roles,
    };

    let key = format!("session:{user_id}");

    session_service.create_session(&key, &session, Duration::minutes(30));

    req.extensions_mut().insert(session.clone());

    Ok(next.run(req).await)
}
