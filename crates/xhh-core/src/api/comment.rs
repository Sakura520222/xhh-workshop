//! 评论相关 API
//!
//!
//! - 发评论/回复（`/bbs/app/comment/create`）
//! - 删除评论（`/bbs/app/comment/delete`）
//! - 评论列表（`/bbs/app/link/game/comments`）
//! - 子评论列表（`/bbs/app/comment/sub/comments`）

use std::collections::BTreeMap;

use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

/// 发评论 / 回复评论请求参数
#[derive(Debug, Clone)]
pub struct CreateCommentReq {
    pub link_id: String,
    pub text: String,
    /// 回复目标评论 ID。**顶级评论传 `-1`。**
    pub reply_id: String,
    /// 根评论 ID。**顶级评论传 `-1`，回复子评论时传该楼层根评论 ID。**
    pub root_id: String,
    /// `"0"` 普通评论，`"1"` 社区评论
    pub is_cy: String,
    /// 带图评论的图片 URL 列表（可选）
    pub imgs: Vec<String>,
}

impl CreateCommentReq {
    /// 构造一个顶级评论请求
    pub fn top_level(link_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            link_id: link_id.into(),
            text: text.into(),
            reply_id: "-1".into(),
            root_id: "-1".into(),
            is_cy: "0".into(),
            imgs: Vec::new(),
        }
    }

    /// 构造一个回复子评论请求
    pub fn reply(
        link_id: impl Into<String>,
        text: impl Into<String>,
        reply_id: impl Into<String>,
        root_id: impl Into<String>,
    ) -> Self {
        Self {
            link_id: link_id.into(),
            text: text.into(),
            reply_id: reply_id.into(),
            root_id: root_id.into(),
            is_cy: "0".into(),
            imgs: Vec::new(),
        }
    }
}

const PATH_CREATE: &str = "/bbs/app/comment/create";
const PATH_DELETE: &str = "/bbs/app/comment/delete";
const PATH_LIST: &str = "/bbs/app/link/game/comments";
const PATH_SUB: &str = "/bbs/app/comment/sub/comments";

/// 发评论 / 回复
///
/// hkey 偏移 +1（§17）。
pub async fn create_comment(client: &XhhClient, req: CreateCommentReq) -> Result<Value> {
    tracing::info!(link_id = %req.link_id, reply_id = %req.reply_id, root_id = %req.root_id, imgs_len = req.imgs.len(), "发评论");
    let imgs_json = if req.imgs.is_empty() {
        "".into()
    } else {
        serde_json::to_string(&req.imgs).unwrap_or_default()
    };
    let mut body = BTreeMap::new();
    body.insert("link_id".into(), req.link_id);
    body.insert("text".into(), req.text);
    body.insert("reply_id".into(), req.reply_id);
    body.insert("root_id".into(), req.root_id);
    body.insert("is_cy".into(), req.is_cy);
    body.insert("imgs".into(), imgs_json);
    client.post(PATH_CREATE, &body, 1).await
}

/// 删除评论
pub async fn delete_comment(client: &XhhClient, comment_id: &str) -> Result<Value> {
    tracing::info!(comment_id = %comment_id, "删除评论");
    let mut body = BTreeMap::new();
    body.insert("comment_id".into(), comment_id.into());
    client.post(PATH_DELETE, &body, 0).await
}

/// 评论列表（按帖子维度）
///
/// **已废弃**（2026-06-11 抓包确认）：Web 端不再调用此接口，
/// 实测返回 `{"msg":"请求失败了","status":"failed"}`。
/// 获取评论请使用 `feed::post_detail`（§9 `/bbs/app/link/tree`），该接口
/// 已返回完整楼层列表并支持分页。
#[deprecated(note = "Web 端已废弃，请使用 feed::post_detail 获取评论")]
pub async fn list_comments(
    client: &XhhClient,
    link_id: &str,
    page: u32,
    limit: u32,
) -> Result<Value> {
    client
        .get(
            PATH_LIST,
            &[
                ("link_id", link_id),
                ("page", &page.to_string()),
                ("limit", &limit.to_string()),
            ],
        )
        .await
}

/// 子评论列表（§20）
///
/// `lastval` 应传 `/bbs/app/link/tree` 返回的该楼层最后一条子评论的 `commentid`，
/// 后续翻页传上次返回的 `lastval`。
pub async fn list_sub_comments(
    client: &XhhClient,
    root_comment_id: &str,
    lastval: Option<&str>,
) -> Result<Value> {
    tracing::debug!(root_comment_id = %root_comment_id, "获取子评论");
    let lastval = lastval.unwrap_or("");
    client
        .get(
            PATH_SUB,
            &[("root_comment_id", root_comment_id), ("lastval", lastval)],
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_request_default_ids() {
        let r = CreateCommentReq::top_level("123", "hi");
        assert_eq!(r.reply_id, "-1");
        assert_eq!(r.root_id, "-1");
        assert_eq!(r.is_cy, "0");
    }

    #[test]
    fn reply_request_keeps_root() {
        let r = CreateCommentReq::reply("L", "hello", "888", "999");
        assert_eq!(r.reply_id, "888");
        assert_eq!(r.root_id, "999");
    }
}
