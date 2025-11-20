use crate::{
    abstract_trait::{
        DynJwtService, DynRefreshTokenCommandRepository, DynTokenService, DynUserQueryRepository,
        IdentityServiceTrait,
    },
    cache::CacheStore,
    domain::{
        requests::UpdateRefreshToken,
        responses::{ApiResponse, TokenResponse, UserResponse, to_user_response},
    },
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
use chrono::Duration;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::Request;
use tracing::{error, info};

pub struct IdentityService {
    refresh_token_command: DynRefreshTokenCommandRepository,
    token: DynJwtService,
    token_service: DynTokenService,
    user_query: DynUserQueryRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

pub struct IdentityServiceDeps {
    pub refresh_token_command: DynRefreshTokenCommandRepository,
    pub token: DynJwtService,
    pub token_service: DynTokenService,
    pub user_query: DynUserQueryRepository,
    pub metrics: Arc<Mutex<Metrics>>,
    pub registry: Arc<Mutex<Registry>>,
    pub cache_store: Arc<CacheStore>,
}

impl IdentityService {
    pub async fn new(deps: IdentityServiceDeps) -> Self {
        let IdentityServiceDeps {
            refresh_token_command,
            token,
            token_service,
            user_query,
            metrics,
            registry,
            cache_store,
        } = deps;

        registry.lock().await.register(
            "identity_service_request_counter",
            "Total number of requests to the IdentityService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.lock().await.register(
            "identity_service_request_duration",
            "Histogram of request durations for the IdentityService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            refresh_token_command,
            token,
            token_service,
            user_query,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("identity-service")
    }

    fn inject_trace_context<T>(&self, cx: &Context, request: &mut Request<T>) {
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(cx, &mut MetadataInjector(request.metadata_mut()))
        });
    }

    fn start_tracing(&self, operation_name: &str, attributes: Vec<KeyValue>) -> TracingContext {
        let start_time = Instant::now();
        let tracer = self.get_tracer();
        let mut span = tracer
            .span_builder(operation_name.to_string())
            .with_kind(SpanKind::Server)
            .with_attributes(attributes)
            .start(&tracer);

        info!("Starting operation: {operation_name}");

        span.add_event(
            "Operation started",
            vec![
                KeyValue::new("operation", operation_name.to_string()),
                KeyValue::new("timestamp", start_time.elapsed().as_secs_f64().to_string()),
            ],
        );

        let cx = Context::current_with_span(span);
        TracingContext { cx, start_time }
    }

    async fn complete_tracing_success(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, true, message)
            .await;
    }

    async fn complete_tracing_error(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        error_message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, false, error_message)
            .await;
    }

    async fn complete_tracing_internal(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        is_success: bool,
        message: &str,
    ) {
        let status_str = if is_success { "SUCCESS" } else { "ERROR" };
        let status = if is_success {
            StatusUtils::Success
        } else {
            StatusUtils::Error
        };
        let elapsed = tracing_ctx.start_time.elapsed().as_secs_f64();

        tracing_ctx.cx.span().add_event(
            "Operation completed",
            vec![
                KeyValue::new("status", status_str),
                KeyValue::new("duration_secs", elapsed.to_string()),
                KeyValue::new("message", message.to_string()),
            ],
        );

        if is_success {
            info!("✅ Operation completed successfully: {message}");
        } else {
            error!("❌ Operation failed: {message}");
        }

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl IdentityServiceTrait for IdentityService {
    async fn refresh_token(&self, token: &str) -> Result<ApiResponse<TokenResponse>, ServiceError> {
        let log_msg = "🔄 Attempting to refresh token";
        info!("{log_msg}");

        let method = Method::Post;
        let tracing_ctx =
            self.start_tracing("RefreshToken", vec![KeyValue::new("component", "auth")]);

        let mut request = Request::new(token);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let user_id = match self.token.verify_token(token, "refresh") {
            Ok(uid) => uid,
            Err(ServiceError::TokenExpired) => {
                let _ = self
                    .refresh_token_command
                    .delete_token(token.to_string())
                    .await;

                let _ = self
                    .cache_store
                    .delete_from_cache(&format!("auth:refresh:{token}"));

                self.complete_tracing_error(&tracing_ctx, method, "Token expired")
                    .await;

                return Err(ServiceError::TokenExpired);
            }
            Err(e) => {
                error!("❌ Invalid token: {:?}", e);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Invalid token")
                    .await;
                return Err(ServiceError::Internal("invalid token".to_string()));
            }
        };

        if let Err(e) = self
            .refresh_token_command
            .delete_token(token.to_string())
            .await
        {
            error!("❌ Failed to delete old refresh token: {:?}", e);
            self.complete_tracing_error(
                &tracing_ctx,
                method.clone(),
                "Failed to delete old refresh token",
            )
            .await;
            return Err(ServiceError::from(e));
        }

        let access_token = match self.token_service.create_access_token(user_id as i32).await {
            Ok(token) => token,
            Err(e) => {
                error!("❌ Failed to generate access token: {:?}", e);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate access token",
                )
                .await;
                return Err(e);
            }
        };

        let refresh_token = match self
            .token_service
            .create_refresh_token(user_id as i32)
            .await
        {
            Ok(token) => token,
            Err(e) => {
                error!("❌ Failed to generate refresh token: {:?}", e);
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "Failed to generate refresh token",
                )
                .await;
                return Err(e);
            }
        };

        let expiry = chrono::Utc::now() + chrono::Duration::hours(24);

        let update_req = &UpdateRefreshToken {
            user_id: user_id as i32,
            token: refresh_token.clone(),
            expired_date: expiry.naive_utc().to_string(),
        };

        if let Err(e) = self.refresh_token_command.update(update_req).await {
            error!("❌ Failed to update refresh token: {:?}", e);
            self.complete_tracing_error(&tracing_ctx, method, "Failed to update refresh token")
                .await;
            return Err(ServiceError::Internal(
                "Failed to update refresh token".into(),
            ));
        }

        self.cache_store.set_to_cache(
            &format!("auth:refresh:{refresh_token}"),
            &user_id,
            chrono::Duration::hours(24),
        );

        self.complete_tracing_success(&tracing_ctx, method, "Token refreshed successfully")
            .await;

        Ok(ApiResponse {
            status: "success".into(),
            message: "token refreshed".into(),
            data: TokenResponse {
                access_token,
                refresh_token,
            },
        })
    }
    async fn get_me(&self, id: i32) -> Result<ApiResponse<Option<UserResponse>>, ServiceError> {
        let log_msg = format!("👤 Fetching user profile | User ID: {id}");
        info!("{log_msg}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetMe",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let cache_key = format!("auth:getme:{id}");

        if let Some(cached_user) = self.cache_store.get_from_cache::<UserResponse>(&cache_key) {
            info!("✅ Cache hit for user: {id}");
            self.complete_tracing_success(&tracing_ctx, method, "User fetched from cache")
                .await;
            return Ok(ApiResponse {
                status: "success".into(),
                message: "user fetched successfully (from cache)".into(),
                data: Some(cached_user),
            });
        }

        let user = match self.user_query.find_by_id(id).await {
            Ok(user) => user,
            Err(err) => {
                error!("❌ Failed to fetch user from DB: {:?}", err);
                self.complete_tracing_error(&tracing_ctx, method.clone(), "Database error")
                    .await;
                return Err(ServiceError::Repo(err));
            }
        };

        let user_response = to_user_response(user);

        if let Some(ref user_resp) = user_response {
            self.cache_store
                .set_to_cache(&cache_key, user_resp, Duration::minutes(30))
        }

        self.complete_tracing_success(&tracing_ctx, method, "User profile fetched")
            .await;

        Ok(ApiResponse {
            status: "success".into(),
            message: "user fetched successfully".into(),
            data: user_response,
        })
    }
}
