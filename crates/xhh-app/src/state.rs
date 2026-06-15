//! 应用全局状态

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use xhh_agent::provider::ChatMessage;
use xhh_agent::runner::AgentRunner;
use xhh_core::client::XhhClient;
use xhh_core::config::Config;

/// 危险工具确认后待恢复的工具调用快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingResume {
    pub tool_calls: Vec<xhh_agent::provider::ToolCall>,
    pub loops_used: u32,
    pub completed_tool_calls: Vec<String>,
}

/// 单个 Agent 会话的持久化数据（不含 runner，runner 由全局懒加载并复用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub ui_messages: Vec<crate::commands::AgentUiMsg>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub pending_resume: Option<PendingResume>,
}

impl SessionData {
    pub fn new(id: String) -> Self {
        let now = now_ts();
        Self {
            id,
            title: "新会话".to_string(),
            messages: vec![ChatMessage::system(xhh_agent::prompt::SYSTEM_PROMPT)],
            ui_messages: Vec::new(),
            created_at: now,
            updated_at: now,
            pending_resume: None,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now_ts();
    }
}

/// Agent 多会话容器（runner 单独存于 AppState.agent_runner）
pub struct AgentSessions {
    /// 当前活跃会话 ID
    pub active_id: String,
    /// session_id → SessionData
    pub sessions: HashMap<String, SessionData>,
}

impl AgentSessions {
    pub fn new() -> Self {
        Self {
            active_id: String::new(),
            sessions: HashMap::new(),
        }
    }

    pub fn active(&self) -> Option<&SessionData> {
        if self.active_id.is_empty() {
            None
        } else {
            self.sessions.get(&self.active_id)
        }
    }

    pub fn active_mut(&mut self) -> Option<&mut SessionData> {
        if self.active_id.is_empty() {
            None
        } else {
            self.sessions.get_mut(&self.active_id)
        }
    }

    pub fn switch(&mut self, id: &str) -> Result<(), String> {
        if !self.sessions.contains_key(id) {
            return Err(format!("会话不存在: {}", id));
        }
        self.active_id = id.to_string();
        Ok(())
    }

    pub fn insert(&mut self, session: SessionData) {
        self.active_id = session.id.clone();
        self.sessions.insert(session.id.clone(), session);
    }

    /// 删除指定会话。若删的是当前活跃会话，自动切到剩余中 updated_at 最大的；
    /// 全删完后 active_id 变为空字符串，由调用方决定是否补一个新会话。
    pub fn remove(&mut self, id: &str) -> Option<SessionData> {
        let removed = self.sessions.remove(id)?;
        if self.active_id == id {
            self.active_id = self
                .sessions
                .values()
                .max_by_key(|s| s.updated_at)
                .map(|s| s.id.clone())
                .unwrap_or_default();
        }
        Some(removed)
    }

    pub fn list_meta(&self) -> Vec<SessionMeta> {
        let mut items: Vec<_> = self
            .sessions
            .values()
            .map(|s| SessionMeta {
                id: s.id.clone(),
                title: s.title.clone(),
                created_at: s.created_at,
                updated_at: s.updated_at,
                message_count: s.ui_messages.len() as u32,
            })
            .collect();
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        items
    }
}

impl Default for AgentSessions {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话列表项（轻量元数据，供前端渲染侧栏）
#[derive(Debug, Clone, Serialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: u32,
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<Option<XhhClient>>>,
    /// 全局共享 runner，所有会话复用同一份 AgentConfig
    pub agent_runner: Arc<Mutex<Option<AgentRunner>>>,
    pub agent_sessions: Arc<Mutex<AgentSessions>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            agent_runner: Arc::new(Mutex::new(None)),
            agent_sessions: Arc::new(Mutex::new(AgentSessions::new())),
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
        // 凭据变更后 runner 失效
        *self.agent_runner.lock().await = None;
        Ok(())
    }

    /// 登出
    pub async fn clear(&self) {
        *self.inner.write().await = None;
        *self.agent_runner.lock().await = None;
        let mut sessions = self.agent_sessions.lock().await;
        sessions.sessions.clear();
        sessions.active_id.clear();
    }
}
