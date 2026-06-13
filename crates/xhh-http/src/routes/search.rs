//! 通用搜索 / 话题 / 社区 / 热搜

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use xhh_core::api::search::{
    search as api_search, search_community, search_found, search_topic, SearchReq, SearchType,
};

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    /// 综合/内容/游戏/小程序/用户/话题/商品
    #[serde(default = "default_search_type")]
    pub search_type: String,
    #[serde(default = "default_offset")]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// 限定搜索范围到指定社区 topic_id（§39）
    pub topic_id: Option<u32>,
}
fn default_search_type() -> String {
    "综合".to_string()
}
fn default_offset() -> u32 {
    0
}
fn default_limit() -> u32 {
    30
}

fn parse_type(s: &str) -> SearchType {
    use SearchType::*;
    match s {
        "内容" | "content" => Content,
        "游戏" | "game" => Game,
        "小程序" | "mini" => MiniProgram,
        "用户" | "user" => User,
        "话题" | "topic" => Topic,
        "商品" | "product" => Product,
        _ => Comprehensive,
    }
}

/// GET /api/search?q=...&search_type=综合
pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = api_search(
        &c,
        SearchReq {
            q: q.q,
            search_type: parse_type(&q.search_type),
            offset: q.offset,
            limit: q.limit,
            topic_id: q.topic_id,
        },
    )
    .await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct KeywordParams {
    pub keyword: String,
}

/// GET /api/search/topic?keyword=...
pub async fn topic(
    State(state): State<AppState>,
    Query(q): Query<KeywordParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = search_topic(&c, &q.keyword).await?;
    Ok(Json(v))
}

/// GET /api/search/community?keyword=...
pub async fn community(
    State(state): State<AppState>,
    Query(q): Query<KeywordParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = search_community(&c, &q.keyword).await?;
    Ok(Json(v))
}

/// GET /api/search/discovery — 热搜
pub async fn discovery(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = search_found(&c).await?;
    Ok(Json(v))
}
