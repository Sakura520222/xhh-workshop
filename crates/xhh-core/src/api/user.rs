//! 用户相关 API
//!
//! - 用户主页详情
//! - 关注列表 / 粉丝列表
//! - 关注 / 取关 / 拉黑（用户关系操作）

use std::collections::BTreeMap;

use serde_json::Value;

use crate::client::XhhClient;
use crate::error::Result;

const PATH_PROFILE: &str = "/bbs/app/profile/user/profile";
const PATH_FOLLOWING: &str = "/bbs/app/profile/following/list";
const PATH_FOLLOWER: &str = "/bbs/app/profile/follower/list";
const PATH_FOLLOW: &str = "/bbs/app/profile/follow/user";
const PATH_UNFOLLOW: &str = "/bbs/app/profile/follow/user/cancel";
const PATH_RELATION: &str = "/bbs/app/profile/build/relation/";
const PATH_USER_INFO: &str = "/bbs/app/api/user/info";
const PATH_USER_LINK_LIST: &str = "/bbs/app/profile/user/link/list";

/// 用户主页详情
pub async fn user_profile(client: &XhhClient, userid: Option<&str>) -> Result<Value> {
    tracing::debug!(userid = userid.unwrap_or("self"), "获取用户主页");
    let userid = userid.unwrap_or(&client.heybox_id);
    client.get(PATH_PROFILE, &[("userid", userid)]).await
}

/// 关注列表
pub async fn following_list(client: &XhhClient, userid: &str) -> Result<Value> {
    tracing::debug!(userid = %userid, "获取关注列表");
    client.get(PATH_FOLLOWING, &[("userid", userid)]).await
}

/// 粉丝列表
pub async fn follower_list(
    client: &XhhClient,
    userid: &str,
    offset: u32,
    limit: u32,
) -> Result<Value> {
    tracing::debug!(userid = %userid, offset = offset, limit = limit, "获取粉丝列表");
    client
        .get(
            PATH_FOLLOWER,
            &[
                ("userid", userid),
                ("offset", &offset.to_string()),
                ("limit", &limit.to_string()),
            ],
        )
        .await
}

/// 关注用户
pub async fn follow_user(client: &XhhClient, userid: &str) -> Result<Value> {
    tracing::info!(userid = %userid, "关注用户");
    let mut body = BTreeMap::new();
    body.insert("userid".into(), userid.into());
    client.post(PATH_FOLLOW, &body, 0).await
}

/// 取关用户
pub async fn unfollow_user(client: &XhhClient, userid: &str) -> Result<Value> {
    tracing::info!(userid = %userid, "取关用户");
    let mut body = BTreeMap::new();
    body.insert("userid".into(), userid.into());
    client.post(PATH_UNFOLLOW, &body, 0).await
}

/// 用户关系操作（拉黑：relation_type=-1）
pub async fn build_relation(client: &XhhClient, userid: &str, relation_type: i64) -> Result<Value> {
    tracing::info!(userid = %userid, relation_type = relation_type, "用户关系操作");
    let mut body = BTreeMap::new();
    body.insert("userid".into(), userid.into());
    body.insert("relation_type".into(), relation_type.to_string());
    client.post(PATH_RELATION, &body, 0).await
}

/// 当前登录用户信息
pub async fn user_info(client: &XhhClient) -> Result<Value> {
    tracing::debug!("获取当前用户信息");
    client.get(PATH_USER_INFO, &[]).await
}

/// 用户帖子列表（§2.10），`userid` 为 None 时默认当前用户
pub async fn user_link_list(
    client: &XhhClient,
    userid: Option<&str>,
    offset: u32,
    limit: u32,
) -> Result<Value> {
    tracing::debug!(
        userid = userid.unwrap_or("self"),
        offset = offset,
        limit = limit,
        "获取用户帖子列表"
    );
    let userid = userid.unwrap_or(&client.heybox_id);
    client
        .get(
            PATH_USER_LINK_LIST,
            &[
                ("userid", userid),
                ("offset", &offset.to_string()),
                ("limit", &limit.to_string()),
            ],
        )
        .await
}
