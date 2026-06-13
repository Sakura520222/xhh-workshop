//! 应用全局状态

use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};
use xhh_agent::provider::{ChatMessage, ToolCall};
use xhh_agent::runner::AgentRunner;
use xhh_core::client::XhhClient;
use xhh_core::config::Config;

/// 危险工具确认后待恢复的工具调用快照
pub struct PendingResume {
    pub tool_calls: Vec<ToolCall>,
    pub loops_used: u32,
    pub completed_tool_calls: Vec<String>,
}

/// Agent 持久会话：复用同一 runner 与消息历史，支撑多轮对话
pub struct AgentSession {
    pub runner: AgentRunner,
    pub messages: Vec<ChatMessage>,
    /// 非空时表示上一次调用因危险操作中断，下次调用应直接恢复执行
    pub pending_resume: Option<PendingResume>,
}

impl AgentSession {
    /// 创建新会话，预置 system prompt 作为上下文起点
    pub fn new(runner: AgentRunner) -> Self {
        Self {
            runner,
            messages: vec![ChatMessage::system(xhh_agent::prompt::SYSTEM_PROMPT)],
            pending_resume: None,
        }
    }
}

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<Option<XhhClient>>>,
    pub agent: Arc<Mutex<Option<AgentSession>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            agent: Arc::new(Mutex::new(None)),
        }
    }
}

impl AppState {
    /// 启动时尝试从配置文件加载已登录的客户端
    pub fn try_load() -> Self {
        let state = Self::default();
        if let Ok(cfg) = Config::load(None) {
            if cfg.has_credentials() {
                if let Ok(client) = XhhClient::new(cfg) {
                    let inner = state.inner.clone();
                    // 同步写入（启动阶段无并发）
                    *inner.blocking_write() = Some(client);
                }
            }
        }
        state
    }

    /// 获取当前客户端（要求已登录）
    pub async fn require_client(&self) -> Result<XhhClient, String> {
        self.inner
            .read()
            .await
            .clone()
            .ok_or_else(|| "未登录".to_string())
    }

    /// 重新加载配置（扫码登录成功后调用）
    pub async fn refresh(&self) -> Result<(), String> {
        let cfg = Config::load(None).map_err(|e| e.to_string())?;
        if !cfg.has_credentials() {
            return Err("凭据为空".to_string());
        }
        let client = XhhClient::new(cfg).map_err(|e| e.to_string())?;
        *self.inner.write().await = Some(client);
        // 凭据变更后旧会话失效，重置 Agent
        *self.agent.lock().await = None;
        Ok(())
    }

    /// 登出
    pub async fn clear(&self) {
        *self.inner.write().await = None;
        *self.agent.lock().await = None;
    }
}
