//! 帖子列表 / 帖子详情 / 用户动态
//!

use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

/// 帖子列表请求参数
#[derive(Debug, Clone, Default)]
pub struct FeedsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub tag_id: Option<u32>,
    /// 按社区 topic_id 过滤（传给 API 的 topic_id 参数）
    pub topic_id: Option<u32>,
    /// 按收藏夹过滤（传给 API 的 fav_folder_id 参数）
    pub fav_folder_id: Option<String>,
}

/// 获取帖子列表
///
/// 对应 `GET /bbs/app/feeds`，注意必须带 `pull=1`，否则返回「非法请求」。
pub async fn feeds(client: &XhhClient, q: FeedsQuery) -> Result<Value> {
    tracing::debug!(page = ?q.page, limit = ?q.limit, tag_id = ?q.tag_id, topic_id = ?q.topic_id, "获取帖子列表");
    let mut extras: Vec<(&str, String)> = vec![("pull", "1".to_string())];
    if let Some(p) = q.page {
        extras.push(("page", p.to_string()));
    }
    if let Some(l) = q.limit {
        extras.push(("limit", l.to_string()));
    }
    if let Some(t) = q.tag_id {
        if t > 0 {
            extras.push(("tag_id", t.to_string()));
        }
    }
    if let Some(tid) = q.topic_id {
        if tid > 0 {
            extras.push(("topic_id", tid.to_string()));
        }
    }
    if let Some(ref fid) = q.fav_folder_id {
        if !fid.is_empty() {
            extras.push(("fav_folder_id", fid.clone()));
        }
    }
    let extras_ref: Vec<(&str, &str)> = extras.iter().map(|(k, v)| (*k, v.as_str())).collect();
    client.get("/bbs/app/feeds", &extras_ref).await
}

/// 帖子详情请求参数
///
/// 对应 `GET /bbs/app/link/tree`（§9）。
/// 所有分页参数可选，不传时等同首次加载。
#[derive(Debug, Clone, Default)]
pub struct PostDetailQuery {
    pub page: Option<u32>,
    /// 排序索引：1=热门，2=正序，3=倒序
    pub index: Option<u32>,
    pub limit: Option<u32>,
    /// 首次加载传 Some(1)，翻页传 Some(0)
    pub is_first: Option<u32>,
    /// 0=全部楼层，1=仅看楼主
    pub owner_only: Option<u32>,
}

/// 帖子详情
///
/// 对应 `GET /bbs/app/link/tree`（§9），返回值 `result.link` 为帖子详情，
/// `result.comments` 为楼层列表。
pub async fn post_detail(client: &XhhClient, link_id: &str, q: PostDetailQuery) -> Result<Value> {
    tracing::debug!(link_id = %link_id, page = ?q.page, index = ?q.index, "获取帖子详情");
    let mut extras: Vec<(&str, String)> = vec![("link_id", link_id.to_string())];
    if let Some(p) = q.page {
        extras.push(("page", p.to_string()));
    }
    if let Some(i) = q.index {
        extras.push(("index", i.to_string()));
    }
    if let Some(l) = q.limit {
        extras.push(("limit", l.to_string()));
    }
    if let Some(f) = q.is_first {
        extras.push(("is_first", f.to_string()));
    }
    if let Some(o) = q.owner_only {
        extras.push(("owner_only", o.to_string()));
    }
    let extras_ref: Vec<(&str, &str)> = extras.iter().map(|(k, v)| (*k, v.as_str())).collect();
    client.get("/bbs/app/link/tree", &extras_ref).await
}

/// 用户动态（用户帖子列表）
///
/// 对应 `GET /bbs/app/profile/events`。
/// - `userid` 为 None 时默认当前用户
/// - `lastval` 为 None 或空字符串时获取第一页
pub async fn user_events(
    client: &XhhClient,
    userid: Option<&str>,
    lastval: Option<&str>,
) -> Result<Value> {
    tracing::debug!(
        userid = userid.unwrap_or("self"),
        lastval = lastval.unwrap_or(""),
        "获取用户动态"
    );
    let userid = userid.unwrap_or(&client.heybox_id);
    let lastval = lastval.unwrap_or("");
    let list_type = "moment";
    let dw = "628";
    client
        .get(
            "/bbs/app/profile/events",
            &[
                ("list_type", list_type),
                ("userid", userid),
                ("dw", dw),
                ("lastval", lastval),
            ],
        )
        .await
}

/// 社区帖子列表请求参数
///
/// 对应 `GET /bbs/app/topic/feeds`（§37）
#[derive(Debug, Clone, Default)]
pub struct CommunityFeedsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub lastval: Option<String>,
    /// 排序方式（如 `hot-rank`），默认综合
    pub sort_filter: Option<String>,
    pub dw: Option<u32>,
}

/// 获取社区帖子列表（§37）
pub async fn community_feeds(
    client: &XhhClient,
    topic_id: u32,
    q: CommunityFeedsQuery,
) -> Result<Value> {
    tracing::debug!(topic_id = topic_id, limit = ?q.limit, "获取社区帖子列表");
    let mut extras: Vec<(String, String)> = vec![("topic_id".into(), topic_id.to_string())];
    if let Some(l) = q.limit {
        extras.push(("limit".into(), l.to_string()));
    }
    if let Some(o) = q.offset {
        extras.push(("offset".into(), o.to_string()));
    }
    if let Some(ref lv) = q.lastval {
        if !lv.is_empty() {
            extras.push(("lastval".into(), lv.clone()));
        }
    }
    if let Some(ref sf) = q.sort_filter {
        if !sf.is_empty() {
            extras.push(("sort_filter".into(), sf.clone()));
        }
    }
    if let Some(d) = q.dw {
        extras.push(("dw".into(), d.to_string()));
    }
    let extras_ref: Vec<(&str, &str)> = extras
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    client.get("/bbs/app/topic/feeds", &extras_ref).await
}

/// 社区菜单（Tab 结构 + 排序筛选器）
///
/// 对应 `GET /bbs/app/topic/menu`（§38）。返回 `menu[].type` 为 `"link"`/`"news"`，
/// `menu[].params.cate_id` 需合并到社区头条请求。
pub async fn topic_menu(client: &XhhClient, topic_id: u32) -> Result<Value> {
    tracing::debug!(topic_id = topic_id, "获取社区菜单");
    client
        .get("/bbs/app/topic/menu", &[("topic_id", &topic_id.to_string())])
        .await
}

/// 社区头条列表请求参数
///
/// 对应 `GET /bbs/app/topic/feeds/news`（§40），响应结构与 [`community_feeds`] 相同。
#[derive(Debug, Clone, Default)]
pub struct CommunityNewsQuery {
    /// 分类 ID（来自 `topic/menu` 的 `params.cate_id`）
    pub cate_id: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub lastval: Option<String>,
    pub sort_filter: Option<String>,
    pub dw: Option<u32>,
}

/// 获取社区头条列表（§40）
pub async fn community_feeds_news(
    client: &XhhClient,
    topic_id: u32,
    q: CommunityNewsQuery,
) -> Result<Value> {
    tracing::debug!(topic_id = topic_id, cate_id = ?q.cate_id, "获取社区头条列表");
    let mut extras: Vec<(String, String)> = vec![("topic_id".into(), topic_id.to_string())];
    if let Some(c) = q.cate_id {
        extras.push(("cate_id".into(), c.to_string()));
    }
    if let Some(l) = q.limit {
        extras.push(("limit".into(), l.to_string()));
    }
    if let Some(o) = q.offset {
        extras.push(("offset".into(), o.to_string()));
    }
    if let Some(ref lv) = q.lastval {
        if !lv.is_empty() {
            extras.push(("lastval".into(), lv.clone()));
        }
    }
    if let Some(ref sf) = q.sort_filter {
        if !sf.is_empty() {
            extras.push(("sort_filter".into(), sf.clone()));
        }
    }
    if let Some(d) = q.dw {
        extras.push(("dw".into(), d.to_string()));
    }
    let extras_ref: Vec<(&str, &str)> = extras
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    client.get("/bbs/app/topic/feeds/news", &extras_ref).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeds_query_default() {
        let q = FeedsQuery::default();
        assert!(q.page.is_none());
        assert!(q.limit.is_none());
        assert!(q.tag_id.is_none());
    }
}
