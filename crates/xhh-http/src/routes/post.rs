//! 发帖 / 编辑 / 删帖

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use xhh_core::api::post::{
    create_post, delete_post, edit_post, ContentInput, CreatePostReq, EditPostReq,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateReq {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub hashtags: Vec<String>,
    /// 社区 topic_id（启用社区模式）
    #[serde(default)]
    pub community_topic_id: Option<String>,
}

/// POST /api/post/create
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateReq>,
) -> ApiResult<Json<Value>> {
    if req.title.is_empty() || req.content.is_empty() {
        return Err(ApiError::BadRequest("title 和 content 均需非空".into()));
    }
    let c = state.require_client().await?;
    let (topic_ids, link_tag) = match req.community_topic_id.as_deref() {
        Some(t) if !t.is_empty() => (vec![t.to_string()], 27i64),
        _ => (vec!["58144".into()], 28i64),
    };
    let post_req = CreatePostReq {
        title: req.title,
        content: ContentInput::Plain(req.content),
        topic_ids,
        hashtags: req.hashtags,
        link_tag,
        ..Default::default()
    };
    let v = create_post(&c, post_req).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct EditReq {
    pub link_id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub hashtags: Vec<String>,
    #[serde(default)]
    pub topic_ids: Vec<String>,
    #[serde(default = "default_link_tag")]
    pub link_tag: i64,
}

fn default_link_tag() -> i64 {
    28
}

/// POST /api/post/edit
pub async fn edit(
    State(state): State<AppState>,
    Json(req): Json<EditReq>,
) -> ApiResult<Json<Value>> {
    if req.link_id.is_empty() {
        return Err(ApiError::BadRequest("link_id 必填".into()));
    }
    let topic_ids = if req.topic_ids.is_empty() {
        vec!["58144".to_string()]
    } else {
        req.topic_ids
    };
    let c = state.require_client().await?;
    let v = edit_post(
        &c,
        EditPostReq {
            link_id: req.link_id,
            title: req.title,
            content: ContentInput::Plain(req.content),
            topic_ids,
            hashtags: req.hashtags,
            link_tag: req.link_tag,
        },
    )
    .await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct DeleteReq {
    pub link_id: String,
}

/// POST /api/post/delete
pub async fn delete(
    State(state): State<AppState>,
    Json(req): Json<DeleteReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = delete_post(&c, &req.link_id).await?;
    Ok(Json(v))
}
