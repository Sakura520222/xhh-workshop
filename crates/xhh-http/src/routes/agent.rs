//! Agent 路由（chat / auto-post）

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use xhh_agent::config::AgentConfig;
use xhh_agent::runner::{AgentResult, AgentRunner};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ChatReq {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AutoPostReq {
    /// 用户自然语言指令
    pub intent: String,
    /// 提示话题标签
    #[serde(default)]
    pub hashtags: Vec<String>,
}

async fn build_runner(state: &AppState) -> ApiResult<AgentRunner> {
    let cfg = AgentConfig::load(None).map_err(|e| ApiError::Config(e.to_string()))?;
    if state.snapshot().await.is_none() {
        return Err(ApiError::NotLoggedIn);
    }
    let client = state.require_client().await?;
    AgentRunner::from_config(cfg, client).map_err(ApiError::Agent)
}

/// POST /api/agent/chat — Agent 通用对话
#[utoipa::path(
    post,
    path = "/api/agent/chat",
    request_body = ChatReq,
    responses(
        (status = 200, description = "Agent 运行结果", body = xhh_agent::runner::AgentResult),
        (status = 401, description = "未登录或未配置 Agent"),
    )
)]
pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatReq>,
) -> ApiResult<Json<AgentResult>> {
    let mut runner = build_runner(&state).await?;
    let r = runner.chat(&req.message).await?;
    Ok(Json(r))
}

/// POST /api/agent/auto-post — Agent 自动发帖（板块/话题由 LLM 自主决定）
#[utoipa::path(
    post,
    path = "/api/agent/auto-post",
    request_body = AutoPostReq,
    responses(
        (status = 200, description = "Agent 运行结果", body = xhh_agent::runner::AgentResult),
        (status = 401, description = "未登录或未配置 Agent"),
    )
)]
pub async fn auto_post(
    State(state): State<AppState>,
    Json(req): Json<AutoPostReq>,
) -> ApiResult<Json<AgentResult>> {
    let mut runner = build_runner(&state).await?;
    let r = runner.auto_post(&req.intent, &req.hashtags).await?;
    Ok(Json(r))
}
