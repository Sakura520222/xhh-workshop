//! Ollama 本地 Provider
//!
//! 调用本地 `http://localhost:11434/api/chat` 端点，
//! 协议与 OpenAI 兼容（支持 tools 字段）。

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::provider::{ChatMessage, ChatResponse, LlmProvider, Role, ToolCall, ToolSpec};

const DEFAULT_TIMEOUT_SECS: u64 = 600; // 本地推理可能很慢

/// Ollama 配置
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub model: String,
    pub base_url: String,
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:14b".into(),
            base_url: "http://localhost:11434".into(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

pub struct OllamaProvider {
    cfg: OllamaConfig,
    client: Client,
}

impl OllamaProvider {
    pub fn new(cfg: OllamaConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()?;
        Ok(Self { cfg, client })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
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
        let url = format!("{}/api/chat", self.cfg.base_url.trim_end_matches('/'));
        tracing::debug!(provider = "ollama", model = %self.cfg.model, msg_count = messages.len(), tool_count = tools.len(), "LLM 请求");

        // Ollama vision 需要在消息里带 images 字段（base64），
        // 与 OpenAI 序列化格式不同，因此手动构建消息
        let has_images = messages.iter().any(|m| !m.images.is_empty());
        let messages_val: Vec<Value> = if has_images {
            messages
                .into_iter()
                .map(|m| {
                    let role_str = match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    };
                    let mut msg = json!({"role": role_str, "content": m.content});
                    if !m.images.is_empty() {
                        // images 字段接受 data URI（由 ai_analyze 命令预下载转换）
                        let imgs: Vec<String> = m
                            .images
                            .iter()
                            .filter_map(|img| {
                                img.find(";base64,").map(|pos| img[pos + 8..].to_string())
                            })
                            .collect();
                        if !imgs.is_empty() {
                            msg["images"] = json!(imgs);
                        }
                    }
                    msg
                })
                .collect()
        } else {
            // 无图片时复用 Serialize（处理 tool_calls 等复杂情况）
            messages
                .into_iter()
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .collect()
        };

        let mut body = json!({
            "model": self.cfg.model,
            "messages": messages_val,
            "stream": false,
        });
        if let Some(t) = temperature {
            body["options"] = json!({ "temperature": t });
        }
        if !tools.is_empty() {
            body["tools"] = json!(tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect::<Vec<_>>());
        }

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            tracing::error!(provider = "ollama", status = %status, body = %truncate(&text, 500), "Ollama API 错误");
            return Err(Error::Provider(format!(
                "Ollama HTTP {} - {}",
                status,
                truncate(&text, 500)
            )));
        }

        let v: Value = serde_json::from_str(&text)?;
        let msg = v
            .get("message")
            .ok_or_else(|| Error::Provider("Ollama 响应缺少 message".into()))?;
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
                    .map(|s| s.to_string())
                    .unwrap_or_else(uuid_str);
                let func = tc.get("function").cloned().unwrap_or(Value::Null);
                let name = func
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments = func
                    .get("arguments")
                    .map(|a| match a {
                        Value::String(s) => s.clone(),
                        _ => a.to_string(),
                    })
                    .unwrap_or_else(|| "{}".into());
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
            tracing::warn!(provider = "ollama", "LLM 返回空内容");
            return Err(Error::EmptyResponse);
        }
        tracing::debug!(
            provider = "ollama",
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

fn uuid_str() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}
