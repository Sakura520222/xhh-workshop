//! 应用全局共享状态
//!
//! 持有 [`XhhClient`]（含登录凭据）与可选的 Agent Runner 句柄。
//! 通过 `axum::extract::State` 注入到所有 handler。

use std::sync::Arc;

use tokio::sync::RwLock;

use xhh_core::client::XhhClient;
use xhh_core::config::Config;

use crate::error::{ApiError, ApiResult};

/// 可共享的应用状态
#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<AppStateInner>>,
    /// 可选 Bearer Token，启用后所有 `/api/*` 请求需带 `Authorization: Bearer <token>`
    pub bearer_token: Option<String>,
}

#[derive(Debug)]
struct AppStateInner {
    /// 当前活动的 XhhClient（已登录则持有 cookie）
    client: Option<XhhClient>,
    /// 当前配置（用于刷新 client / 反查凭据）
    config: Config,
}

impl AppState {
    /// 创建空状态（启动时未登录）
    pub fn new(bearer_token: Option<String>) -> Self {
        let config = Config::load(None).unwrap_or_default();
        let client = if config.has_credentials() {
            XhhClient::new(config.clone()).ok()
        } else {
            None
        };
        Self {
            inner: Arc::new(RwLock::new(AppStateInner { client, config })),
            bearer_token,
        }
    }

    /// 校验 Bearer Token（启用时）
    pub fn check_bearer(&self, header_value: Option<&str>) -> ApiResult<()> {
        if let Some(expected) = &self.bearer_token {
            let got = header_value
                .and_then(|h| h.strip_prefix("Bearer "))
                .unwrap_or("");
            if got.is_empty() || got != expected.as_str() {
                return Err(ApiError::NotLoggedIn);
            }
        }
        Ok(())
    }

    /// 读取当前的 XhhClient（要求已登录）
    pub async fn require_client(&self) -> ApiResult<XhhClient> {
        let guard = self.inner.read().await;
        guard.client.clone().ok_or_else(|| ApiError::NotLoggedIn)
    }

    /// 仅当前存在 XhhClient 时返回 Some
    pub async fn try_client(&self) -> Option<XhhClient> {
        self.inner.read().await.client.clone()
    }

    /// 重新加载配置 + 重建 client（用于扫码登录成功后）
    pub async fn refresh(&self) -> ApiResult<Config> {
        let config = Config::load(None).map_err(|e| ApiError::Config(e.to_string()))?;
        let client = if config.has_credentials() {
            XhhClient::new(config.clone()).map_err(|e| ApiError::Config(e.to_string()))?
        } else {
            return Err(ApiError::NotLoggedIn);
        };
        let mut guard = self.inner.write().await;
        guard.client = Some(client);
        guard.config = config.clone();
        Ok(config)
    }

    /// 清空登录态（登出）
    pub async fn clear(&self) {
        let mut guard = self.inner.write().await;
        guard.client = None;
        guard.config = Config::default();
    }

    /// 当前登录信息（仅返回脱敏字段）
    pub async fn snapshot(&self) -> Option<ConfigSnapshot> {
        let guard = self.inner.read().await;
        guard.client.as_ref().map(|c| ConfigSnapshot {
            heybox_id: c.heybox_id.clone(),
            nickname: c.config.nickname.clone(),
            device_id: c.device_id.clone(),
            login_time: c.config.login_time,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ConfigSnapshot {
    pub heybox_id: String,
    pub nickname: String,
    pub device_id: String,
    pub login_time: i64,
}
