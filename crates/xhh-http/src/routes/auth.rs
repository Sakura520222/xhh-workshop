//! 认证路由：扫码登录 / 登录态检查 / 登出

use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use xhh_core::auth::{get_qr_code, poll_qr_state_once, QrCodeResp, QrLoginSuccess, QrPollResult};
use xhh_core::client::XhhClient;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct QrQuery {
    /// 自定义设备 ID（可选）
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginStatus {
    pub ok: bool,
    pub nickname: String,
    pub heybox_id: String,
}

/// GET /api/auth/qrcode — 获取二维码 URL
#[utoipa::path(
    get,
    path = "/api/auth/qrcode",
    params(("device_id" = Option<String>, Query, description = "自定义设备 ID")),
    responses((status = 200, body = QrCodeResp))
)]
pub async fn get_qrcode(
    State(state): State<AppState>,
    Query(q): Query<QrQuery>,
) -> ApiResult<Json<QrCodeResp>> {
    let device_id = match q.device_id {
        Some(d) if !d.is_empty() => d,
        _ => state
            .snapshot()
            .await
            .map(|s| s.device_id)
            .unwrap_or_default(),
    };
    let anon =
        XhhClient::anonymous(Some(device_id)).map_err(|e| ApiError::Internal(e.to_string()))?;
    let qr = get_qr_code(&anon).await?;
    Ok(Json(qr))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginReq {
    pub raw_query: String,
    pub device_id: String,
}

/// POST /api/auth/login — 同步轮询直到扫码成功或超时（最长 300s）
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginReq,
    responses((status = 200, body = QrLoginSuccess), (status = 401, description = "超时"))
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<QrLoginSuccess>> {
    let anon = XhhClient::anonymous(Some(req.device_id.clone()))
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        if Instant::now() > deadline {
            return Err(ApiError::Internal("扫码超时".into()));
        }
        match poll_qr_state_once(&anon, &req.raw_query, &req.device_id).await? {
            QrPollResult::Waiting { .. } | QrPollResult::Scanned => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            QrPollResult::Success(mut s) => {
                s.config.device_id = req.device_id.clone();
                s.config
                    .save(None)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                // 重新加载到 state
                state.refresh().await?;
                return Ok(Json(s));
            }
        }
    }
}

/// GET /api/auth/status — 当前登录态
#[utoipa::path(get, path = "/api/auth/status", responses((status = 200, body = LoginStatus)))]
pub async fn status(State(state): State<AppState>) -> ApiResult<Json<LoginStatus>> {
    match state.snapshot().await {
        Some(s) => Ok(Json(LoginStatus {
            ok: true,
            nickname: s.nickname,
            heybox_id: s.heybox_id,
        })),
        None => Ok(Json(LoginStatus {
            ok: false,
            nickname: String::new(),
            heybox_id: String::new(),
        })),
    }
}

/// POST /api/auth/logout — 登出
#[utoipa::path(post, path = "/api/auth/logout", responses((status = 200)))]
pub async fn logout(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    // 写入空配置文件，标记登出
    let empty = xhh_core::config::Config::default();
    empty
        .save(None)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    state.clear().await;
    Ok(Json(serde_json::json!({"ok": true})))
}
