//! 路由集合
//!
//! 按 API 分组：auth / feed / post / comment / interaction / user / search / misc / agent。

pub mod agent;
pub mod auth;
pub mod comment;
pub mod feed;
pub mod interaction;
pub mod misc;
pub mod post;
pub mod search;
pub mod user;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// 挂载所有 `/api/*` 子路由
pub fn api_routes(state: AppState) -> Router {
    Router::new()
        // auth
        .route("/api/auth/qrcode", get(auth::get_qrcode))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/status", get(auth::status))
        .route("/api/auth/logout", post(auth::logout))
        // feed
        .route("/api/feeds", get(feed::list_feeds))
        .route("/api/post/detail/{link_id}", get(feed::post_detail))
        .route("/api/user/events", get(feed::user_events))
        // post
        .route("/api/post/create", post(post::create))
        .route("/api/post/edit", post(post::edit))
        .route("/api/post/delete", post(post::delete))
        // comment
        .route("/api/comment/list", get(comment::list))
        .route("/api/comment/sub", get(comment::sub_list))
        .route("/api/comment/create", post(comment::create))
        .route("/api/comment/delete", post(comment::delete))
        // interaction
        .route("/api/like/post", post(interaction::like_post))
        .route("/api/like/comment", post(interaction::like_comment))
        .route("/api/favour/toggle", post(interaction::favour))
        .route("/api/favour/folders", get(interaction::folders))
        .route("/api/favour/folder", post(interaction::create_folder))
        // user
        .route("/api/user/profile", get(user::profile))
        .route("/api/user/following", get(user::following))
        .route("/api/user/follower", get(user::follower))
        .route("/api/user/follow", post(user::follow))
        .route("/api/user/unfollow", post(user::unfollow))
        // search
        .route("/api/search", get(search::search))
        .route("/api/search/topic", get(search::topic))
        .route("/api/search/community", get(search::community))
        .route("/api/search/discovery", get(search::discovery))
        // misc
        .route("/api/emoji", get(misc::emoji))
        .route("/api/notifications", get(misc::notifications))
        .route("/api/notifications/unread", get(misc::notification_unread))
        // agent
        .route("/api/agent/chat", post(agent::chat))
        .route("/api/agent/auto-post", post(agent::auto_post))
        .with_state(state)
}
