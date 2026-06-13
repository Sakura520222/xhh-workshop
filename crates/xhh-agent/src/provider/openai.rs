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

/// OpenAI 兼容 Provider 配置
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// API Key（如 `sk-xxx`）
    pub api_key: String,
    /// 模型 ID（如 `gpt-4o-mini` / `deepseek-chat`）
    pub model: String,
    /// 基础 URL（不含 `/chat/completions`，例如 `https://api.openai.com/v1`）
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
        let client = Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()?;
        Ok(Self { cfg, client })
    }
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
