//! HTTP API 统一错误类型

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("未登录或凭据失效")]
    NotLoggedIn,

    #[error("配置错误: {0}")]
    Config(String),

    #[error("API 调用失败: {0}")]
    Core(#[from] xhh_core::Error),

    #[error("Agent 调用失败: {0}")]
    Agent(#[from] xhh_agent::Error),

    #[error("请求体解析失败: {0}")]
    BadRequest(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::NotLoggedIn => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::Core(e) => {
                // 把 xhh_core::Error 映射到合适的 HTTP 状态
                match e {
                    xhh_core::Error::AuthExpired | xhh_core::Error::NotLoggedIn => {
                        (StatusCode::UNAUTHORIZED, self.to_string())
                    }
                    xhh_core::Error::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()),
                    _ => (StatusCode::BAD_GATEWAY, self.to_string()),
                }
            }
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::Agent(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        tracing::warn!(%status, msg = %msg, "API 错误");
        (
            status,
            Json(ErrorResponse {
                error: status.canonical_reason().unwrap_or("error").to_string(),
                message: msg,
            }),
        )
            .into_response()
    }
}

/// Result 别名
pub type ApiResult<T> = std::result::Result<T, ApiError>;
