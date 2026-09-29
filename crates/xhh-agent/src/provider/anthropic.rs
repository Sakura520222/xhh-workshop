//! Anthropic Claude Provider
//!
//! 与 OpenAI 协议的差异：
//! - `system` 单独放在请求 body，不进 messages
//! - assistant 的工具调用通过 content blocks 的 `tool_use` 类型表达
//! - tool 消息通过 user 角色 + `tool_result` content block 表达

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::provider::{ChatMessage, ChatResponse, LlmProvider, Role, ToolCall, ToolSpec};

const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const DEFAULT_TIMEOUT_SECS: u64 = 120;
const LIST_MODELS_TIMEOUT_SECS: u64 = 30;

/// Anthropic Claude 配置
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub timeout_secs: u64,
    /// 最大输出 tokens（Claude 必须显式指定）
    pub max_tokens: u32,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "claude-haiku-4-5-20251001".into(),
            base_url: "https://api.anthropic.com".into(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_tokens: 4096,
        }
    }
}

pub struct AnthropicProvider {
    cfg: AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(cfg: AnthropicConfig) -> Result<Self> {
        if cfg.api_key.is_empty() {
            return Err(Error::Config("Anthropic api_key 不能为空".into()));
        }
        let base_url = normalize_base_url(&cfg.base_url);
        let client = Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()?;
        Ok(Self {
            cfg: AnthropicConfig { base_url, ..cfg },
            client,
        })
    }
}

/// 规范化 base_url：去掉首尾空白与结尾斜杠；末段若已是版本号则剥掉，避免拼 `/v1/messages` 时重复
pub fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return AnthropicConfig::default().base_url;
    }
    let seg = trimmed.rsplit('/').next().unwrap_or("");
    let ver = seg.strip_prefix('v').unwrap_or("");
    if !ver.is_empty() && ver.chars().all(|c| c.is_ascii_digit()) {
        return trimmed[..trimmed.len() - seg.len()]
            .trim_end_matches('/')
            .to_string();
    }
    trimmed.to_string()
}

/// 拉取模型列表（GET /v1/models），按字母序返回
pub async fn list_models(api_key: &str, base_url: &str, timeout_secs: u64) -> Result<Vec<String>> {
    let url = format!("{}/v1/models", normalize_base_url(base_url));
    let client = Client::builder()
        .timeout(Duration::from_secs(if timeout_secs == 0 {
            LIST_MODELS_TIMEOUT_SECS
        } else {
            timeout_secs
        }))
        .build()?;
    let resp = client
        .get(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_API_VERSION)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(Error::Provider(format!(
            "Anthropic HTTP {} - {}",
            status,
            truncate(&text, 300)
        )));
    }
    let v: Value = serde_json::from_str(&text)?;
    let mut models: Vec<String> = v
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
                .map(String::from)
                .collect()
        })
        .ok_or_else(|| Error::Provider(format!("响应缺少 data 字段: {}", truncate(&text, 200))))?;
    models.sort();
    Ok(models)
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }
    fn model(&self) -> &str {
        &self.cfg.model
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSpec>,
        temperature: Option<f32>,
    ) -> Result<ChatResponse> {
        let url = format!("{}/v1/messages", self.cfg.base_url.trim_end_matches('/'));
        tracing::debug!(provider = "anthropic", model = %self.cfg.model, msg_count = messages.len(), tool_count = tools.len(), "LLM 请求");

        // 提取 system 消息
        let system_text: String = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");

        // 转换非 system 消息为 Claude content blocks 风格
        let mapped: Vec<Value> = messages
            .into_iter()
            .filter(|m| m.role != Role::System)
            .map(|m| map_message_to_anthropic(&m))
            .collect::<Result<_>>()?;

        let mut body = json!({
            "model": self.cfg.model,
            "max_tokens": self.cfg.max_tokens,
            "messages": mapped,
        });
        if !system_text.is_empty() {
            body["system"] = json!(system_text);
        }
        if let Some(t) = temperature {
            body["temperature"] = json!(t);
        }
        if !tools.is_empty() {
            body["tools"] = json!(tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect::<Vec<_>>());
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            tracing::error!(provider = "anthropic", status = %status, body = %truncate(&text, 500), "Anthropic API 错误");
            return Err(Error::Provider(format!(
                "Anthropic HTTP {} - {}",
                status,
                truncate(&text, 500)
            )));
        }

        let v: Value = serde_json::from_str(&text)?;
        parse_anthropic_response(&v)
    }
}

/// 把统一消息 → Claude messages API 风格
fn map_message_to_anthropic(m: &ChatMessage) -> Result<Value> {
    match m.role {
        Role::User if !m.images.is_empty() => {
            let mut blocks = Vec::new();
            for img in &m.images {
                blocks.push(json!({
                    "type": "image",
                    "source": {"type": "url", "url": img}
                }));
            }
            if !m.content.is_empty() {
                blocks.push(json!({"type": "text", "text": &m.content}));
            }
            Ok(json!({"role": "user", "content": blocks}))
        }
        Role::User => Ok(json!({
            "role": "user",
            "content": m.content,
        })),
        Role::Assistant => {
            if m.tool_calls.is_empty() {
                Ok(json!({
                    "role": "assistant",
                    "content": m.content,
                }))
            } else {
                let mut blocks = Vec::new();
                if !m.content.is_empty() {
                    blocks.push(json!({"type": "text", "text": m.content}));
                }
                for tc in &m.tool_calls {
                    let input: Value = if tc.arguments.is_empty() {
                        json!({})
                    } else {
                        serde_json::from_str(&tc.arguments).unwrap_or(json!({}))
                    };
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": input,
                    }));
                }
                Ok(json!({
                    "role": "assistant",
                    "content": blocks,
                }))
            }
        }
        Role::Tool => {
            // Claude: tool_result 通过 user 角色 + tool_result block
            let tool_use_id = m.tool_call_id.as_deref().unwrap_or("");
            Ok(json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": m.content,
                }],
            }))
        }
        Role::System => unreachable!("system messages 已在调用前过滤"),
    }
}

fn parse_anthropic_response(v: &Value) -> Result<ChatResponse> {
    let content = v.get("content").and_then(|c| c.as_array()).ok_or_else(|| {
        Error::Provider(format!(
            "Anthropic 响应缺少 content: {}",
            truncate(&v.to_string(), 200)
        ))
    })?;

    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for block in content {
        match block.get("type").and_then(|v| v.as_str()) {
            Some("text") => {
                if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                    text_parts.push(t.to_string());
                }
            }
            Some("tool_use") => {
                let id = block
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = block
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let input = block.get("input").cloned().unwrap_or(json!({}));
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments: serde_json::to_string(&input).unwrap_or_else(|_| "{}".into()),
                });
            }
            _ => {}
        }
    }

    let text = text_parts.join("\n");
    if text.is_empty() && tool_calls.is_empty() {
        tracing::warn!(provider = "anthropic", "LLM 返回空内容");
        return Err(Error::EmptyResponse);
    }
    tracing::debug!(
        provider = "anthropic",
        content_len = text.len(),
        tool_calls_len = tool_calls.len(),
        "LLM 响应"
    );
    Ok(ChatResponse {
        content: text,
        tool_calls,
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}
