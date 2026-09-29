//! OpenAI 兼容 Provider
//!
//! 通过修改 `base_url` 可支持：
//! - OpenAI: `https://api.openai.com/v1`
//! - DeepSeek: `https://api.deepseek.com/v1`
//! - Moonshot: `https://api.moonshot.cn/v1`
//! - 智谱: `https://open.bigmodel.cn/api/paas/v4`
//! - 自托管 vLLM: `http://localhost:8000/v1`

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::provider::{ChatMessage, ChatResponse, LlmProvider, Role, ToolCall, ToolSpec};

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const LIST_MODELS_TIMEOUT_SECS: u64 = 30;

/// OpenAI 兼容 Provider 配置
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// API Key（如 `sk-xxx`）
    pub api_key: String,
    /// 模型 ID（如 `gpt-4o-mini` / `deepseek-chat`）
    pub model: String,
    /// 基础 URL（不含 `/chat/completions`；可省略 `/v1`，运行时自动补全）
    pub base_url: String,
    /// 请求超时（秒），LLM 推理可能慢，默认 120
    pub timeout_secs: u64,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gpt-4o-mini".into(),
            base_url: "https://api.openai.com/v1".into(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

/// OpenAI 兼容 Provider
pub struct OpenAiProvider {
    cfg: OpenAiConfig,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(cfg: OpenAiConfig) -> Result<Self> {
        if cfg.api_key.is_empty() {
            return Err(Error::Config("OpenAI api_key 不能为空".into()));
        }
        let base_url = normalize_base_url(&cfg.base_url);
        let client = Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()?;
        Ok(Self {
            cfg: OpenAiConfig { base_url, ..cfg },
            client,
        })
    }
}

/// 路径中是否已有版本号段（v1 / v2 ...），如 Cloudflare 网关 `.../v1/{account}/openai`
fn has_version_segment(url: &str) -> bool {
    url.split('/').any(|seg| {
        let ver = seg.strip_prefix('v').unwrap_or("");
        !ver.is_empty() && ver.chars().all(|c| c.is_ascii_digit())
    })
}

/// 规范化 base_url：去掉首尾空白与结尾斜杠，末段无版本号时补 `/v1`
pub fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return OpenAiConfig::default().base_url;
    }
    if has_version_segment(trimmed) {
        trimmed.to_string()
    } else {
        format!("{}/v1", trimmed)
    }
}

/// 拉取模型列表（GET /models），按字母序返回
pub async fn list_models(api_key: &str, base_url: &str, timeout_secs: u64) -> Result<Vec<String>> {
    let url = format!("{}/models", normalize_base_url(base_url));
    let client = Client::builder()
        .timeout(Duration::from_secs(if timeout_secs == 0 {
            LIST_MODELS_TIMEOUT_SECS
        } else {
            timeout_secs
        }))
        .build()?;
    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(Error::Provider(format!(
            "OpenAI HTTP {} - {}",
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
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
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
        let url = format!(
            "{}/chat/completions",
            self.cfg.base_url.trim_end_matches('/')
        );
        tracing::debug!(provider = "openai", model = %self.cfg.model, msg_count = messages.len(), tool_count = tools.len(), "LLM 请求");

        let mut body = json!({
            "model": self.cfg.model,
            "messages": messages,
        });
        if let Some(t) = temperature {
            body["temperature"] = json!(t);
        }
        if !tools.is_empty() {
            body["tools"] = json!(tools
                .iter()
                .map(|t| json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                }))
                .collect::<Vec<_>>());
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.cfg.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            tracing::error!(provider = "openai", status = %status, body = %truncate(&text, 500), "OpenAI API 错误");
            return Err(Error::Provider(format!(
                "OpenAI HTTP {} - {}",
                status,
                truncate(&text, 500)
            )));
        }

        let value: Value = serde_json::from_str(&text)?;
        let choice = value.get("choices").and_then(|c| c.get(0)).ok_or_else(|| {
            Error::Provider(format!("响应缺少 choices[0]: {}", truncate(&text, 200)))
        })?;
        let msg = choice
            .get("message")
            .ok_or_else(|| Error::Provider("响应缺少 message".into()))?;
        let content = msg
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut tool_calls = Vec::new();
        if let Some(arr) = msg.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in arr {
                let id = tc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let func = tc.get("function").cloned().unwrap_or(Value::Null);
                let name = func
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments = func
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string();
                if !name.is_empty() {
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
            }
        }

        if content.is_empty() && tool_calls.is_empty() {
            tracing::warn!(provider = "openai", "LLM 返回空内容");
            return Err(Error::EmptyResponse);
        }

        tracing::debug!(
            provider = "openai",
            content_len = content.len(),
            tool_calls_len = tool_calls.len(),
            "LLM 响应"
        );
        Ok(ChatResponse {
            content,
            tool_calls,
        })
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}

// ─── 让 ChatMessage 自定义序列化，让 OpenAI 风格 ─────

impl Serialize for ChatMessage {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        // User + 图片 → vision 多模态格式（content 为数组）
        if self.role == Role::User && !self.images.is_empty() {
            let mut content_arr: Vec<Value> = Vec::new();
            if !self.content.is_empty() {
                content_arr.push(json!({"type": "text", "text": &self.content}));
            }
            for img in &self.images {
                content_arr.push(json!({"type": "image_url", "image_url": {"url": img}}));
            }
            let mut s = serializer.serialize_struct("ChatMessage", 2)?;
            s.serialize_field("role", "user")?;
            s.serialize_field("content", &content_arr)?;
            return s.end();
        }

        match self.role {
            Role::System | Role::User | Role::Assistant => {
                let mut s = serializer.serialize_struct("ChatMessage", 2)?;
                s.serialize_field("role", &self.role)?;
                s.serialize_field(
                    "content",
                    if self.content.is_empty() {
                        ""
                    } else {
                        &self.content
                    },
                )?;
                if !self.tool_calls.is_empty() {
                    // OpenAI: assistant.tool_calls = [{id, type:"function", function:{name, arguments}}]
                    let mapped: Vec<Value> = self
                        .tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                }
                            })
                        })
                        .collect();
                    s.serialize_field("tool_calls", &mapped)?;
                }
                s.end()
            }
            Role::Tool => {
                let mut s = serializer.serialize_struct("ChatMessage", 3)?;
                s.serialize_field("role", "tool")?;
                s.serialize_field("content", &self.content)?;
                s.serialize_field("tool_call_id", self.tool_call_id.as_deref().unwrap_or(""))?;
                s.end()
            }
        }
    }
}

// 让 Role 序列化为 OpenAI 风格
impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        })
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "system" => Self::System,
            "user" => Self::User,
            "assistant" => Self::Assistant,
            "tool" | "function" => Self::Tool,
            _ => Self::User,
        })
    }
}

// 让 ChatMessage 自定义反序列化（容错）
impl<'de> Deserialize<'de> for ChatMessage {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            role: Role,
            #[serde(default)]
            content: String,
            #[serde(default)]
            tool_calls: Vec<RawToolCall>,
            #[serde(default)]
            tool_call_id: Option<String>,
            #[serde(default)]
            name: Option<String>,
        }
        #[derive(Deserialize)]
        struct RawToolCall {
            id: Option<String>,
            #[serde(default)]
            function: Option<RawFunction>,
        }
        #[derive(Deserialize)]
        struct RawFunction {
            name: Option<String>,
            arguments: Option<String>,
        }
        let h = Helper::deserialize(deserializer)?;
        let tool_calls = h
            .tool_calls
            .into_iter()
            .filter_map(|tc| {
                let f = tc.function?;
                Some(ToolCall {
                    id: tc.id.unwrap_or_default(),
                    name: f.name.unwrap_or_default(),
                    arguments: f.arguments.unwrap_or_else(|| "{}".to_string()),
                })
            })
            .collect();
        Ok(ChatMessage {
            role: h.role,
            content: h.content,
            tool_calls,
            tool_call_id: h.tool_call_id,
            name: h.name,
            images: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_appends_v1_when_missing() {
        assert_eq!(
            normalize_base_url("https://api.firefly520.top"),
            "https://api.firefly520.top/v1"
        );
        assert_eq!(
            normalize_base_url("https://api.firefly520.top/"),
            "https://api.firefly520.top/v1"
        );
        assert_eq!(
            normalize_base_url("  https://api.firefly520.top  "),
            "https://api.firefly520.top/v1"
        );
        assert_eq!(normalize_base_url(""), "https://api.openai.com/v1");
    }

    #[test]
    fn normalize_keeps_existing_version() {
        assert_eq!(
            normalize_base_url("https://api.firefly520.top/v1"),
            "https://api.firefly520.top/v1"
        );
        assert_eq!(
            normalize_base_url("https://openrouter.ai/api/v1/"),
            "https://openrouter.ai/api/v1"
        );
        // Cloudflare 网关式：版本段在路径中部
        assert_eq!(
            normalize_base_url("https://gateway.example.com/v1/acc/openai"),
            "https://gateway.example.com/v1/acc/openai"
        );
    }

    #[test]
    fn normalize_rejects_non_version_v_words() {
        // v2ray 不是版本段，仍应补 /v1
        assert_eq!(
            normalize_base_url("https://proxy.example.com/v2ray"),
            "https://proxy.example.com/v2ray/v1"
        );
    }
}
