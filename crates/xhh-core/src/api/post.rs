//! 发帖 / 编辑 / 草稿 / 删帖 / 视频帖
//!

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

/// 富文本块（text 字段基础元素）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// 文本块
    #[serde(rename = "text")]
    Text { text: String },
    /// 图片块
    #[serde(rename = "img")]
    Img {
        /// 图片 URL（CDN 地址）
        url: String,
        /// 图片描述/路径（web 端传 CDN URL）
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// 图片宽度（字符串类型，如 "800"）
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<String>,
        /// 图片高度（字符串类型，如 "600"）
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<String>,
    },
    /// 视频块
    #[serde(rename = "video")]
    Video { url: String },
}

/// 把 `Vec<ContentBlock>` 或纯字符串 → JSON 字符串（用于 body.text 字段）
pub fn encode_content(content: &ContentInput) -> String {
    match content {
        ContentInput::Plain(s) => serde_json::to_string(&[ContentBlock::Text { text: s.clone() }])
            .unwrap_or_else(|_| "[]".to_string()),
        ContentInput::Blocks(b) => serde_json::to_string(b).unwrap_or_else(|_| "[]".to_string()),
    }
}

/// 内容输入：要么是纯文本，要么是富文本块数组
#[derive(Debug, Clone)]
pub enum ContentInput {
    Plain(String),
    Blocks(Vec<ContentBlock>),
}

impl From<String> for ContentInput {
    fn from(s: String) -> Self {
        Self::Plain(s)
    }
}

impl From<&str> for ContentInput {
    fn from(s: &str) -> Self {
        Self::Plain(s.to_string())
    }
}

impl From<Vec<ContentBlock>> for ContentInput {
    fn from(b: Vec<ContentBlock>) -> Self {
        Self::Blocks(b)
    }
}

/// 发帖请求参数
#[derive(Debug, Clone)]
pub struct CreatePostReq {
    /// 标题（≤30 字）
    pub title: String,
    /// 正文
    pub content: ContentInput,
    /// 话题 ID（默认 `["58144"]` = 盒友杂谈，社区模式填社区 topic_id）
    pub topic_ids: Vec<String>,
    /// 话题标签名列表（如 `["原神"]`），用作 hashtag
    pub hashtags: Vec<String>,
    /// 分区 ID（默认 28=盒友杂谈，社区模式 27）
    pub link_tag: i64,
    /// 可见范围（1=所有人）
    pub view_limit: i64,
    /// 1=原创
    pub original: i64,
}

impl Default for CreatePostReq {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: ContentInput::Plain(String::new()),
            topic_ids: vec!["58144".into()],
            hashtags: Vec::new(),
            link_tag: 28,
            view_limit: 1,
            original: 1,
        }
    }
}

/// 编辑帖请求
#[derive(Debug, Clone)]
pub struct EditPostReq {
    pub link_id: String,
    pub title: String,
    pub content: ContentInput,
    pub topic_ids: Vec<String>,
    pub hashtags: Vec<String>,
    pub link_tag: i64,
}

/// 草稿请求
#[derive(Debug, Clone)]
pub struct DraftReq {
    pub title: String,
    pub content: ContentInput,
    pub topic_ids: Vec<String>,
    pub link_tag: i64,
}

/// 视频帖请求
#[derive(Debug, Clone)]
pub struct CreateVideoPostReq {
    pub title: String,
    pub video_url: String,
    /// 附加文字（可选）
    pub content: Option<String>,
    pub topic_ids: Vec<String>,
    pub link_tag: i64,
}

const PATH_POST: &str = "/bbs/app/api/link/post";
const PATH_VIDEO_POST: &str = "/bbs/app/api/video-link/post";
const PATH_DELETE: &str = "/bbs/app/link/delete";

/// 发图文帖
pub async fn create_post(client: &XhhClient, req: CreatePostReq) -> Result<Value> {
    tracing::info!(title = %req.title, topic_ids = ?req.topic_ids, hashtags = ?req.hashtags, link_tag = req.link_tag, "发帖");
    let body = build_post_body(
        &req.title,
        &req.content,
        &req.topic_ids,
        &req.hashtags,
        req.link_tag,
        req.view_limit,
        req.original,
    );
    tracing::debug!(body_keys = ?body.keys().collect::<Vec<_>>(), "发帖 body");
    client.post(PATH_POST, &body, 1).await
}

/// 编辑帖子
pub async fn edit_post(client: &XhhClient, req: EditPostReq) -> Result<Value> {
    tracing::info!(link_id = %req.link_id, title = %req.title, "编辑帖子");
    let mut body = build_post_body(
        &req.title,
        &req.content,
        &req.topic_ids,
        &req.hashtags,
        req.link_tag,
        1,
        1,
    );
    body.insert("edit".into(), "1".into());
    body.insert("link_id".into(), req.link_id.clone());
    client.post(PATH_POST, &body, 1).await
}

/// 保存草稿
pub async fn save_draft(client: &XhhClient, req: DraftReq) -> Result<Value> {
    tracing::info!(title = %req.title, "保存草稿");
    let mut body = build_post_body(
        &req.title,
        &req.content,
        &req.topic_ids,
        &[],
        req.link_tag,
        1,
        1,
    );
    body.insert("draft".into(), "1".into());
    client.post(PATH_POST, &body, 1).await
}

/// 删帖
pub async fn delete_post(client: &XhhClient, link_id: &str) -> Result<Value> {
    tracing::info!(link_id = %link_id, "删帖");
    let mut body = BTreeMap::new();
    body.insert("link_id".into(), link_id.into());
    client.post(PATH_DELETE, &body, 0).await
}

/// 发视频帖
pub async fn create_video_post(client: &XhhClient, req: CreateVideoPostReq) -> Result<Value> {
    tracing::info!(title = %req.title, video_url = %req.video_url, "发视频帖");
    let mut blocks: Vec<ContentBlock> = Vec::new();
    if let Some(text) = req.content {
        if !text.is_empty() {
            blocks.push(ContentBlock::Text { text });
        }
    }
    blocks.push(ContentBlock::Video { url: req.video_url });

    let text_json = serde_json::to_string(&blocks)?;

    let mut body = BTreeMap::new();
    body.insert("text".into(), text_json);
    body.insert("title".into(), req.title);
    body.insert("desc".into(), "".into());
    body.insert("post_type".into(), "2".into());
    body.insert("view_limit".into(), "1".into());
    body.insert("link_tag".into(), req.link_tag.to_string());
    body.insert("post_card_ids".into(), "".into());
    body.insert("topic_ids".into(), req.topic_ids.join(","));
    body.insert("original".into(), "1".into());
    body.insert("declaration".into(), "1".into());
    body.insert("extra_declaration".into(), "0".into());
    client.post(PATH_VIDEO_POST, &body, 1).await
}

/// 构造 POST body（共用于发帖/编辑/草稿）
fn build_post_body(
    title: &str,
    content: &ContentInput,
    topic_ids: &[String],
    hashtags: &[String],
    link_tag: i64,
    view_limit: i64,
    original: i64,
) -> BTreeMap<String, String> {
    let text_json = encode_content(content);
    let hashtags_json = if hashtags.is_empty() {
        "".into()
    } else {
        serde_json::to_string(hashtags).unwrap_or_default()
    };
    let mut body = BTreeMap::new();
    body.insert("text".into(), text_json);
    body.insert("title".into(), title.into());
    body.insert("desc".into(), "".into());
    body.insert("post_type".into(), "1".into());
    body.insert("view_limit".into(), view_limit.to_string());
    body.insert("link_tag".into(), link_tag.to_string());
    body.insert("post_card_ids".into(), "".into());
    body.insert("topic_ids".into(), topic_ids.join(","));
    body.insert("hashtags".into(), hashtags_json);
    body.insert("original".into(), original.to_string());
    body.insert("declaration".into(), original.to_string());
    body.insert("extra_declaration".into(), "0".into());
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_plain_content() {
        let s = encode_content(&ContentInput::Plain("hello".into()));
        assert_eq!(s, r#"[{"type":"text","text":"hello"}]"#);
    }

    #[test]
    fn encode_blocks() {
        let blocks = vec![
            ContentBlock::Text { text: "a".into() },
            ContentBlock::Img {
                url: "u".into(),
                text: None,
                width: None,
                height: None,
            },
        ];
        let s = encode_content(&ContentInput::Blocks(blocks));
        assert!(s.contains(r#""type":"text""#));
        assert!(s.contains(r#""type":"img""#));
    }

    #[test]
    fn body_has_required_keys() {
        let body = build_post_body(
            "t",
            &ContentInput::Plain("c".into()),
            &["58144".into()],
            &["hashtag".into()],
            28,
            1,
            1,
        );
        assert_eq!(body.get("title").unwrap(), "t");
        assert_eq!(body.get("post_type").unwrap(), "1");
        assert_eq!(body.get("link_tag").unwrap(), "28");
        assert_eq!(body.get("topic_ids").unwrap(), "58144");
        assert!(body.get("hashtags").unwrap().contains("hashtag"));
        assert_eq!(body.get("view_limit").unwrap(), "1");
    }
}
