//! Agent 主循环
//!
//! 多轮迭代：思考 → 调用工具 → 反馈 → 再思考，直到无工具调用或达到上限。
//!
//! 使用方式：
//! ```ignore
//! use xhh_agent::runner::AgentRunner;
//! let runner = AgentRunner::new(provider, registry, client, cfg);
//! let result = runner.run("帮我发一条测试帖").await?;
//! ```

use std::time::Duration;

use serde::Serialize;
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::time::sleep;

use xhh_core::client::XhhClient;

use crate::config::{AgentConfig, DailyCounters};
use crate::error::{Error, Result};
use crate::prompt;
use crate::provider::{
    AnthropicProvider, ChatMessage, LlmProvider, OllamaProvider, OpenAiProvider,
};
use crate::text::truncate_chars;
use crate::tool::{Tool, ToolConfirmation, ToolRegistry};

/// 工具确认决策
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolConfirmationDecision {
    Allow,
    Deny,
}

/// 工具确认器
#[async_trait::async_trait]
pub trait ToolConfirmationHandler: Send + Sync {
    async fn confirm(&self, confirmation: &ToolConfirmation) -> Result<ToolConfirmationDecision>;
}

/// 终端确认器
pub struct StdinConfirmationHandler;

#[async_trait::async_trait]
impl ToolConfirmationHandler for StdinConfirmationHandler {
    async fn confirm(&self, confirmation: &ToolConfirmation) -> Result<ToolConfirmationDecision> {
        let mut stdout = io::stdout();
        stdout
            .write_all(
                format!(
                    "\n需要确认危险操作\n工具: {}\n风险: {:?}\n操作: {}\n参数: {}\n确认执行请输入 yes：",
                    confirmation.tool_name,
                    confirmation.risk_level,
                    confirmation.summary,
                    confirmation.arguments_json,
                )
                .as_bytes(),
            )
            .await?;
        stdout.flush().await?;

        let mut input = String::new();
        let mut reader = io::BufReader::new(io::stdin());
        reader.read_line(&mut input).await?;
        if matches!(input.trim(), "yes" | "y" | "YES" | "Y") {
            Ok(ToolConfirmationDecision::Allow)
        } else {
            Ok(ToolConfirmationDecision::Deny)
        }
    }
}

/// 自动允许确认器
pub struct AlwaysAllowConfirmationHandler;

#[async_trait::async_trait]
impl ToolConfirmationHandler for AlwaysAllowConfirmationHandler {
    async fn confirm(&self, _confirmation: &ToolConfirmation) -> Result<ToolConfirmationDecision> {
        Ok(ToolConfirmationDecision::Allow)
    }
}

/// 自动拒绝确认器
pub struct AlwaysDenyConfirmationHandler;

#[async_trait::async_trait]
impl ToolConfirmationHandler for AlwaysDenyConfirmationHandler {
    async fn confirm(&self, _confirmation: &ToolConfirmation) -> Result<ToolConfirmationDecision> {
        Ok(ToolConfirmationDecision::Deny)
    }
}

/// 单次 run() 的日志事件
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct AgentLog {
    pub loop_index: u32,
    pub message: String,
}

/// 一次完整 Agent 运行的结果
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct AgentResult {
    /// 最终输出（最后一轮 assistant 的文本）
    pub final_output: String,
    /// 调用的工具列表（按顺序）
    pub tool_calls: Vec<String>,
    /// 日志
    pub logs: Vec<AgentLog>,
    /// 实际消耗的轮数
    pub loops_used: u32,
}

/// Agent 主驱动器
pub struct AgentRunner {
    provider: Box<dyn LlmProvider>,
    tools: ToolRegistry,
    client: XhhClient,
    config: AgentConfig,
    counters: DailyCounters,
    confirmation_handler: Box<dyn ToolConfirmationHandler>,
}

impl AgentRunner {
    /// 从配置构建（最常用入口）
    pub fn from_config(
        config: AgentConfig,
        counters: DailyCounters,
        client: XhhClient,
    ) -> Result<Self> {
        let provider_kind = config.build_provider_config()?;
        let provider: Box<dyn LlmProvider> = match &provider_kind {
            crate::config::ProviderKind::OpenAi(c) => {
                tracing::info!(model = %c.model, base_url = %c.base_url, "初始化 OpenAI Provider");
                Box::new(OpenAiProvider::new(c.clone())?)
            }
            crate::config::ProviderKind::Anthropic(c) => {
                tracing::info!(model = %c.model, base_url = %c.base_url, "初始化 Anthropic Provider");
                Box::new(AnthropicProvider::new(c.clone())?)
            }
            crate::config::ProviderKind::Ollama(c) => {
                tracing::info!(model = %c.model, base_url = %c.base_url, "初始化 Ollama Provider");
                Box::new(OllamaProvider::new(c.clone())?)
            }
        };
        let tools = ToolRegistry::with_defaults();
        let confirmation_handler: Box<dyn ToolConfirmationHandler> =
            if config.confirm_dangerous_tools {
                Box::new(StdinConfirmationHandler)
            } else {
                Box::new(AlwaysAllowConfirmationHandler)
            };
        tracing::info!(
            tool_count = tools.names().len(),
            max_loops = config.max_loops,
            dry_run = config.dry_run,
            "Agent 初始化完成"
        );
        Ok(Self {
            provider,
            tools,
            client,
            config,
            counters,
            confirmation_handler,
        })
    }

    /// 自定义 provider 与工具集（测试用）
    pub fn new(
        provider: Box<dyn LlmProvider>,
        tools: ToolRegistry,
        client: XhhClient,
        config: AgentConfig,
        counters: DailyCounters,
    ) -> Self {
        Self {
            provider,
            tools,
            client,
            config,
            counters,
            confirmation_handler: Box::new(StdinConfirmationHandler),
        }
    }

    pub fn with_confirmation_handler(
        mut self,
        confirmation_handler: Box<dyn ToolConfirmationHandler>,
    ) -> Self {
        self.confirmation_handler = confirmation_handler;
        self
    }

    /// 通用聊天（不预设场景，由用户消息驱动）
    pub async fn chat(&mut self, user_message: &str) -> Result<AgentResult> {
        self.consume_quota().await?;
        let mut messages = vec![
            ChatMessage::system(prompt::SYSTEM_PROMPT),
            ChatMessage::user(user_message),
        ];
        self.run_loop(&mut messages).await
    }

    /// 带历史的聊天（REPL 多轮对话用），复用已有消息上下文
    ///
    /// 调用方维护 `messages`，首次调用前需包含 system prompt。
    /// 本方法追加 user message、执行循环、追加 assistant 响应。
    pub async fn chat_with_history(
        &mut self,
        messages: &mut Vec<ChatMessage>,
        user_message: &str,
    ) -> Result<AgentResult> {
        messages.push(ChatMessage::user(user_message));
        self.run_loop(messages).await
    }

    /// 一键自动发帖（用户给主题，LLM 生成并发出）
    pub async fn auto_post(&mut self, topic: &str, hashtags: &[String]) -> Result<AgentResult> {
        self.consume_quota().await?;
        let user_msg = prompt::build_auto_post_prompt(topic, hashtags);
        let mut messages = vec![
            ChatMessage::system(prompt::SYSTEM_PROMPT),
            ChatMessage::user(user_msg),
        ];
        self.run_loop(&mut messages).await
    }

    /// 自动回复某条评论（提供上下文 + 链接 ID）
    pub async fn auto_reply_comment(
        &mut self,
        link_id: &str,
        post_summary: &str,
    ) -> Result<AgentResult> {
        self.consume_quota().await?;
        let mut user_msg = prompt::build_auto_reply_prompt(post_summary);
        user_msg.push_str(&format!("\n\n目标帖子 link_id: {}", link_id));
        let mut messages = vec![
            ChatMessage::system(prompt::SYSTEM_PROMPT),
            ChatMessage::user(user_msg),
        ];
        self.run_loop(&mut messages).await
    }

    /// 消费一次配额
    ///
    /// - 用户自配置的 Provider（`quota_enforced=false`）：完全跳过，不检查、不计数、不写入
    /// - 后端 AI 服务（`quota_enforced=true`）：按"Agent 会话"计数（每会话 1 次，而非每个工具调用 1 次）
    async fn consume_quota(&mut self) -> Result<()> {
        if !self.config.quota_enforced {
            tracing::debug!("配额未启用，跳过检查");
            return Ok(());
        }
        self.counters.check_limit(self.config.max_per_day)?;
        tracing::debug!(
            used = self.counters.count,
            max = self.config.max_per_day,
            "配额检查通过"
        );
        if !self.config.dry_run {
            self.counters.increment();
            let _ = self.counters.save(None);
        }
        Ok(())
    }

    /// 主循环
    async fn run_loop(&mut self, messages: &mut Vec<ChatMessage>) -> Result<AgentResult> {
        let max_loops = self.config.max_loops.max(1);
        tracing::info!(max_loops = max_loops, "Agent 主循环启动");
        let mut result = AgentResult::default();
        let specs = self.tools.specs();

        for i in 1..=max_loops {
            let resp = self
                .provider
                .chat(messages.clone(), specs.clone(), self.config.temperature)
                .await?;

            result.loops_used = i;
            result.logs.push(AgentLog {
                loop_index: i,
                message: format!(
                    "[loop {}] LLM 返回 {} 字符文本 + {} 个工具调用",
                    i,
                    resp.content.len(),
                    resp.tool_calls.len()
                ),
            });

            if !resp.has_tool_calls() {
                tracing::info!(loop = i, "Agent 循环结束: 无工具调用");
                result.final_output = resp.content.clone();
                // 记录 assistant 响应到消息历史（REPL 多轮对话需要）
                messages.push(ChatMessage::assistant(resp.content));
                return Ok(result);
            }

            // 记录 assistant 消息（含工具调用）
            let assistant_msg =
                ChatMessage::assistant_with_tools(resp.content.clone(), resp.tool_calls.clone());
            messages.push(assistant_msg);

            // 依次执行工具调用
            for tc in &resp.tool_calls {
                result.tool_calls.push(tc.name.clone());
                let tool_log = match self.tools.get(&tc.name) {
                    Some(t) => {
                        let mut denied = false;
                        let exec_result = if self.config.dry_run {
                            Ok(format!(
                                "[DRY RUN] 工具 {} 会被调用，参数: {}",
                                tc.name, tc.arguments
                            ))
                        } else if !self.confirm_tool(t, &tc.arguments).await? {
                            denied = true;
                            Ok(Self::denied_tool_result(&tc.name))
                        } else {
                            t.execute(&self.client, &tc.arguments).await
                        };
                        match exec_result {
                            Ok(out) => {
                                messages.push(ChatMessage::tool(&tc.id, &tc.name, out.clone()));
                                if denied {
                                    tracing::info!(loop = i, tool = %tc.name, "工具调用被用户拒绝");
                                    format!("[DENY] {} -> {}", tc.name, truncate_chars(&out, 100))
                                } else {
                                    tracing::info!(loop = i, tool = %tc.name, "工具调用成功");
                                    format!("[OK] {} -> {}", tc.name, truncate_chars(&out, 100))
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("调用失败: {}", e);
                                tracing::warn!(loop = i, tool = %tc.name, error = %e, "工具调用失败");
                                messages.push(ChatMessage::tool(&tc.id, &tc.name, err_msg.clone()));
                                format!("[ERR] {} -> {}", tc.name, err_msg)
                            }
                        }
                    }
                    None => {
                        let err_msg = format!("工具 {} 不存在", tc.name);
                        tracing::warn!(loop = i, tool = %tc.name, "工具不存在");
                        messages.push(ChatMessage::tool(&tc.id, &tc.name, &err_msg));
                        format!("[ERR] {}", err_msg)
                    }
                };
                result.logs.push(AgentLog {
                    loop_index: i,
                    message: tool_log,
                });
            }

            // 短暂间隔，避免过快
            sleep(Duration::from_millis(200)).await;
        }

        result.final_output = "达到最大循环次数".into();
        tracing::warn!(max_loops = max_loops, "Agent 达到最大循环次数");
        Err(Error::MaxLoopExceeded(max_loops))
    }

    async fn confirm_tool(&self, tool: &dyn Tool, arguments_json: &str) -> Result<bool> {
        if !self.config.confirm_dangerous_tools || !tool.requires_confirmation() {
            return Ok(true);
        }

        let confirmation = tool.confirmation(arguments_json);
        match self.confirmation_handler.confirm(&confirmation).await? {
            ToolConfirmationDecision::Allow => Ok(true),
            ToolConfirmationDecision::Deny => Ok(false),
        }
    }

    fn denied_tool_result(tool_name: &str) -> String {
        json!({
            "ok": false,
            "cancelled": true,
            "message": format!("用户拒绝执行危险操作 {}", tool_name),
        })
        .to_string()
    }

    /// 获取当前配额剩余（当 `quota_enforced=false` 时返回 `None`）
    pub fn remaining_quota(&self) -> Option<u32> {
        if self.config.quota_enforced {
            Some(self.counters.remaining(self.config.max_per_day))
        } else {
            None
        }
    }
}
