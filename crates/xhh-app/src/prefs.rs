//! 窗口偏好持久化

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 窗口背景效果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WindowEffect {
    /// 无效果
    None,
    /// 云母（Win11 22H2+）
    #[default]
    Mica,
    /// 亚克力
    Acrylic,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PrefsFile {
    #[serde(default)]
    window_effect: WindowEffect,
}

fn prefs_path() -> PathBuf {
    let dirs = directories::BaseDirs::new().expect("无法解析用户目录");
    dirs.config_dir().join("xhh").join("window_prefs.json")
}

pub fn load_effect() -> WindowEffect {
    match std::fs::read(prefs_path()) {
        Ok(bytes) => serde_json::from_slice::<PrefsFile>(&bytes)
            .map(|p| p.window_effect)
            .unwrap_or_default(),
        Err(_) => WindowEffect::default(),
    }
}

pub fn save_effect(effect: WindowEffect) -> Result<(), String> {
    let path = prefs_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = PrefsFile { window_effect: effect };
    let json = serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
