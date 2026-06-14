//! xhh-agent: 小黑盒 Agent 模块
//!
//! LLM 驱动的自动化能力，支持：
//! - 多 Provider（OpenAI 兼容、Anthropic Claude、Ollama 本地）
//! - 工具调用（function calling）—— 发帖 / 回复评论 / 点赞 / 收藏
//! - 多轮 Agent 主循环

#![forbid(unsafe_code)]
#![allow(missing_docs)]

pub mod config;
pub mod error;
pub mod prompt;
pub mod provider;
pub mod runner;
mod text;
pub mod tool;

pub use error::{Error, Result};
