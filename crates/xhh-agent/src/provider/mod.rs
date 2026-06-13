//! Provider 抽象与三个实现
//!
//! - [`openai`] — OpenAI 兼容（OpenAI / DeepSeek / Moonshot / 智谱 / 自托管 vLLM）
//! - [`anthropic`] — Anthropic Claude
//! - [`ollama`] — Ollama 本地
//!
//! 通过统一 [`LlmProvider`] trait，调用方（Agent runner）不感知具体协议。

pub mod anthropic;
pub mod ollama;
pub mod openai;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use ollama::{OllamaConfig, OllamaProvider};
pub use openai::{OpenAiConfig, OpenAiProvider};

/// 角色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    /// function-calling 的工具结果消息
    Tool,
}

/// 一条对话消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    /// 当 role=Assistant 时可能携带的 tool_calls
    pub tool_calls: Vec<ToolCall>,
    /// 当 role=Tool 时关联的 tool_call_id
    pub tool_call_id: Option<String>,
    /// 当 role=Tool 时该工具的 name
    pub name: Option<String>,
    /// 图片 URL 列表（vision 模型用，仅 role=User 时有效）
    pub images: Vec<String>,
}

impl ChatMessage {
    /// 构造一条 system 消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
            images: Vec::new(),
        }
    }

    /// 构造一条 user 消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
            images: Vec::new(),
        }
    }

    /// 构造一条 user 消息（含图片，用于 vision 模型）
    pub fn user_with_images(content: impl Into<String>, images: Vec<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
            images,
        }
    }

    /// 构造一条 assistant 消息（纯文本）
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
            images: Vec::new(),
        }
    }

    /// 构造一条 assistant 消息（含工具调用）
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls,
            tool_call_id: None,
            name: None,
            images: Vec::new(),
        }
    }

    /// 构造一条工具结果消息
    pub fn tool(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            name: Some(name.into()),
            images: Vec::new(),
        }
    }
}

/// LLM 返回的工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用唯一 ID（由 LLM 生成）
    pub id: String,
    /// 工具名
    pub name: String,
    /// 参数（JSON 字符串）
    pub arguments: String,
}

/// 工具规格（function-calling 定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema（OpenAI tools 格式）
    pub parameters: serde_json::Value,
}

impl ToolSpec {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// LLM 返回
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// 文本内容（可能是空字符串，纯工具调用）
    pub content: String,
    /// 工具调用列表（可能为空）
    pub tool_calls: Vec<ToolCall>,
}

impl ChatResponse {
    /// 是否触发了任何工具调用
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// LLM Provider 统一接口
///
/// 实现者负责把通用消息/工具集映射到自家协议、调用 API、解析响应。
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 名称（如 "openai" / "claude" / "ollama"）
    fn name(&self) -> &str;

    /// 模型 ID（如 "gpt-4o-mini" / "claude-haiku-4-5-20251001" / "qwen2.5:14b"）
    fn model(&self) -> &str;

    /// 单次对话调用
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSpec>,
        temperature: Option<f32>,
    ) -> Result<ChatResponse, crate::Error>;
}
