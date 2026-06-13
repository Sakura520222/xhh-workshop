//! Agent 工具集：把 xhh-core 的 API 包装为 function-calling 工具
//!
//! 设计：
//! - [`Tool`] trait — 统一接口
//! - [`ToolRegistry`] — 集中注册 + 按 name 查找
//! - 17 个内置工具，覆盖帖子/评论/点赞/收藏/搜索/社区/用户/上传全部功能
//!
//! 每个 Tool 接收 JSON 字符串参数，返回 JSON 字符串结果。

use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use xhh_core::api::{
    comment as api_comment, feed as api_feed, interaction as api_inter, post as api_post,
    search as api_search, upload as api_upload, user as api_user,
};
use xhh_core::client::XhhClient;

use crate::error::{Error, Result};
use crate::provider::ToolSpec;
use crate::text::truncate_chars;

/// 危险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// 工具确认信息
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct ToolConfirmation {
    pub tool_name: &'static str,
    pub risk_level: RiskLevel,
    pub summary: String,
    pub arguments_json: String,
}

/// 工具执行接口
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名（与 ToolSpec.name 一致）
    fn name(&self) -> &'static str;

    /// 工具规格（用于注册到 LLM）
    fn spec(&self) -> ToolSpec;

    /// 是否需要执行前确认
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// 构建确认信息
    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::Medium,
            summary: format!("执行工具 {}", self.name()),
            arguments_json: arguments_json.to_string(),
        }
    }

    /// 执行（arguments_json 是 LLM 给的 JSON 字符串）
    async fn execute(&self, client: &XhhClient, arguments_json: &str) -> Result<String>;
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

fn parsed_args(arguments_json: &str) -> Value {
    if arguments_json.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(arguments_json).unwrap_or_else(|_| json!({}))
    }
}

fn arg_str<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(|s| s.as_str()).unwrap_or("")
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册全部内置工具（17 个）
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        // 查询类
        reg.register(Box::new(SearchCommunityTool));
        reg.register(Box::new(SearchTopicTool));
        reg.register(Box::new(SearchFeedsTool));
        reg.register(Box::new(SearchPostsTool));
        reg.register(Box::new(CommunityFeedsTool));
        reg.register(Box::new(MyPostsTool));
        reg.register(Box::new(UserProfileTool));
        reg.register(Box::new(PostDetailTool));
        // 帖子操作
        reg.register(Box::new(CreatePostTool));
        reg.register(Box::new(EditPostTool));
        reg.register(Box::new(DeletePostTool));
        // 评论操作
        reg.register(Box::new(ReplyCommentTool));
        reg.register(Box::new(DeleteCommentTool));
        // 互动
        reg.register(Box::new(LikePostTool));
        reg.register(Box::new(LikeCommentTool));
        reg.register(Box::new(FavouriteTool));
        // 上传
        reg.register(Box::new(UploadImageTool));
        reg
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    /// 收集所有 ToolSpec（喂给 LLM tools 参数）
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.spec()).collect()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 内置工具 ──────────────────────────────────────────────

/// 发帖工具
pub struct CreatePostTool;

#[async_trait]
impl Tool for CreatePostTool {
    fn name(&self) -> &'static str {
        "create_post"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "create_post",
            "在小黑盒发布一篇图文帖子。仅当用户明确要求发帖时使用。",
            json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "帖子标题，最多 30 字",
                        "maxLength": 30
                    },
                    "content": {
                        "type": "string",
                        "description": "帖子正文，300-800 字，自然中文，避免机器人腔调"
                    },
                    "hashtags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "话题标签名列表，如 [\"原神\"]，无则为空数组"
                    },
                    "community_topic_id": {
                        "type": "string",
                        "description": "（可选）发到指定社区的 topic_id。留空则发到默认的盒友杂谈"
                    }
                },
                "required": ["title", "content"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let title = arg_str(&v, "title");
        let content = arg_str(&v, "content");
        let topic = arg_str(&v, "community_topic_id");
        let target = if topic.is_empty() {
            "默认板块".to_string()
        } else {
            format!("社区 topic_id={}", topic)
        };
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::High,
            summary: format!(
                "发布帖子《{}》到{}，正文预览：{}",
                if title.is_empty() {
                    "未提供标题"
                } else {
                    title
                },
                target,
                truncate_chars(content, 80)
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = if args_json.is_empty() {
            json!({})
        } else {
            serde_json::from_str(args_json).map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: format!("参数解析失败: {}", e),
            })?
        };

        let title = v.get("title").and_then(|s| s.as_str()).unwrap_or("");
        if title.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "title 不能为空".into(),
            });
        }
        let content = v.get("content").and_then(|s| s.as_str()).unwrap_or("");
        if content.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "content 不能为空".into(),
            });
        }
        let hashtags: Vec<String> = v
            .get("hashtags")
            .and_then(|h| h.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let (topic_ids, link_tag) =
            if let Some(tid) = v.get("community_topic_id").and_then(|s| s.as_str()) {
                if !tid.is_empty() {
                    (vec![tid.to_string()], 27i64)
                } else {
                    (vec!["58144".into()], 28i64)
                }
            } else {
                (vec!["58144".into()], 28i64)
            };

        let req = api_post::CreatePostReq {
            title: title.into(),
            content: content.into(),
            topic_ids,
            hashtags,
            link_tag,
            ..Default::default()
        };
        let resp = api_post::create_post(client, req)
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;

        // 多路径解析 link_id（服务端可能放在 result.link_id / link_id / result.linkid / linkid）
        let link_id = resp
            .pointer("/result/link_id")
            .or_else(|| resp.get("link_id"))
            .or_else(|| resp.pointer("/result/linkid"))
            .or_else(|| resp.get("linkid"));
        let status_ok = resp.get("status").and_then(|s| s.as_str()) == Some("ok");

        if !status_ok {
            return Ok(json!({
                "ok": false,
                "message": format!("发帖失败，服务端响应: {}", resp),
                "raw_response": resp,
            })
            .to_string());
        }

        Ok(json!({
            "ok": true,
            "link_id": link_id,
            "message": if link_id.is_some() {
                format!("发帖成功，link_id={}", link_id.unwrap())
            } else {
                "发帖成功（服务端未返回 link_id，请通过 search_feeds 或我的帖子列表确认）".to_string()
            },
            "raw_response": resp,
        })
        .to_string())
    }
}

/// 搜索社区（拿到 topic_id 后用于 create_post.community_topic_id）
pub struct SearchCommunityTool;

#[async_trait]
impl Tool for SearchCommunityTool {
    fn name(&self) -> &'static str {
        "search_community"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "search_community",
            "按关键词搜索小黑盒社区/板块。返回社区列表，包含 topic_id 与热度。发帖到指定板块前，先用本工具拿到目标 topic_id。",
            json!({
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "社区/游戏名称关键词，如 \"原神\"、\"黑神话悟空\"、\"无畏契约\""
                    }
                },
                "required": ["keyword"]
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args_json).map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("参数解析失败: {}", e),
        })?;
        let keyword = v.get("keyword").and_then(|s| s.as_str()).unwrap_or("");
        if keyword.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "keyword 不能为空".into(),
            });
        }
        let resp = api_search::search_community(client, keyword)
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;
        let arr = resp
            .pointer("/result/search_result")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let communities: Vec<Value> = arr
            .iter()
            .filter(|c| c.get("search_type").and_then(|s| s.as_str()) == Some("topic"))
            .take(5)
            .map(|c| {
                json!({
                    "topic_id": c.get("topic_id"),
                    "name": c.get("name"),
                    "hot_desc": c.pointer("/hot/desc"),
                    "game_app_id": c.pointer("/game/app_id"),
                })
            })
            .collect();
        Ok(json!({
            "keyword": keyword,
            "count": communities.len(),
            "communities": communities,
            "hint": "如要发帖到这些社区之一，把 topic_id 传给 create_post.community_topic_id"
        })
        .to_string())
    }
}

/// 搜索话题标签（拿到 name 后用于 create_post.hashtags）
pub struct SearchTopicTool;

#[async_trait]
impl Tool for SearchTopicTool {
    fn name(&self) -> &'static str {
        "search_topic"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "search_topic",
            "搜索话题标签（hashtag）。发帖时想带上某个话题标签，先用本工具拿到标准名再传给 create_post.hashtags。",
            json!({
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "话题关键词，如 \"原神\"、\"黑神话悟空\""
                    }
                },
                "required": ["keyword"]
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args_json).map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("参数解析失败: {}", e),
        })?;
        let keyword = v.get("keyword").and_then(|s| s.as_str()).unwrap_or("");
        if keyword.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "keyword 不能为空".into(),
            });
        }
        let resp = api_search::search_topic(client, keyword)
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;
        let arr = resp
            .pointer("/result/search_result")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let topics: Vec<Value> = arr
            .iter()
            .take(5)
            .map(|c| {
                json!({
                    "name": c.get("name"),
                    "id": c.get("id"),
                    "content_num": c.pointer("/num/content_num"),
                })
            })
            .collect();
        Ok(json!({
            "keyword": keyword,
            "count": topics.len(),
            "topics": topics,
            "hint": "把话题的 name 字段传给 create_post.hashtags 数组"
        })
        .to_string())
    }
}

/// 回复评论工具
pub struct ReplyCommentTool;

#[async_trait]
impl Tool for ReplyCommentTool {
    fn name(&self) -> &'static str {
        "reply_comment"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "reply_comment",
            "在某条帖子下发表评论或回复某条评论。请用自然的中文。",
            json!({
                "type": "object",
                "properties": {
                    "link_id": {"type": "string", "description": "目标帖子 ID"},
                    "text": {"type": "string", "description": "评论文本，建议 5-80 字"},
                    "reply_id": {
                        "type": "string",
                        "description": "（可选）回复的目标评论 ID；不填则发顶级评论"
                    },
                    "root_id": {
                        "type": "string",
                        "description": "（可选）根评论 ID；reply_id 是子评论时填该楼层的根评论 ID"
                    }
                },
                "required": ["link_id", "text"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let link_id = arg_str(&v, "link_id");
        let text = arg_str(&v, "text");
        let reply_id = arg_str(&v, "reply_id");
        let target = if reply_id.is_empty() {
            format!("帖子 {}", link_id)
        } else {
            format!("帖子 {} 下的评论 {}", link_id, reply_id)
        };
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::Medium,
            summary: format!("在{}发送评论：{}", target, truncate_chars(text, 80)),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args_json).map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("参数解析失败: {}", e),
        })?;
        let link_id = v.get("link_id").and_then(|s| s.as_str()).unwrap_or("");
        let text = v.get("text").and_then(|s| s.as_str()).unwrap_or("");
        if link_id.is_empty() || text.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "link_id 与 text 均需非空".into(),
            });
        }
        let req = match v.get("reply_id").and_then(|s| s.as_str()) {
            Some(rid) if !rid.is_empty() => {
                let root = v
                    .get("root_id")
                    .and_then(|s| s.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .unwrap_or_else(|| rid.to_string());
                api_comment::CreateCommentReq::reply(link_id, text, rid, &root)
            }
            _ => api_comment::CreateCommentReq::top_level(link_id, text),
        };
        let resp = api_comment::create_comment(client, req)
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;
        let cid = resp.pointer("/result/comment_id");
        Ok(json!({
            "ok": true,
            "comment_id": cid,
            "message": "评论发送成功"
        })
        .to_string())
    }
}

/// 帖子点赞工具
pub struct LikePostTool;

#[async_trait]
impl Tool for LikePostTool {
    fn name(&self) -> &'static str {
        "like_post"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "like_post",
            "点赞某个帖子（toggle 切换式：已点赞则取消，未点赞则点赞）。",
            json!({
                "type": "object",
                "properties": {
                    "link_id": {"type": "string", "description": "目标帖子 ID"}
                },
                "required": ["link_id"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let link_id = arg_str(&v, "link_id");
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::Medium,
            summary: format!(
                "切换帖子 {} 的点赞状态",
                if link_id.is_empty() {
                    "未提供 ID"
                } else {
                    link_id
                }
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args_json).map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("参数解析失败: {}", e),
        })?;
        let link_id = v.get("link_id").and_then(|s| s.as_str()).unwrap_or("");
        if link_id.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "link_id 不能为空".into(),
            });
        }
        api_inter::like_post(client, link_id, 1)
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;
        Ok(json!({"ok": true, "message": "点赞成功"}).to_string())
    }
}

/// 收藏工具
pub struct FavouriteTool;

#[async_trait]
impl Tool for FavouriteTool {
    fn name(&self) -> &'static str {
        "favourite"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "favourite",
            "收藏某个帖子（toggle 切换式）。",
            json!({
                "type": "object",
                "properties": {
                    "link_id": {"type": "string", "description": "目标帖子 ID"},
                    "folder_id": {
                        "type": "string",
                        "description": "（可选）收藏夹 ID，默认收藏夹可省略"
                    }
                },
                "required": ["link_id"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let link_id = arg_str(&v, "link_id");
        let folder_id = arg_str(&v, "folder_id");
        let target = if folder_id.is_empty() {
            format!(
                "帖子 {}",
                if link_id.is_empty() {
                    "未提供 ID"
                } else {
                    link_id
                }
            )
        } else {
            format!(
                "帖子 {} 到收藏夹 {}",
                if link_id.is_empty() {
                    "未提供 ID"
                } else {
                    link_id
                },
                folder_id
            )
        };
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::Medium,
            summary: format!("切换{}的收藏状态", target),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args_json).map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("参数解析失败: {}", e),
        })?;
        let link_id = v.get("link_id").and_then(|s| s.as_str()).unwrap_or("");
        let folder_id = v.get("folder_id").and_then(|s| s.as_str()).unwrap_or("");
        if link_id.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "link_id 不能为空".into(),
            });
        }
        let folder = if folder_id.is_empty() {
            None
        } else {
            Some(folder_id)
        };
        api_inter::toggle_favourite(client, link_id, folder)
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;
        Ok(json!({"ok": true, "message": "收藏切换成功"}).to_string())
    }
}

/// 抓取最新帖子（让 LLM 拿到上下文）
pub struct SearchFeedsTool;

#[async_trait]
impl Tool for SearchFeedsTool {
    fn name(&self) -> &'static str {
        "search_feeds"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "search_feeds",
            "获取最新帖子列表，用于了解当前热门内容或选择目标帖子。",
            json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "返回条数，默认 5，最大 20",
                        "default": 5
                    }
                }
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args_json: &str) -> Result<String> {
        let v: Value = if args_json.is_empty() {
            json!({})
        } else {
            serde_json::from_str(args_json).unwrap_or(json!({}))
        };
        let limit = v.get("limit").and_then(|i| i.as_u64()).unwrap_or(5).min(20) as u32;
        let resp = api_feed::feeds(
            client,
            api_feed::FeedsQuery {
                page: Some(1),
                limit: Some(limit),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: e.to_string(),
        })?;

        let links = resp
            .pointer("/result/links")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let summary: Vec<Value> = links
            .iter()
            .take(limit as usize)
            .map(|l| {
                json!({
                    "link_id": l.get("linkid"),
                    "title": l.get("title"),
                    "comment_num": l.get("comment_num"),
                    "author": l.pointer("/user/username"),
                    "topic": l.pointer("/topics/0/name"),
                })
            })
            .collect();
        Ok(json!({"feeds": summary, "count": summary.len()}).to_string())
    }
}

/// 按社区/板块拉取帖子
pub struct CommunityFeedsTool;

#[async_trait]
impl Tool for CommunityFeedsTool {
    fn name(&self) -> &'static str {
        "community_feeds"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "community_feeds",
            "按社区 topic_id 拉取该板块的帖子列表。用 search_community 拿到 topic_id 后，用本工具浏览该社区的热门帖子。",
            json!({
                "type": "object",
                "properties": {
                    "topic_id": {"type": "string", "description": "社区 topic_id（从 search_community 获取）"},
                    "limit":    {"type": "integer", "description": "返回条数，默认 5，最大 20", "default": 5}
                },
                "required": ["topic_id"]
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let topic_id = v.get("topic_id").and_then(|s| s.as_str()).unwrap_or("");
        if topic_id.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "topic_id 不能为空".into(),
            });
        }
        let tid: u32 = topic_id.parse().map_err(|_| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("topic_id 必须是数字: {}", topic_id),
        })?;
        let limit = v.get("limit").and_then(|i| i.as_u64()).unwrap_or(5).min(20) as u32;
        let resp = api_feed::community_feeds(
            client,
            tid,
            api_feed::CommunityFeedsQuery {
                limit: Some(limit),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: e.to_string(),
        })?;
        let links = resp
            .pointer("/result/links")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let summary: Vec<Value> = links
            .iter()
            .take(limit as usize)
            .map(|l| {
                json!({
                    "link_id":     l.get("linkid"),
                    "title":       l.get("title"),
                    "comment_num": l.get("comment_num"),
                    "author":      l.pointer("/user/username"),
                    "topic":       l.pointer("/topics/0/name"),
                    "up":          l.get("up"),
                    "create_at":   l.get("create_at"),
                })
            })
            .collect();
        Ok(json!({"feeds": summary, "count": summary.len(), "topic_id": topic_id}).to_string())
    }
}

// ─── EditPostTool / DeletePostTool / DeleteCommentTool / LikeCommentTool ────

/// 编辑帖子
pub struct EditPostTool;

#[async_trait]
impl Tool for EditPostTool {
    fn name(&self) -> &'static str {
        "edit_post"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "edit_post",
            "编辑已有帖子的标题和内容。",
            json!({
                "type": "object",
                "properties": {
                    "link_id":   {"type": "string", "description": "目标帖子 ID"},
                    "title":     {"type": "string", "description": "新标题"},
                    "content":   {"type": "string", "description": "新正文"},
                    "hashtags":  {"type": "array", "items": {"type": "string"}, "description": "话题标签"},
                    "topic_ids": {"type": "array", "items": {"type": "string"}, "description": "话题 ID 列表，留空用默认"}
                },
                "required": ["link_id", "title", "content"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let link_id = arg_str(&v, "link_id");
        let title = arg_str(&v, "title");
        let content = arg_str(&v, "content");
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::High,
            summary: format!(
                "编辑帖子 {}，新标题《{}》，正文预览：{}",
                if link_id.is_empty() {
                    "未提供 ID"
                } else {
                    link_id
                },
                if title.is_empty() {
                    "未提供标题"
                } else {
                    title
                },
                truncate_chars(content, 80)
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let link_id = v.get("link_id").and_then(|s| s.as_str()).unwrap_or("");
        let title = v.get("title").and_then(|s| s.as_str()).unwrap_or("");
        let content = v.get("content").and_then(|s| s.as_str()).unwrap_or("");
        let topic_ids: Vec<String> = v
            .get("topic_ids")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["58144".into()]);
        let hashtags: Vec<String> = v
            .get("hashtags")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let req = api_post::EditPostReq {
            link_id: link_id.into(),
            title: title.into(),
            content: content.into(),
            topic_ids,
            hashtags,
            link_tag: 28,
        };
        let resp = api_post::edit_post(client, req).await?;
        Ok(json!({"ok": resp.get("status").and_then(|s| s.as_str()) == Some("ok"), "response": resp}).to_string())
    }
}

/// 删帖
pub struct DeletePostTool;

#[async_trait]
impl Tool for DeletePostTool {
    fn name(&self) -> &'static str {
        "delete_post"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "delete_post",
            "删除指定帖子（不可逆）。",
            json!({
                "type": "object",
                "properties": {
                    "link_id": {"type": "string", "description": "目标帖子 ID"}
                },
                "required": ["link_id"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let link_id = arg_str(&v, "link_id");
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::High,
            summary: format!(
                "删除帖子 {}。此操作不可逆",
                if link_id.is_empty() {
                    "未提供 ID"
                } else {
                    link_id
                }
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let link_id = v.get("link_id").and_then(|s| s.as_str()).unwrap_or("");
        let resp = api_post::delete_post(client, link_id).await?;
        Ok(json!({"ok": resp.get("status").and_then(|s| s.as_str()) == Some("ok"), "response": resp}).to_string())
    }
}

/// 删除评论
pub struct DeleteCommentTool;

#[async_trait]
impl Tool for DeleteCommentTool {
    fn name(&self) -> &'static str {
        "delete_comment"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "delete_comment",
            "删除指定评论（仅限自己的评论）。调用前必须先查询评论所属帖子，并提供帖子 ID 与根评论 ID，用于删除前从 API 精确验证目标评论。",
            json!({
                "type": "object",
                "properties": {
                    "comment_id": {"type": "string", "description": "目标评论 ID"},
                    "link_id": {"type": "string", "description": "评论所属帖子 ID"},
                    "root_comment_id": {"type": "string", "description": "目标评论所属根评论 ID；删除根评论时与 comment_id 相同"}
                },
                "required": ["comment_id", "link_id", "root_comment_id"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let cid = arg_str(&v, "comment_id");
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::High,
            summary: format!(
                "删除评论 {}。此操作不可逆",
                if cid.is_empty() { "未提供 ID" } else { cid }
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let cid = v.get("comment_id").and_then(|s| s.as_str()).unwrap_or("");
        let resp = api_comment::delete_comment(client, cid).await?;
        Ok(json!({"ok": resp.get("status").and_then(|s| s.as_str()) == Some("ok"), "response": resp}).to_string())
    }
}

/// 评论点赞/取消点赞（toggle）
pub struct LikeCommentTool;

#[async_trait]
impl Tool for LikeCommentTool {
    fn name(&self) -> &'static str {
        "like_comment"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "like_comment",
            "点赞某条评论（toggle 切换式：已赞则取消，未赞则赞）。",
            json!({
                "type": "object",
                "properties": {
                    "comment_id": {"type": "string", "description": "目标评论 ID"}
                },
                "required": ["comment_id"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let cid = arg_str(&v, "comment_id");
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::Medium,
            summary: format!(
                "切换评论 {} 的点赞状态",
                if cid.is_empty() { "未提供 ID" } else { cid }
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let cid = v.get("comment_id").and_then(|s| s.as_str()).unwrap_or("");
        let resp = api_inter::toggle_like_comment(client, cid).await?;
        Ok(json!({"ok": resp.get("status").and_then(|s| s.as_str()) == Some("ok"), "message": "评论点赞切换"}).to_string())
    }
}

// ─── SearchPostsTool / MyPostsTool / UserProfileTool / UploadImageTool ────

/// 查看我的帖子列表（当前用户的发帖动态）
pub struct MyPostsTool;

#[async_trait]
impl Tool for MyPostsTool {
    fn name(&self) -> &'static str {
        "my_posts"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "my_posts",
            "查看当前登录用户的帖子列表。传入 lastval 可翻页（上次返回的 lastval 字段）。",
            json!({
                "type": "object",
                "properties": {
                    "lastval": {"type": "string", "description": "翻页游标，首页留空"}
                }
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = if args.is_empty() {
            json!({})
        } else {
            serde_json::from_str(args)?
        };
        let lastval = v.get("lastval").and_then(|s| s.as_str()).unwrap_or("");
        let resp = api_feed::user_events(client, None, Some(lastval)).await?;
        let moments = resp
            .pointer("/result/moments")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        let last = resp
            .pointer("/result/lastval")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let list: Vec<Value> = moments
            .iter()
            .map(|m| {
                json!({
                    "link_id":     m.get("linkid"),
                    "title":       m.get("title"),
                    "comment_num": m.get("comment_num"),
                    "up":          m.get("up"),
                    "create_at":   m.get("create_at"),
                    "topic":       m.pointer("/topics/0/name"),
                })
            })
            .collect();
        Ok(json!({"count": list.len(), "posts": list, "lastval": last}).to_string())
    }
}

/// 查看帖子详情（正文、作者、评论数、点赞数等）
pub struct PostDetailTool;

#[async_trait]
impl Tool for PostDetailTool {
    fn name(&self) -> &'static str {
        "post_detail"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "post_detail",
            "查看帖子详情（正文内容、作者、评论数、点赞数、所属板块等）。",
            json!({
                "type": "object",
                "properties": {
                    "link_id": {"type": "string", "description": "帖子 ID（如 182461976）"}
                },
                "required": ["link_id"]
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let link_id = v.get("link_id").and_then(|s| s.as_str()).unwrap_or("");
        if link_id.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "link_id 不能为空".into(),
            });
        }
        let resp = api_feed::post_detail(client, link_id, Default::default())
            .await
            .map_err(|e| Error::ToolCall {
                tool: self.name().into(),
                msg: e.to_string(),
            })?;
        let link = resp.pointer("/result/link").cloned().unwrap_or(Value::Null);
        Ok(json!({
            "link_id":     link.get("linkid"),
            "title":       link.get("title"),
            "description": link.get("description"),
            "text":        link.get("text"),
            "author":      link.pointer("/user/username"),
            "author_id":   link.get("userid"),
            "comment_num": link.get("comment_num"),
            "up":          link.get("up"),
            "create_at":   link.get("create_at"),
            "topic":       link.pointer("/topics/0/name"),
            "hashtags":    link.get("hashtags"),
            "is_up":       link.get("is_up"),
            "is_favour":   link.get("is_favour"),
        })
        .to_string())
    }
}

/// 通用搜索帖子（7 种 search_type）
pub struct SearchPostsTool;

#[async_trait]
impl Tool for SearchPostsTool {
    fn name(&self) -> &'static str {
        "search_posts"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "search_posts",
            "通用搜索，支持搜索帖子内容/用户/游戏/话题/商品。search_type 可选：综合/内容/用户/游戏/话题/商品/小程序。传入 topic_id 可限定搜索范围到指定社区。",
            json!({
                "type": "object",
                "properties": {
                    "q":           {"type": "string", "description": "搜索关键词"},
                    "search_type": {"type": "string", "description": "搜索类型，默认综合", "default": "综合"},
                    "limit":       {"type": "integer", "description": "返回条数，默认 5", "default": 5},
                    "topic_id":    {"type": "string", "description": "（可选）限定搜索范围到指定社区 topic_id（先用 search_community 获取）"}
                },
                "required": ["q"]
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let q = v.get("q").and_then(|s| s.as_str()).unwrap_or("");
        let limit = v.get("limit").and_then(|i| i.as_u64()).unwrap_or(5).min(20) as u32;
        let st_str = v
            .get("search_type")
            .and_then(|s| s.as_str())
            .unwrap_or("综合");
        let st = match st_str {
            "内容" | "content" => api_search::SearchType::Content,
            "用户" | "user" => api_search::SearchType::User,
            "游戏" | "game" => api_search::SearchType::Game,
            "话题" | "topic" => api_search::SearchType::Topic,
            "商品" | "product" => api_search::SearchType::Product,
            "小程序" | "mini" => api_search::SearchType::MiniProgram,
            _ => api_search::SearchType::Comprehensive,
        };
        let topic_id = v
            .get("topic_id")
            .and_then(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|t| *t > 0);
        let resp = api_search::search(
            client,
            api_search::SearchReq {
                q: q.into(),
                search_type: st,
                offset: 0,
                limit,
                topic_id,
            },
        )
        .await?;
        let items = resp
            .pointer("/result/items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let summary: Vec<Value> = items
            .iter()
            .take(limit as usize)
            .filter_map(|item| {
                let t = item.get("type").and_then(|s| s.as_str())?;
                let info = item.get("info")?;
                Some(json!({
                    "type": t,
                    "title":     info.get("title"),
                    "username":  info.get("username"),
                    "name":      info.get("name"),
                    "link_id":   info.get("linkid"),
                    "userid":    info.get("userid"),
                    "content_num": info.pointer("/num/content_num"),
                }))
            })
            .collect();
        Ok(json!({"count": summary.len(), "items": summary}).to_string())
    }
}

/// 查看用户主页详情
pub struct UserProfileTool;

#[async_trait]
impl Tool for UserProfileTool {
    fn name(&self) -> &'static str {
        "user_profile"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "user_profile",
            "查看某用户的主页信息（签名、等级、关注数、粉丝数、发帖数、勋章等）。",
            json!({
                "type": "object",
                "properties": {
                    "userid": {"type": "string", "description": "目标用户数字 ID"}
                },
                "required": ["userid"]
            }),
        )
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let userid = v.get("userid").and_then(|s| s.as_str()).unwrap_or("");
        let resp = api_user::user_profile(client, Some(userid)).await?;
        let d = resp
            .pointer("/result/account_detail")
            .cloned()
            .unwrap_or(Value::Null);
        Ok(json!({
            "username":     d.get("username"),
            "userid":       d.get("userid"),
            "level":        d.pointer("/level_info/level"),
            "signature":    d.get("signature"),
            "ip_location":  d.get("ip_location"),
            "follow_num":   d.pointer("/bbs_info/follow_num"),
            "fan_num":      d.pointer("/bbs_info/fan_num"),
            "post_link_num": d.pointer("/bbs_info/post_link_num"),
            "awd_num":      d.pointer("/bbs_info/awd_num"),
            "medals":       d.get("medals"),
        })
        .to_string())
    }
}

/// 上传图片到 COS 图床（四步流程）
pub struct UploadImageTool;

#[async_trait]
impl Tool for UploadImageTool {
    fn name(&self) -> &'static str {
        "upload_image"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "upload_image",
            "上传图片到小黑盒 COS 图床，返回可直接用于评论/发帖的图片 URL。需要提供本地文件路径。",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "本地图片文件绝对路径"},
                    "mimetype":  {"type": "string", "description": "MIME 类型，默认 image/png", "default": "image/png"}
                },
                "required": ["file_path"]
            }),
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation(&self, arguments_json: &str) -> ToolConfirmation {
        let v = parsed_args(arguments_json);
        let file_path = arg_str(&v, "file_path");
        ToolConfirmation {
            tool_name: self.name(),
            risk_level: RiskLevel::High,
            summary: format!(
                "上传本地图片文件 {} 到小黑盒图床",
                if file_path.is_empty() {
                    "未提供路径"
                } else {
                    file_path
                }
            ),
            arguments_json: arguments_json.to_string(),
        }
    }

    async fn execute(&self, client: &XhhClient, args: &str) -> Result<String> {
        let v: Value = serde_json::from_str(args)?;
        let file_path = v.get("file_path").and_then(|s| s.as_str()).unwrap_or("");
        let mimetype = v
            .get("mimetype")
            .and_then(|s| s.as_str())
            .unwrap_or("image/png");
        if file_path.is_empty() {
            return Err(Error::ToolCall {
                tool: self.name().into(),
                msg: "file_path 不能为空".into(),
            });
        }
        let bytes = std::fs::read(file_path).map_err(|e| Error::ToolCall {
            tool: self.name().into(),
            msg: format!("读取文件失败: {}", e),
        })?;
        let name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        // 尝试解码图片获取尺寸，失败则用默认值
        let (w, h) = image::guess_format(&bytes)
            .ok()
            .and_then(|_| image::load_from_memory(&bytes).ok())
            .map(|img| (img.width(), img.height()))
            .unwrap_or((0, 0));
        let result = api_upload::upload_image_bytes(client, &bytes, name, mimetype, w, h).await?;
        Ok(json!({
            "ok": true,
            "url": result.preview_url,
            "key": result.key,
            "width": w,
            "height": h,
        })
        .to_string())
    }
}

// 注：api_search 已被 SearchCommunityTool / SearchTopicTool / SearchPostsTool 使用
// 注：api_upload 已被 UploadImageTool 使用

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default_tools() {
        let reg = ToolRegistry::with_defaults();
        let names: Vec<&str> = reg.names();
        // 查询类
        assert!(names.contains(&"search_community"));
        assert!(names.contains(&"search_topic"));
        assert!(names.contains(&"search_feeds"));
        assert!(names.contains(&"search_posts"));
        assert!(names.contains(&"user_profile"));
        assert!(names.contains(&"post_detail"));
        // 帖子
        assert!(names.contains(&"create_post"));
        assert!(names.contains(&"edit_post"));
        assert!(names.contains(&"delete_post"));
        // 评论
        assert!(names.contains(&"reply_comment"));
        assert!(names.contains(&"delete_comment"));
        // 互动
        assert!(names.contains(&"like_post"));
        assert!(names.contains(&"like_comment"));
        assert!(names.contains(&"favourite"));
        // 上传
        assert!(names.contains(&"upload_image"));
        assert_eq!(reg.names().len(), 17);
    }

    #[test]
    fn specs_are_valid_json_schema() {
        let reg = ToolRegistry::with_defaults();
        assert_eq!(reg.specs().len(), 17);
        for s in reg.specs() {
            assert!(!s.name.is_empty());
            assert!(s.parameters.is_object());
        }
    }

    #[test]
    fn lookup_by_name() {
        let reg = ToolRegistry::with_defaults();
        assert!(reg.get("create_post").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn dangerous_tools_require_confirmation() {
        let reg = ToolRegistry::with_defaults();
        let dangerous = [
            "create_post",
            "edit_post",
            "delete_post",
            "reply_comment",
            "delete_comment",
            "like_post",
            "like_comment",
            "favourite",
            "upload_image",
        ];
        for name in dangerous {
            assert!(
                reg.get(name).unwrap().requires_confirmation(),
                "{} 应要求确认",
                name
            );
        }
        assert!(!reg.get("search_feeds").unwrap().requires_confirmation());
        assert!(!reg.get("post_detail").unwrap().requires_confirmation());
    }

    #[test]
    fn create_post_confirmation_contains_summary() {
        let tool = CreatePostTool;
        let confirmation = tool.confirmation(
            r#"{"title":"测试标题","content":"测试正文内容","community_topic_id":"123"}"#,
        );
        assert_eq!(confirmation.tool_name, "create_post");
        assert_eq!(confirmation.risk_level, RiskLevel::High);
        assert!(confirmation.summary.contains("测试标题"));
        assert!(confirmation.summary.contains("topic_id=123"));
    }

    #[test]
    fn delete_comment_requires_preview_fields() {
        let spec = DeleteCommentTool.spec();
        let required = spec.parameters["required"].as_array().unwrap();
        for field in ["comment_id", "link_id", "root_comment_id"] {
            assert!(
                required.iter().any(|value| value.as_str() == Some(field)),
                "{} 应为删除评论必填参数",
                field
            );
        }
    }
}
