//! HTTP 客户端封装
//!
//! 持有连接池复用的 reqwest::Client 与登录凭据，提供通用
//! [`XhhClient::request`] 方法，自动注入签名 query / Cookie / Headers。

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::{Client, Method};
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::hkey::build_query_params;

/// Web 端默认 UA
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// 全局请求超时（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 复用的 HTTP 客户端 + 凭据
#[derive(Debug, Clone)]
pub struct XhhClient {
    inner: Client,
    /// 用户 ID（登录前为空字符串）
    pub heybox_id: String,
    /// 设备指纹
    pub device_id: String,
    /// 完整 Cookie 字符串
    pub cookie: String,
    /// 配置（保留以便后续更新 token / cookie）
    pub config: Config,
}

impl XhhClient {
    /// 用配置创建客户端。若配置中没有 device_id，会自动生成一个。
    ///
    /// 同时检测并修复旧版 cookie 格式（曾经用 `pkey=`，应为 `user_pkey=`），
    /// 如果发现旧格式且配置已有 pkey/heybox_id，则原地修复并写回磁盘。
    pub fn new(mut config: Config) -> Result<Self> {
        if config.device_id.is_empty() {
            config.device_id = uuid::Uuid::new_v4().simple().to_string();
            tracing::debug!(device_id = %config.device_id, "生成新设备 ID");
        }
        // 自动修复 cookie 格式：旧版（pkey=...）→ 新版（user_pkey=...）
        if config.has_credentials()
            && (!config.cookie.contains("user_pkey=") || config.cookie.starts_with("pkey="))
        {
            tracing::info!("检测到旧格式 cookie，自动修复为 user_pkey= 格式");
            if config.x_xhh_tokenid.is_empty() {
                config.x_xhh_tokenid = crate::crypto::generate_token_id();
            }
            config.cookie = format!(
                "user_pkey={}; user_heybox_id={}; x_xhh_tokenid={}",
                config.pkey, config.heybox_id, config.x_xhh_tokenid
            );
            let _ = config.save(None);
        }
        tracing::info!(heybox_id = %config.heybox_id, device_id = %config.device_id, "XhhClient 创建");
        let inner = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        Ok(Self {
            inner,
            heybox_id: config.heybox_id.clone(),
            device_id: config.device_id.clone(),
            cookie: config.cookie.clone(),
            config,
        })
    }

    /// 仅创建匿名客户端（未登录场景，如获取二维码）
    pub fn anonymous(device_id: Option<String>) -> Result<Self> {
        let cfg = Config {
            device_id: device_id.unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string()),
            ..Default::default()
        };
        Self::new(cfg)
    }

    /// 内部 reqwest::Client 引用（让 auth 模块等需要直接操作 HTTP 的地方复用连接池）
    pub fn inner(&self) -> &Client {
        &self.inner
    }

    /// 把当前 cookie/heybox_id 同步回 [`config`]，便于调用方持久化
    pub fn sync_to_config(&mut self) -> &mut Config {
        self.config.heybox_id = self.heybox_id.clone();
        self.config.cookie = self.cookie.clone();
        self.config.device_id = self.device_id.clone();
        &mut self.config
    }

    /// 通用请求方法
    ///
    /// - `method`: HTTP 方法
    /// - `path`:   API 路径，如 `/bbs/app/feeds`
    /// - `offset`: hkey 时间戳偏移（发帖类=1，其余=0）
    /// - `body`:   POST body（form-urlencoded），GET 时传 None
    /// - `extra_query`: 额外 query 参数（如 feeds 的 `pull=1`）
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        offset: i64,
        body: Option<&BTreeMap<String, String>>,
        extra_query: &[(&str, &str)],
    ) -> Result<Value> {
        tracing::debug!(method = %method, path = %path, offset = offset, "HTTP 请求");

        let app = if path.contains("/post_editor/topic_selection/") {
            "heybox"
        } else {
            "web"
        };

        let mut params = build_query_params(path, &self.heybox_id, &self.device_id, offset, app);
        for (k, v) in extra_query {
            params.push((k.to_string(), v.to_string()));
        }

        let url = format!("{}{}", crate::BASE_URL, path);

        let mut req = self
            .inner
            .request(method, &url)
            .header(reqwest::header::HOST, "api.xiaoheihe.cn")
            .header(reqwest::header::REFERER, "https://www.xiaoheihe.cn/")
            .header(reqwest::header::ORIGIN, "https://www.xiaoheihe.cn")
            .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT);
        if !self.cookie.is_empty() {
            req = req.header(reqwest::header::COOKIE, &self.cookie);
        }

        if let Some(body) = body {
            let body_str = encode_form(body);
            tracing::debug!(path = %path, body = %body_str, "POST 请求体");
            req = req
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded;charset=utf-8",
                )
                .body(body_str);
        }

        let resp = req.query(&params).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!(status = %status, path = %path, body = %text, "HTTP 非 2xx");
            return Err(Error::ApiError {
                status: status.as_u16().to_string(),
                msg: text,
            });
        }

        let value: Value = resp.json().await?;
        tracing::debug!(path = %path, status = %status, "HTTP 响应成功");
        Ok(value)
    }

    /// 简单 GET 请求（offset=0, 无 body）
    pub async fn get(&self, path: &str, extra_query: &[(&str, &str)]) -> Result<Value> {
        self.request(Method::GET, path, 0, None, extra_query).await
    }

    /// 简单 POST 请求
    pub async fn post(
        &self,
        path: &str,
        body: &BTreeMap<String, String>,
        offset: i64,
    ) -> Result<Value> {
        self.request(Method::POST, path, offset, Some(body), &[])
            .await
    }
}

/// 把 `BTreeMap<String, String>` 编码为 application/x-www-form-urlencoded 字符串
fn encode_form(map: &BTreeMap<String, String>) -> String {
    let mut encoder = form_urlencoded::Serializer::new(String::new());
    for (k, v) in map {
        encoder.append_pair(k, v);
    }
    encoder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoded_body_basic() {
        let mut m = BTreeMap::new();
        m.insert("title".into(), "你好".into());
        m.insert("link_id".into(), "123".into());
        let s = encode_form(&m);
        assert!(s.contains("title="));
        assert!(s.contains("link_id=123"));
    }

    #[test]
    fn client_anonymous_has_device_id() {
        let c = XhhClient::anonymous(None).unwrap();
        assert!(!c.device_id.is_empty());
    }

    #[test]
    fn client_with_existing_config() {
        let cfg = Config {
            heybox_id: "1".into(),
            device_id: "abc".into(),
            cookie: "pkey=x".into(),
            ..Default::default()
        };
        let c = XhhClient::new(cfg.clone()).unwrap();
        assert_eq!(c.heybox_id, "1");
        assert_eq!(c.device_id, "abc");
        assert_eq!(c.cookie, "pkey=x");
    }
}
