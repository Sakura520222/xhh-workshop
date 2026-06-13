//! 配置文件读写
//!
//! 配置文件路径：
//! - Windows: `%APPDATA%/xhh/config.json`
//! - macOS:   `~/Library/Application Support/xhh/config.json`
//! - Linux:   `~/.config/xhh/config.json`

use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// 配置文件在磁盘上的全部字段
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct Config {
    /// 用户 ID（数字字符串）
    #[serde(default)]
    pub heybox_id: String,
    /// 核心登录凭据（user_pkey 的值）
    #[serde(default)]
    pub pkey: String,
    /// 设备指纹
    #[serde(default)]
    pub device_id: String,
    /// 昵称
    #[serde(default)]
    pub nickname: String,
    /// 登录时间（Unix 秒）
    #[serde(default)]
    pub login_time: i64,
    /// 完整 Cookie 字符串（`pkey=...; user_heybox_id=...; x_xhh_tokenid=...`）
    #[serde(default)]
    pub cookie: String,
    /// 本地生成的 Base64 Token
    #[serde(default)]
    pub x_xhh_tokenid: String,
}

impl Config {
    /// 返回应用程序配置目录（按平台规范）
    ///
    /// - Windows: `%APPDATA%/xhh/`
    /// - macOS:   `~/Library/Application Support/xhh/`
    /// - Linux:   `~/.config/xhh/`
    pub fn config_dir() -> PathBuf {
        // 不使用 ProjectDirs（Windows 上默认带冗余的 org/app/config 三层），
        // 直接用 BaseDirs.config_dir() + "xhh"。
        let dirs = directories::BaseDirs::new().expect("无法解析用户目录");
        dirs.config_dir().join("xhh")
    }

    /// 返回默认配置文件路径
    pub fn default_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// 从指定路径读取配置，路径为 None 时用 [`Config::default_path`]
    ///
    /// 文件不存在时返回空 [`Config::default()`]，不视为错误。
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(Self::default_path);
        if !path.exists() {
            tracing::debug!(?path, "配置文件不存在，使用默认值");
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            tracing::debug!(?path, "配置文件为空，使用默认值");
            return Ok(Self::default());
        }
        let cfg: Config = serde_json::from_str(&content)?;
        tracing::debug!(?path, heybox_id = %cfg.heybox_id, nickname = %cfg.nickname, "配置已加载");
        Ok(cfg)
    }

    /// 把配置写入指定路径，路径为 None 时用 [`Config::default_path`]
    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(Self::default_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        tracing::debug!(?path, "配置已保存");
        Ok(())
    }

    /// 是否已具备完整登录凭据
    pub fn has_credentials(&self) -> bool {
        !self.pkey.is_empty() && !self.heybox_id.is_empty()
    }

    /// 把 login_time 转为人类可读字符串（系统本地时区，YYYY-MM-DD HH:MM:SS）
    pub fn login_time_display(&self) -> String {
        if self.login_time <= 0 {
            return "未知".to_string();
        }
        match Local.timestamp_opt(self.login_time, 0) {
            chrono::LocalResult::Single(t) => t.format("%Y-%m-%d %H:%M:%S").to_string(),
            _ => "无效".to_string(),
        }
    }

    /// 重新生成 cookie 字符串（需要先填好 pkey/heybox_id/x_xhh_tokenid）
    ///
    /// 注意：`user_pkey` 是带 `user_` 前缀的。
    /// 实测必须用 `user_pkey=...` 才能通过服务端校验。
    pub fn build_cookie(&self) -> String {
        format!(
            "user_pkey={}; user_heybox_id={}; x_xhh_tokenid={}",
            self.pkey, self.heybox_id, self.x_xhh_tokenid
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cookie_format() {
        let cfg = Config {
            pkey: "ABC".into(),
            heybox_id: "12345".into(),
            x_xhh_tokenid: "TOKEN".into(),
            ..Config::default()
        };
        assert_eq!(
            cfg.build_cookie(),
            "user_pkey=ABC; user_heybox_id=12345; x_xhh_tokenid=TOKEN"
        );
    }

    #[test]
    fn has_credentials_check() {
        let empty = Config::default();
        assert!(!empty.has_credentials());

        let ok = Config {
            pkey: "k".into(),
            heybox_id: "1".into(),
            ..Config::default()
        };
        assert!(ok.has_credentials());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("xhh_config_test_roundtrip.json");
        let _ = std::fs::remove_file(&tmp);

        let cfg = Config {
            pkey: "P".into(),
            heybox_id: "9".into(),
            nickname: "tester".into(),
            ..Config::default()
        };
        cfg.save(Some(&tmp)).unwrap();
        let loaded = Config::load(Some(&tmp)).unwrap();
        assert_eq!(loaded.pkey, "P");
        assert_eq!(loaded.heybox_id, "9");
        assert_eq!(loaded.nickname, "tester");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = std::path::Path::new("/definitely/does/not/exist.json");
        let cfg = Config::load(Some(path)).unwrap();
        assert_eq!(cfg.pkey, "");
        assert_eq!(cfg.heybox_id, "");
    }
}
