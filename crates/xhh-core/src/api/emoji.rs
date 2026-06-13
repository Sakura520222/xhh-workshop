//! Emoji 列表 API
//!

use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

const PATH_EMOJI: &str = "/bbs/app/api/emojis/list";

/// 获取 Emoji 列表（按 group_code 分组）
///
/// 前端按 `emojis[].type === 1 || type === 3` 过滤显示。
pub async fn list_emojis(client: &XhhClient) -> Result<Value> {
    tracing::debug!("获取 Emoji 列表");
    client.get(PATH_EMOJI, &[]).await
}
