use crate::{
    abstract_trait::auth::AuthGrpcClientTrait,
    domain::{
        requests::{
            auth::{AuthRequest as DomainLoginRequest, RegisterRequest as DomainRegisterRequest},
            reset_token::CreateResetPasswordRequest as DomainResetPasswordRequest,
        },
        response::{api::ApiResponse, token::TokenResponse, user::UserResponse},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use genproto::{
    auth::{
        ApiResponseLogin, ForgotPasswordRequest, GetMeRequest, LoginRequest, RefreshTokenRequest,
        ResetPasswordRequest, VerifyCodeRequest, auth_service_client::AuthServiceClient,
    },
    common::RegisterRequest,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use shared::{
    errors::{AppErrorGrpc, HttpError},
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use tokio::time::Instant;
use tonic::{Request, transport::Channel};
use tracing::{error, info};

#[derive(Debug)]
pub struct AuthGrpcClientService {
    client: AuthServiceClient<Channel>,
    metrics: Metrics,
}

impl AuthGrpcClientService {
    pub fn new(client: AuthServiceClient<Channel>) -> Result<Self> {
        let metrics = Metrics::new(global::meter("auth-service-client"));

        Ok(Self { client, metrics })
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("auth-service-client")
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
            info!("Operation completed successfully: {message}");
        } else {
            error!("Operation failed: {message}");
        }

        self.metrics.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl AuthGrpcClientTrait for AuthGrpcClientService {
    async fn register_user(
        &self,
        req: &DomainRegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, HttpError> {
        info!("Registering user: {}", req.email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RegisterUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("operation", "register"),
                KeyValue::new("user.email", req.email.clone()),
                KeyValue::new("user.firstname", req.first_name.clone()),
            ],
        );

        let request_data = req.clone();

        let mut request = Request::new(RegisterRequest {
            firstname: req.first_name.clone(),
            lastname: req.last_name.clone(),
            email: req.email.clone(),
            password: req.password.clone(),
            confirm_password: req.confirm_password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.clone().register_user(request).await {
            Ok(response) => {
                info!("gRPC register succeeded");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "gRPC register succeeded",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC register failed: {}: {}",
                    status.code(),
                    status.message()
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "gRPC register failed: {}: {}",
                        status.code(),
                        status.message()
                    ),
                )
                .await;

                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let user_data = match inner.data {
            Some(user_data) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "User registered successfully",
                )
                .await;
                user_data
            }
            None => {
                error!("User data is missing in gRPC response");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    "User data is missing in gRPC response",
                )
                .await;

                return Err(HttpError::Internal(
                    "User data is missing in gRPC response".into(),
                ));
            }
        };

        let domain_user: UserResponse = user_data.into();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        };

        info!("User {} registered successfully", request_data.email);
        Ok(api_response)
    }
    async fn login_user(
        &self,
        input: &DomainLoginRequest,
    ) -> Result<ApiResponse<TokenResponse>, HttpError> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "LoginUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("operation", "login"),
                KeyValue::new("user.email", input.email.clone()),
            ],
        );

        let mut client = self.client.clone();

        let mut request = Request::new(LoginRequest {
            email: input.email.clone(),
            password: input.password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match client.login_user(request).await {
            Ok(response) => {
                info!("gRPC login succeeded");
                self.complete_tracing_success(&tracing_ctx, method.clone(), "gRPC login succeeded")
                    .await;
                response
            }
            Err(status) => {
                error!("gRPC login failed: {}: {}", status.code(), status.message());
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("gRPC login failed: {}: {}", status.code(), status.message()),
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner: ApiResponseLogin = response.into_inner();

        let proto_token = match inner.data {
            Some(token) => {
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "User logged in successfully",
                )
                .await;
                token
            }
            None => {
                error!("Token data is missing in gRPC response");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    "Token data is missing in gRPC response",
                )
                .await;
                return Err(HttpError::Internal(
                    "Token data is missing in gRPC response".into(),
                ));
            }
        };

        let domain_token: TokenResponse = proto_token.into();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_token,
        };

        info!("User {} logged in successfully", input.email);
        Ok(api_response)
    }
    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, HttpError> {
        info!("Fetching profile for user ID: {}", id);

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetMe",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let mut request = Request::new(GetMeRequest { id });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.clone().get_me(request).await {
            Ok(response) => {
                info!("gRPC get_me succeeded");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "gRPC get_me succeeded",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC get_me failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "gRPC get_me failed: {}: {}",
                        status.code(),
                        status.message()
                    ),
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let user_data = match inner.data {
            Some(user_data) => user_data,
            None => {
                error!("User data is missing in gRPC response");
                self.complete_tracing_error(
                    &tracing_ctx,
                    method.clone(),
                    "User data is missing in gRPC response",
                )
                .await;

                return Err(HttpError::Internal(
                    "User data is missing in gRPC response".into(),
                ));
            }
        };

        let domain_user: UserResponse = user_data.into();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_user,
        };

        info!("Fetched profile for user ID: {}", id);
        Ok(api_response)
    }
    async fn forgot(&self, email: &str) -> Result<ApiResponse<bool>, HttpError> {
        info!("Initiating forgot password for: {}", email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "ForgotPassword",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", email.to_string()),
            ],
        );

        let mut request = Request::new(ForgotPasswordRequest {
            email: email.to_string(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.clone().forgot_password(request).await {
            Ok(response) => {
                info!("gRPC forgot_password succeeded");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "gRPC forgot_password succeeded",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC forgot_password failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "gRPC forgot_password failed: {}: {}",
                        status.code(),
                        status.message()
                    ),
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: true,
        };

        info!("Forgot password initiated for: {email}");
        Ok(api_response)
    }
    async fn reset_password(
        &self,
        request: &DomainResetPasswordRequest,
    ) -> Result<ApiResponse<bool>, HttpError> {
        info!("Resetting password for token: {}", request.reset_token);

        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "ResetPassword",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("token", request.reset_token.clone()),
            ],
        );

        let mut grpc_request = Request::new(ResetPasswordRequest {
            reset_token: request.reset_token.clone(),
            password: request.password.clone(),
            confirm_password: request.confirm_password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut grpc_request);

        let response = match self.client.clone().reset_password(grpc_request).await {
            Ok(response) => {
                info!("gRPC reset_password succeeded");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "gRPC reset_password succeeded",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC reset_password failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "gRPC reset_password failed: {}: {}",
                        status.code(),
                        status.message()
                    ),
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: true,
        };

        info!(
            "Password reset successfully for token: {}",
            request.reset_token
        );
        Ok(api_response)
    }
    async fn verify_code(&self, code: &str) -> Result<ApiResponse<bool>, HttpError> {
        info!("Verifying code: {}", code);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "VerifyCode",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("code", code.to_string()),
            ],
        );

        let mut request = Request::new(VerifyCodeRequest {
            code: code.to_string(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.clone().verify_code(request).await {
            Ok(response) => {
                info!("gRPC verify_code succeeded");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "gRPC verify_code succeeded",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC verify_code failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "gRPC verify_code failed: {}: {}",
                        status.code(),
                        status.message()
                    ),
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: true,
        };

        info!("Verification code {code} is valid");
        Ok(api_response)
    }
    async fn refresh_token(&self, token: &str) -> Result<ApiResponse<TokenResponse>, HttpError> {
        info!("Refreshing token: {}", token);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RefreshToken",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("token", token.to_string()),
            ],
        );

        let mut request = Request::new(RefreshTokenRequest {
            refresh_token: token.to_string(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let response = match self.client.clone().refresh_token(request).await {
            Ok(response) => {
                info!("gRPC refresh_token succeeded");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method.clone(),
                    "gRPC refresh_token succeeded",
                )
                .await;
                response
            }
            Err(status) => {
                error!(
                    "gRPC refresh_token failed: {}: {}",
                    status.code(),
                    status.message()
                );
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "gRPC refresh_token failed: {}: {}",
                        status.code(),
                        status.message()
                    ),
                )
                .await;
                return Err(AppErrorGrpc::from(status).into());
            }
        };

        let inner = response.into_inner();

        let proto_token = match inner.data {
            Some(data) => data,
            None => {
                return Err(HttpError::Internal(
                    "Missing token data in refresh_token response".into(),
                ));
            }
        };

        let domain_token: TokenResponse = proto_token.into();

        let api_response = ApiResponse {
            status: inner.status,
            message: inner.message,
            data: domain_token,
        };

        info!("Token refreshed successfully");
        Ok(api_response)
    }
}
