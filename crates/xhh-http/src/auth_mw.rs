//! Bearer Token 中间件
//!
//! 启用 Bearer Token 时，所有 `/api/*` 请求必须带
//! `Authorization: Bearer <token>`，否则返回 401。

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::AppState;

/// 校验 Authorization 头
pub async fn require_bearer(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    tracing::debug!(has_auth = auth.is_some(), "Bearer Token 校验");
    state.check_bearer(auth)?;
    Ok(next.run(req).await)
}
