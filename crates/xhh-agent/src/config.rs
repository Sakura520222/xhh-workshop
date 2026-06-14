//! Agent 配置（含 provider 选择、密钥）

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::provider::{AnthropicConfig, OllamaConfig, OpenAiConfig};

/// Agent 配置（持久化到 ~/.xiaoheihe/agent.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 当前活跃 provider 名（"openai" / "anthropic" / "ollama"）
    #[serde(default)]
    pub active_provider: String,
    /// OpenAI 兼容配置
    #[serde(default)]
    pub openai: Option<OpenAiCfg>,
    /// Anthropic 配置
    #[serde(default)]
    pub anthropic: Option<AnthropicCfg>,
    /// Ollama 配置
    #[serde(default)]
    pub ollama: Option<OllamaCfg>,
    /// 单次 Agent 循环最大轮数（防止死循环，默认 8）
    #[serde(default = "default_max_loops")]
    pub max_loops: u32,
    /// LLM 温度（0-2，默认 None=用 provider 默认值）
    #[serde(default)]
    pub temperature: Option<f32>,
    /// 试运行模式（不实际调用工具，仅打印）
    #[serde(default)]
    pub dry_run: bool,
    /// 危险工具执行前是否需要确认
    #[serde(default = "default_confirm_dangerous_tools")]
    pub confirm_dangerous_tools: bool,
}

fn default_max_loops() -> u32 {
    8
}
fn default_confirm_dangerous_tools() -> bool {
    true
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            active_provider: String::new(),
            openai: None,
            anthropic: None,
            ollama: None,
            max_loops: default_max_loops(),
            temperature: None,
            dry_run: false,
            confirm_dangerous_tools: true,
        }
    }
}

/// OpenAI 子配置（持久化用）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAiCfg {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

/// Anthropic 子配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnthropicCfg {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: u32,
}

/// Ollama 子配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OllamaCfg {
    pub model: String,
    pub base_url: String,
}

impl AgentConfig {
    pub fn config_dir() -> PathBuf {
        let dirs = directories::BaseDirs::new().expect("无法解析用户目录");
        dirs.config_dir().join("xhh")
    }

    pub fn default_path() -> PathBuf {
        Self::config_dir().join("agent.json")
    }

    pub fn load(path: Option<&Path>) -> Result<Self> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(Self::default_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_str(&content)?)
    }

    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(Self::default_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// 解析当前活跃 provider 的运行时配置
    pub fn build_provider_config(&self) -> Result<ProviderKind> {
        match self.active_provider.as_str() {
            "openai" | "" => {
                let c = self
                    .openai
                    .as_ref()
                    .ok_or_else(|| Error::Config("未配置 openai".into()))?;
                Ok(ProviderKind::OpenAi(OpenAiConfig {
                    api_key: c.api_key.clone(),
                    model: if c.model.is_empty() {
                        "gpt-4o-mini".into()
                    } else {
                        c.model.clone()
                    },
                    base_url: if c.base_url.is_empty() {
                        "https://api.openai.com/v1".into()
                    } else {
                        c.base_url.clone()
                    },
                    timeout_secs: 120,
                }))
            }
            "anthropic" | "claude" => {
                let c = self
                    .anthropic
                    .as_ref()
                    .ok_or_else(|| Error::Config("未配置 anthropic".into()))?;
                Ok(ProviderKind::Anthropic(AnthropicConfig {
                    api_key: c.api_key.clone(),
                    model: if c.model.is_empty() {
                        "claude-haiku-4-5-20251001".into()
                    } else {
                        c.model.clone()
                    },
                    base_url: if c.base_url.is_empty() {
                        "https://api.anthropic.com".into()
                    } else {
                        c.base_url.clone()
                    },
                    max_tokens: if c.max_tokens == 0 {
                        4096
                    } else {
                        c.max_tokens
                    },
                    timeout_secs: 120,
                }))
            }
            "ollama" => {
                let c = self
                    .ollama
                    .as_ref()
                    .ok_or_else(|| Error::Config("未配置 ollama".into()))?;
                Ok(ProviderKind::Ollama(OllamaConfig {
                    model: if c.model.is_empty() {
                        "qwen2.5:14b".into()
                    } else {
                        c.model.clone()
                    },
                    base_url: if c.base_url.is_empty() {
                        "http://localhost:11434".into()
                    } else {
                        c.base_url.clone()
                    },
                    timeout_secs: 600,
                }))
            }
            other => Err(Error::Config(format!(
                "未知 provider: {}（支持 openai/anthropic/ollama）",
                other
            ))),
        }
    }
}

/// 解析后的具体 provider 配置
#[derive(Debug, Clone)]
pub enum ProviderKind {
    OpenAi(OpenAiConfig),
    Anthropic(AnthropicConfig),
    Ollama(OllamaConfig),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_agent_config() {
        let c = AgentConfig::default();
        assert_eq!(c.max_loops, 8);
        assert!(c.confirm_dangerous_tools);

        let deserialized: AgentConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(deserialized.max_loops, c.max_loops);
    }

    #[test]
    fn build_provider_ollama_default() {
        let cfg = AgentConfig {
            active_provider: "ollama".into(),
            ollama: Some(OllamaCfg::default()),
            ..Default::default()
        };
        let kind = cfg.build_provider_config().unwrap();
        match kind {
            ProviderKind::Ollama(c) => {
                assert_eq!(c.model, "qwen2.5:14b");
                assert_eq!(c.base_url, "http://localhost:11434");
            }
            _ => panic!("应该是 Ollama"),
        }
    }
}
