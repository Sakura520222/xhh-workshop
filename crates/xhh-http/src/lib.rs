//! xhh-http: 小黑盒本地 HTTP REST 服务
//!
//! 基于 axum 实现，复用 xhh-core 的所有 API 能力。
//! 启动后默认监听 `127.0.0.1:9876`，提供：
//!
//! - `/api/auth/*` — 扫码登录、登录态检查
//! - `/api/post/*` — 发帖 / 编辑 / 删帖 / 详情
//! - `/api/feed/*` — 帖子列表 / 用户动态
//! - `/api/comment/*` — 评论 CRUD
//! - `/api/interaction/*` — 点赞 / 收藏
//! - `/api/user/*` — 用户主页 / 关注 / 粉丝
//! - `/api/search/*` — 通用 / 话题 / 社区 / 热搜
//! - `/api/agent/*` — Agent 调用（chat / auto-post）
//! - `/api/emoji` / `/api/notifications` — 辅助
//! - `/docs` — Swagger UI（utoipa 自动生成 OpenAPI）

#![forbid(unsafe_code)]

pub mod app;
pub mod auth_mw;
pub mod error;
pub mod routes;
pub mod state;

pub use app::{build_app, parse_addr};
pub use error::ApiError;
pub use state::AppState;
