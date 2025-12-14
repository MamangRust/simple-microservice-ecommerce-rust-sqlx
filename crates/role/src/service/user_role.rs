use crate::{
    abstract_trait::user_role::{
        repository::DynUserRoleCommandRepository, service::UserRoleCommandServiceTrait,
    },
    domain::{
        requests::user_role::CreateUserRoleRequest,
        response::{api::ApiResponse, user_role::UserRoleResponse},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use shared::{
    errors::ServiceError,
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use tokio::time::Instant;
use tonic::Request;
use tracing::{error, info};

#[derive(Clone)]
pub struct UserRoleCommandService {
    pub command: DynUserRoleCommandRepository,
    pub metrics: Metrics,
}

impl UserRoleCommandService {
    pub fn new(command: DynUserRoleCommandRepository) -> Result<Self> {
        let metrics = Metrics::new(global::meter("user-role-service"));

        Ok(Self { command, metrics })
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-role-service")
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

        self.metrics.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl UserRoleCommandServiceTrait for UserRoleCommandService {
    async fn assign_role_to_user(
        &self,
        req: &CreateUserRoleRequest,
    ) -> Result<ApiResponse<UserRoleResponse>, ServiceError> {
        info!(
            "🏗️ Assigning role_id={} to user_id={}",
            req.role_id, req.user_id
        );

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "assign_role_to_user",
            vec![
                KeyValue::new("component", "user_role"),
                KeyValue::new("operation", "assign"),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let model = match self.command.assign_role_to_user(req).await {
            Ok(model) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role assigned successfully",
                )
                .await;
                model
            }
            Err(err) => {
                error!(
                    "❌ Failed to assign role_id={} to user_id={}: {:?}",
                    req.role_id, req.user_id, err
                );

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to assign role")
                    .await;

                return Err(ServiceError::Repo(err));
            }
        };

        let response = UserRoleResponse::from(model);

        info!(
            "✅ Successfully assigned role_id={} to user_id={} ",
            response.role_id, response.user_id
        );

        Ok(ApiResponse {
            status: "success".into(),
            message: "Role assigned successfully".into(),
            data: response,
        })
    }
    async fn update_role_to_user(
        &self,
        req: &CreateUserRoleRequest,
    ) -> Result<ApiResponse<UserRoleResponse>, ServiceError> {
        info!(
            "🔧 Updating user_id={} to new role_id={}",
            req.user_id, req.role_id
        );

        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "update_role_to_user",
            vec![
                KeyValue::new("component", "user_role"),
                KeyValue::new("operation", "update"),
            ],
        );

        let mut request = Request::new(req.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let model = match self.command.update_role_to_user(req).await {
            Ok(model) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "Role updated successfully",
                )
                .await;
                model
            }
            Err(err) => {
                error!(
                    "❌ Failed to update role for user_id={}: {:?}",
                    req.user_id, err
                );

                self.complete_tracing_error(&tracing_ctx, method.clone(), "Failed to update role")
                    .await;

                return Err(ServiceError::Repo(err));
            }
        };

        let response = UserRoleResponse::from(model);

        info!(
            "🔄 Updated role for user_id={} to role_id={} ",
            response.user_id, response.role_id
        );

        Ok(ApiResponse {
            status: "success".into(),
            message: "Role updated successfully".into(),
            data: response,
        })
    }
}
