//! 帖子列表 / 详情 / 用户动态

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use xhh_core::api::feed::{
    feeds as api_feeds, post_detail as api_post_detail, user_events as api_user_events, FeedsQuery,
    PostDetailQuery,
};

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct FeedsParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub tag_id: Option<u32>,
    pub topic_id: Option<u32>,
}

/// GET /api/feeds
pub async fn list_feeds(
    State(state): State<AppState>,
    Query(q): Query<FeedsParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = api_feeds(
        &c,
        FeedsQuery {
            page: q.page,
            limit: q.limit,
            tag_id: q.tag_id,
            topic_id: q.topic_id,
            fav_folder_id: None,
        },
    )
    .await?;
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
pub struct PostDetailParams {
    pub page: Option<u32>,
    pub index: Option<u32>,
    pub limit: Option<u32>,
    pub is_first: Option<u32>,
    pub owner_only: Option<u32>,
}

/// GET /api/post/detail/:link_id?page=1&index=1&limit=20&is_first=1
///
/// 仅首屏请求（`is_first=1`）走本地缓存；其余透传 API。
pub async fn post_detail(
    State(state): State<AppState>,
    axum::extract::Path(link_id): axum::extract::Path<String>,
    Query(q): Query<PostDetailParams>,
) -> ApiResult<Json<Value>> {
    let query = PostDetailQuery {
        page: q.page,
        index: q.index,
        limit: q.limit,
        is_first: q.is_first,
        owner_only: q.owner_only,
    };
    if q.is_first == Some(1) {
        // 先查缓存：命中则无需登录 / 联网，支持离线查看
        let cm = xhh_core::cache::CacheManager::from_default_config();
        if let Some(v) = cm.get_post(&link_id).ok().flatten() {
            return Ok(Json(v));
        }
        let c = state.require_client().await?;
        let v = api_post_detail(&c, &link_id, query).await?;
        let _ = cm.put_post(&link_id, v.clone());
        Ok(Json(v))
    } else {
        let c = state.require_client().await?;
        let v = api_post_detail(&c, &link_id, query).await?;
        Ok(Json(v))
    }
}

#[derive(Debug, Deserialize)]
pub struct UserEventsParams {
    pub userid: Option<String>,
    pub lastval: Option<String>,
}

/// GET /api/user/events
pub async fn user_events(
    State(state): State<AppState>,
    Query(q): Query<UserEventsParams>,
) -> ApiResult<Json<Value>> {
    let c = state.require_client().await?;
    let v = api_user_events(&c, q.userid.as_deref(), q.lastval.as_deref()).await?;
    Ok(Json(v))
}
