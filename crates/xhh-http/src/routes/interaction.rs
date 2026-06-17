//! 点赞 / 收藏

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use xhh_core::api::interaction::{
    create_favourite_folder, favourite, favourite_folders, like_post as api_like_post,
    toggle_like_comment, unfavourite,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LinkIdReq {
    pub link_id: String,
    /// 帖子点赞：1=点赞, 0=取消，默认 1
    pub award_type: Option<i64>,
}

/// POST /api/like/post
pub async fn like_post(
    State(state): State<AppState>,
    Json(req): Json<LinkIdReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = api_like_post(&c, &req.link_id, req.award_type.unwrap_or(1)).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct CommentIdReq {
    pub comment_id: String,
}

/// POST /api/like/comment
pub async fn like_comment(
    State(state): State<AppState>,
    Json(req): Json<CommentIdReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = toggle_like_comment(&c, &req.comment_id).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct FavourReq {
    pub link_id: String,
    pub folder_id: Option<String>,
    /// 2=收藏（默认）, 1=取消
    pub favour_type: Option<i64>,
}

/// POST /api/favour/toggle
pub async fn favour(
    State(state): State<AppState>,
    Json(req): Json<FavourReq>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = if req.favour_type.unwrap_or(1) == 2 {
        unfavourite(&c, &req.link_id, req.folder_id.as_deref()).await?
    } else {
        favourite(&c, &req.link_id, req.folder_id.as_deref()).await?
    };
    Ok(Json(v))
}

/// GET /api/favour/folders
pub async fn folders(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = favourite_folders(&c).await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderReq {
    pub name: String,
}

/// POST /api/favour/folder
pub async fn create_folder(
    State(state): State<AppState>,
    Json(req): Json<CreateFolderReq>,
) -> ApiResult<Json<Value>> {
    if req.name.is_empty() {
        return Err(ApiError::BadRequest("name 必填".into()));
    }
    let c = state.require_client().await?;
    let v = create_favourite_folder(&c, &req.name).await?;
    Ok(Json(v))
}
