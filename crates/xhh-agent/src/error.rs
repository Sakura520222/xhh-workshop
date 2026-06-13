//! Agent 错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Provider 错误: {0}")]
    Provider(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("工具 {tool} 调用失败: {msg}")]
    ToolCall { tool: String, msg: String },

    #[error("工具 {0} 不存在")]
    ToolNotFound(String),

    #[error("Agent 已达到每日调用上限 ({0})")]
    DailyLimitReached(u32),

    #[error("Agent 达到最大循环次数 ({0})")]
    MaxLoopExceeded(u32),

    #[error("LLM 返回空内容或异常响应")]
    EmptyResponse,

    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),

    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core API 错误: {0}")]
    Core(#[from] xhh_core::Error),

    #[error("内部错误: {0}")]
    Internal(String),
}

/// Agent Result
pub type Result<T> = std::result::Result<T, Error>;
