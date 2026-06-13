//! Emoji + 通知列表

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use xhh_core::api::{emoji::list_emojis, notification::list_all_messages};

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct NotifyParams {
    #[serde(default = "default_offset")]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
fn default_offset() -> u32 {
    0
}
fn default_limit() -> u32 {
    10
}

/// GET /api/emoji
pub async fn emoji(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = list_emojis(&c).await?;
    Ok(Json(v))
}

/// GET /api/notifications?offset=0&limit=10
pub async fn notifications(
    State(state): State<AppState>,
    Query(q): Query<NotifyParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = list_all_messages(&c, q.offset, q.limit).await?;
    Ok(Json(v))
}
