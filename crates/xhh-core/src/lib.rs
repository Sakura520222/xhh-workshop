//! xhh-core: 小黑盒 Web API 核心库
//!
//! 提供 hkey 签名算法、扫码登录、发帖/评论/点赞/收藏/COS 上传等
//! 全部接口的纯 Rust 实现。无 UI 依赖，可被 CLI、HTTP 服务、
//! Tauri 应用、第三方 crate 复用。

#![forbid(unsafe_code)]
#![allow(missing_docs)]
#![allow(clippy::module_inception)]

pub mod auth;
pub mod client;
pub mod config;
pub mod crypto;
pub mod error;
pub mod hkey;

pub mod api;
pub mod models;

pub use error::{Error, Result};

/// 库版本（与 Cargo.toml 中 package.version 同步）
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// API 基础地址
pub const BASE_URL: &str = "https://api.xiaoheihe.cn";

/// Web 端 UA 标识
pub const WEB_VERSION: &str = "2.5";
pub const APP_VERSION: &str = "999.0.4";
