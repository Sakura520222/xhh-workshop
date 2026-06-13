//! API 子模块入口
//!
//! 按业务功能拆分：post / feed / comment / interaction / user / search / notification / emoji / upload
//! M1.7-M1.9 阶段逐步填充。

pub mod comment;
pub mod emoji;
pub mod feed;
pub mod interaction;
pub mod notification;
pub mod post;
pub mod search;
pub mod upload;
pub mod user;
