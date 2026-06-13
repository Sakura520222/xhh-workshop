//! Agent 配置（含 provider 选择、密钥、限额）

use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::provider::{AnthropicConfig, OllamaConfig, OpenAiConfig};

/// Agent 配置（持久化到 ~/.xiaoheihe/agent.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 当前活跃 provider 名（"openai" / "anthropic" / "ollama" / "xhh-backend"）
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
    /// 每日最多调用次数（默认 10）
    ///
    /// **仅当 `quota_enforced=true` 时生效**。
    /// 用户自配置的 Provider（openai/anthropic/ollama）默认不消耗配额。
    #[serde(default = "default_max_per_day")]
    pub max_per_day: u32,
    /// 是否启用配额计费。
    ///
    /// - `false`（默认）：用户自配置的本地/第三方 Provider，不消耗任何配额，调用次数无限制
    /// - `true`：使用后端 AI 服务（"xhh-backend"）时启用配额，按每日 max_per_day 限制
    #[serde(default)]
    pub quota_enforced: bool,
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
fn default_max_per_day() -> u32 {
    10
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
            max_per_day: default_max_per_day(),
            quota_enforced: false,
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

/// 每日使用计数（持久化）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyCounters {
    /// 今日日期（YYYY-MM-DD UTC），用来判断是否需要重置
    #[serde(default)]
    pub date: String,
    /// 当日已调用次数
    #[serde(default)]
    pub count: u32,
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
            return Ok(Self {
                max_loops: default_max_loops(),
                max_per_day: default_max_per_day(),
                ..Self::default()
            });
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

impl DailyCounters {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(default_counters_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&content).map_err(Into::into)
    }

    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(default_counters_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// 累加一次（自动按 UTC 日期重置）
    pub fn increment(&mut self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        if self.date != today {
            self.date = today;
            self.count = 0;
        }
        self.count += 1;
    }

    /// 当日还剩多少配额
    pub fn remaining(&self, max_per_day: u32) -> u32 {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        if self.date != today {
            return max_per_day;
        }
        max_per_day.saturating_sub(self.count)
    }

    /// 检查是否达到今日上限
    pub fn check_limit(&self, max_per_day: u32) -> Result<()> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let used = if self.date == today { self.count } else { 0 };
        if used >= max_per_day {
            return Err(Error::DailyLimitReached(max_per_day));
        }
        // 实际记录时间需要真实时间，下面是用于反警告的辅助
        let _ = Utc.timestamp_opt(0, 0);
        Ok(())
    }
}

fn default_counters_path() -> PathBuf {
    AgentConfig::config_dir().join("agent_counters.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_agent_config() {
        let c = AgentConfig::default();
        assert_eq!(c.max_loops, 8);
        assert_eq!(c.max_per_day, 10);
        assert!(c.confirm_dangerous_tools);

        let deserialized: AgentConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(deserialized.max_loops, c.max_loops);
        assert_eq!(deserialized.max_per_day, c.max_per_day);
    }

    #[test]
    fn counters_reset_on_new_day() {
        let mut c = DailyCounters {
            date: "2025-01-01".into(),
            count: 100,
        };
        c.increment();
        assert_eq!(c.count, 1);
        assert_ne!(c.date, "2025-01-01");
    }

    #[test]
    fn counters_limit_check() {
        let c = DailyCounters {
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            count: 5,
        };
        assert!(c.check_limit(10).is_ok());
        let c2 = DailyCounters {
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            count: 10,
        };
        assert!(matches!(
            c2.check_limit(10),
            Err(Error::DailyLimitReached(10))
        ));
    }

    #[test]
    fn build_provider_ollama_default() {
        let mut cfg = AgentConfig::default();
        cfg.active_provider = "ollama".into();
        cfg.ollama = Some(OllamaCfg::default());
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
