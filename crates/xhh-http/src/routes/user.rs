//! 用户主页 / 关注 / 粉丝

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use xhh_core::api::user::{
    follow_user, follower_list, following_list, unfollow_user, user_profile,
};

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ProfileParams {
    pub userid: Option<String>,
}

/// GET /api/user/profile?userid=...
pub async fn profile(
    State(state): State<AppState>,
    Query(q): Query<ProfileParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = user_profile(&c, q.userid.as_deref()).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct FollowListParams {
    pub userid: String,
}

/// GET /api/user/following?userid=...
pub async fn following(
    State(state): State<AppState>,
    Query(q): Query<FollowListParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = following_list(&c, &q.userid).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct FollowerParams {
    pub userid: String,
    #[serde(default = "default_offset")]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
fn default_offset() -> u32 {
    0
}
fn default_limit() -> u32 {
    100
}

/// GET /api/user/follower?userid=...
pub async fn follower(
    State(state): State<AppState>,
    Query(q): Query<FollowerParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = follower_list(&c, &q.userid, q.offset, q.limit).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct UseridReq {
    pub userid: String,
}

/// POST /api/user/follow
pub async fn follow(
    State(state): State<AppState>,
    Json(req): Json<UseridReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = follow_user(&c, &req.userid).await?;
    Ok(Json(v))
}

/// POST /api/user/unfollow
pub async fn unfollow(
    State(state): State<AppState>,
    Json(req): Json<UseridReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = unfollow_user(&c, &req.userid).await?;
    Ok(Json(v))
}
