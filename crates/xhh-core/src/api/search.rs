//! 搜索 API
//!
//! - 通用搜索（7 种 search_type）
//! - 话题标签搜索（only_hashtag=1）
//! - 社区搜索（only_hashtag=0）
//! - 推荐话题 / 社区（topic_selection/index）
//! - 搜索发现（热搜词）
//! - 搜索欢迎页

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

const PATH_GENERAL_SEARCH: &str = "/bbs/app/api/general/search/v1";
const PATH_TOPIC_SEARCH: &str = "/bbs/app/api/post_editor/topic_selection/search";
const PATH_TOPIC_INDEX: &str = "/bbs/app/api/post_editor/topic_selection/index";
const PATH_SEARCH_FOUND: &str = "/bbs/app/api/search/found";
const PATH_WELCOME_PAGE: &str = "/bbs/app/api/search/welcome_page/v2";

/// 通用搜索的 search_type 枚举（§28）
///
/// 服务端接受的英文值，由前端 JS 源码 DJnnVajz.js tabs 定义确认。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchType {
    /// 综合
    #[serde(rename = "general")]
    Comprehensive,
    /// 内容（帖子/链接）
    #[serde(rename = "link")]
    Content,
    /// 游戏
    #[serde(rename = "game")]
    Game,
    /// 小程序
    #[serde(rename = "mini_program")]
    MiniProgram,
    /// 用户
    #[serde(rename = "user")]
    User,
    /// 话题
    #[serde(rename = "hashtag")]
    Topic,
    /// 商品
    #[serde(rename = "mall")]
    Product,
}

impl SearchType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Comprehensive => "general",
            Self::Content => "link",
            Self::Game => "game",
            Self::MiniProgram => "mini_program",
            Self::User => "user",
            Self::Topic => "hashtag",
            Self::Product => "mall",
        }
    }
}

/// 通用搜索请求
#[derive(Debug, Clone)]
pub struct SearchReq {
    pub q: String,
    pub search_type: SearchType,
    pub offset: u32,
    pub limit: u32,
    /// 限定搜索范围到指定社区（§39 方式一），None 则全局搜索
    pub topic_id: Option<u32>,
}

impl Default for SearchReq {
    fn default() -> Self {
        Self {
            q: String::new(),
            search_type: SearchType::Comprehensive,
            offset: 0,
            limit: 30,
            topic_id: None,
        }
    }
}

/// 通用搜索
pub async fn search(client: &XhhClient, req: SearchReq) -> Result<Value> {
    tracing::debug!(q = %req.q, search_type = ?req.search_type, topic_id = ?req.topic_id, "通用搜索");
    let mut params: Vec<(&str, String)> = vec![
        ("q", req.q.clone()),
        ("search_type", req.search_type.as_str().to_string()),
        ("is_pull_down", "0".to_string()),
        ("dw", "628".to_string()),
        ("offset", req.offset.to_string()),
        ("limit", req.limit.to_string()),
        ("no_more", "false".to_string()),
    ];
    if let Some(tid) = req.topic_id {
        if tid > 0 {
            params.push(("topic_id", tid.to_string()));
        }
    }
    let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    client.get(PATH_GENERAL_SEARCH, &params_ref).await
}

/// 话题标签搜索（only_hashtag=1）
pub async fn search_topic(client: &XhhClient, keyword: &str) -> Result<Value> {
    tracing::debug!(keyword = %keyword, "话题搜索");
    client
        .get(PATH_TOPIC_SEARCH, &[("q", keyword), ("only_hashtag", "1")])
        .await
}

/// 社区搜索（only_hashtag=0）
pub async fn search_community(client: &XhhClient, keyword: &str) -> Result<Value> {
    tracing::debug!(keyword = %keyword, "社区搜索");
    client
        .get(PATH_TOPIC_SEARCH, &[("q", keyword), ("only_hashtag", "0")])
        .await
}

/// 推荐话题 / 社区列表
pub async fn topic_index(client: &XhhClient) -> Result<Value> {
    tracing::debug!("获取推荐话题列表");
    client.get(PATH_TOPIC_INDEX, &[]).await
}

/// 搜索发现（热搜词）
pub async fn search_found(client: &XhhClient) -> Result<Value> {
    tracing::debug!("获取搜索发现");
    client.get(PATH_SEARCH_FOUND, &[]).await
}

/// 搜索欢迎页（推荐位）
pub async fn search_welcome_page(client: &XhhClient) -> Result<Value> {
    tracing::debug!("获取搜索欢迎页");
    client.get(PATH_WELCOME_PAGE, &[]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_type_str_mapping() {
        assert_eq!(SearchType::Comprehensive.as_str(), "general");
        assert_eq!(SearchType::Content.as_str(), "link");
        assert_eq!(SearchType::User.as_str(), "user");
        assert_eq!(SearchType::Topic.as_str(), "hashtag");
        assert_eq!(SearchType::Product.as_str(), "mall");
    }

    #[test]
    fn search_type_serde_english() {
        let j = serde_json::to_string(&SearchType::User).unwrap();
        assert_eq!(j, r#""user""#);
        let t: SearchType = serde_json::from_str(r#""user""#).unwrap();
        assert_eq!(t, SearchType::User);
    }
}
