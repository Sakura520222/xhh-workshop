//! 评论相关

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

#[allow(deprecated)]
use xhh_core::api::comment::{
    create_comment, delete_comment, list_comments, list_sub_comments, CreateCommentReq,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub link_id: String,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
fn default_page() -> u32 {
    1
}
fn default_limit() -> u32 {
    20
}

/// GET /api/comment/list?link_id=...&page=1&limit=20
#[allow(deprecated)]
pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = list_comments(&c, &q.link_id, q.page, q.limit).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct SubParams {
    pub root_comment_id: String,
    pub lastval: Option<String>,
}

/// GET /api/comment/sub?root_comment_id=...&lastval=...
pub async fn sub_list(
    State(state): State<AppState>,
    Query(q): Query<SubParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = list_sub_comments(&c, &q.root_comment_id, q.lastval.as_deref()).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct CreateReq {
    pub link_id: String,
    pub text: String,
    #[serde(default = "default_neg_one")]
    pub reply_id: String,
    #[serde(default = "default_neg_one")]
    pub root_id: String,
    #[serde(default)]
    pub is_cy: Option<String>,
    #[serde(default)]
    pub imgs: Vec<String>,
}
fn default_neg_one() -> String {
    "-1".to_string()
}

/// POST /api/comment/create
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateReq>,
) -> ApiResult<Json<Value>> {
    if req.link_id.is_empty() || req.text.is_empty() {
        return Err(ApiError::BadRequest("link_id 与 text 均需非空".into()));
    }
    let c = state.require_client().await?;
    let cr = CreateCommentReq {
        link_id: req.link_id,
        text: req.text,
        reply_id: req.reply_id,
        root_id: req.root_id,
        is_cy: req.is_cy.unwrap_or_else(|| "0".into()),
        imgs: req.imgs,
    };
    let v = create_comment(&c, cr).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct DeleteReq {
    pub comment_id: String,
}

/// POST /api/comment/delete
pub async fn delete(
    State(state): State<AppState>,
    Json(req): Json<DeleteReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = delete_comment(&c, &req.comment_id).await?;
    Ok(Json(v))
}
