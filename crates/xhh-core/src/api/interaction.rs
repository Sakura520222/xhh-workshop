//! 点赞 / 收藏 API
//!
//!
//! 帖子点赞为**显式操作**（award_type=1 点赞 / 0 取消），调用方需根据 `is_award_link` 判断状态。
//! 评论点赞、收藏为**切换式**（toggle），调用方无需判断状态。

use std::collections::BTreeMap;

use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

const PATH_LIKE_POST: &str = "/bbs/app/profile/award/link";
const PATH_LIKE_COMMENT: &str = "/bbs/app/comment/support";
const PATH_FAVOUR: &str = "/bbs/app/link/favour";
const PATH_FAV_FOLDERS: &str = "/bbs/app/profile/fav/folders";
const PATH_FAV_FOLDER_ADD: &str = "/bbs/app/profile/fav/folder/add";
const PATH_FAV_FOLDER_LINKS: &str = "/bbs/app/profile/fav/folder/v2/links";

/// 帖子点赞 / 取消点赞（显式操作，非切换）
///
/// - `award_type=1` 点赞，`award_type=0` 取消点赞
/// - 调用方需根据帖子详情的 `is_award_link` 字段判断当前状态后传入
pub async fn like_post(client: &XhhClient, link_id: &str, award_type: i64) -> Result<Value> {
    tracing::info!(link_id = %link_id, award_type = award_type, "帖子点赞");
    let mut body = BTreeMap::new();
    body.insert("link_id".into(), link_id.into());
    body.insert("award_type".into(), award_type.to_string());
    client.post(PATH_LIKE_POST, &body, 0).await
}

/// 评论点赞 / 取消点赞（toggle，support_type=2）
pub async fn toggle_like_comment(client: &XhhClient, comment_id: &str) -> Result<Value> {
    tracing::info!(comment_id = %comment_id, "评论点赞切换");
    let mut body = BTreeMap::new();
    body.insert("comment_id".into(), comment_id.into());
    body.insert("support_type".into(), "2".into());
    client.post(PATH_LIKE_COMMENT, &body, 0).await
}

/// 收藏 / 取消收藏（toggle）
///
/// `folder_id` 为 None 时使用默认收藏夹。
pub async fn toggle_favourite(
    client: &XhhClient,
    link_id: &str,
    folder_id: Option<&str>,
) -> Result<Value> {
    tracing::info!(link_id = %link_id, folder_id = ?folder_id, "收藏切换");
    let folder = folder_id.unwrap_or("");
    let mut body = BTreeMap::new();
    body.insert("link_id".into(), link_id.into());
    body.insert("fav_folder_id".into(), folder.into());
    client.post(PATH_FAVOUR, &body, 0).await
}

/// 收藏夹列表
pub async fn favourite_folders(client: &XhhClient) -> Result<Value> {
    tracing::debug!("获取收藏夹列表");
    client.get(PATH_FAV_FOLDERS, &[]).await
}

/// 创建收藏夹
pub async fn create_favourite_folder(client: &XhhClient, name: &str) -> Result<Value> {
    tracing::info!(name = %name, "创建收藏夹");
    let mut body = BTreeMap::new();
    body.insert("name".into(), name.into());
    client.post(PATH_FAV_FOLDER_ADD, &body, 0).await
}

/// 收藏夹内容列表（offset 分页）
///
/// `folder_id` 为 None 时返回全部收藏内容。
pub async fn favourite_folder_links(
    client: &XhhClient,
    folder_id: Option<&str>,
    offset: u32,
    limit: u32,
) -> Result<Value> {
    tracing::debug!(folder_id = ?folder_id, offset = offset, limit = limit, "获取收藏内容列表");
    let mut params: Vec<(&str, String)> = vec![
        ("enable_new_style_collect", "1".into()),
        ("dw", "604".into()),
        ("offset", offset.to_string()),
        ("limit", limit.to_string()),
        ("no_more", "false".into()),
    ];
    if let Some(fid) = folder_id {
        if !fid.is_empty() {
            params.push(("folder_id", fid.into()));
        }
    }
    let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    client.get(PATH_FAV_FOLDER_LINKS, &params_ref).await
}

#[cfg(test)]
mod tests {
    // 函数签名即测试，集成测试在 M1.10 之后通过 CLI 走通
}
