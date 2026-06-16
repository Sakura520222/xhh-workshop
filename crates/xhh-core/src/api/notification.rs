//! 消息/通知列表 API
//!

use serde::Serialize;
use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

const PATH_MESSAGES: &str = "/bbs/app/user/message";

/// 各类未读计数（来自消息列表响应的 `message_unread_num`）
#[derive(Debug, Clone, Default, Serialize)]
pub struct UnreadCount {
    pub comment: u32,
    pub award: u32,
}

impl UnreadCount {
    /// 合计未读数
    pub fn total(&self) -> u32 {
        self.comment.saturating_add(self.award)
    }
}

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

/// 轻量获取未读计数：仅拉取 limit=1 的消息列表，解析 `message_unread_num`。
pub async fn unread_count(client: &XhhClient) -> Result<UnreadCount> {
    let v = list_messages(client, 0, 0, 1).await?;
    Ok(parse_unread_count(&v))
}

/// 从消息列表响应中提取未读计数
pub fn parse_unread_count(v: &Value) -> UnreadCount {
    let num = v
        .get("result")
        .and_then(|r| r.get("message_unread_num"))
        .unwrap_or(&Value::Null);
    UnreadCount {
        comment: num
            .get("comment")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or(0),
       award: num
           .get("award")
           .and_then(Value::as_u64)
           .map(|n| n as u32)
           .unwrap_or(0),
   }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_unread_count_normal() {
        let v = json!({ "result": { "message_unread_num": { "award": 2, "comment": 3 } } });
        let c = parse_unread_count(&v);
        assert_eq!(c.comment, 3);
        assert_eq!(c.award, 2);
        assert_eq!(c.total(), 5);
    }

    #[test]
    fn parse_unread_count_missing_fields() {
        let v = json!({ "result": { "message_unread_num": {} } });
        let c = parse_unread_count(&v);
        assert_eq!(c.total(), 0);
    }

    #[test]
    fn parse_unread_count_missing_block() {
        let v = json!({ "status": "ok" });
        let c = parse_unread_count(&v);
        assert_eq!(c.total(), 0);
    }
}
