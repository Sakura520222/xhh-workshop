//! 消息/通知列表 API
//!

use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

const PATH_MESSAGES: &str = "/bbs/app/user/message";

/// 消息列表请求
pub async fn list_messages(
    client: &XhhClient,
    list_type: u32,
    offset: u32,
    limit: u32,
) -> Result<Value> {
    tracing::debug!(
        list_type = list_type,
        offset = offset,
        limit = limit,
        "获取消息列表"
    );
    client
        .get(
            PATH_MESSAGES,
            &[
                ("list_type", &list_type.to_string()),
                ("offset", &offset.to_string()),
                ("limit", &limit.to_string()),
                ("no_more", "false"),
            ],
        )
        .await
}

/// 全部消息（list_type=0）
pub async fn list_all_messages(client: &XhhClient, offset: u32, limit: u32) -> Result<Value> {
    list_messages(client, 0, offset, limit).await
}
