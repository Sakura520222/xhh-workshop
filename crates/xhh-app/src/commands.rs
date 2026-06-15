//! Tauri IPC 命令
//!
//! 所有命令统一返回 `Result<T, String>`，Tauri 自动序列化。

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use xhh_core::api::{
    comment as api_comment, emoji as api_emoji, feed as api_feed, interaction as api_inter,
    post as api_post, search as api_search, user as api_user,
};
use xhh_core::auth::{get_qr_code, poll_qr_state_once, QrCodeResp, QrPollResult};
use xhh_core::client::XhhClient;

use crate::state::{AgentSessions, AppState, SessionData, SessionMeta};

// ─── Auth ────────────────────────────────────────────────

/// 获取登录二维码
#[tauri::command]
pub async fn auth_get_qr_code() -> Result<QrCodeResp, String> {
    tracing::info!("获取登录二维码");
    let anon = XhhClient::anonymous(None).map_err(|e| e.to_string())?;
    get_qr_code(&anon).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct LoginResult {
    pub ok: bool,
    pub nickname: String,
    pub heybox_id: String,
    pub avatar: String,
    pub message: String,
}

/// 扫码登录（阻塞轮询，最长 300s）
#[tauri::command]
pub async fn auth_login(
    state: State<'_, AppState>,
    raw_query: String,
    device_id: String,
) -> Result<LoginResult, String> {
    tracing::info!(device_id = %device_id, "开始扫码登录轮询");
    let anon = XhhClient::anonymous(Some(device_id.clone())).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        if Instant::now() > deadline {
            return Ok(LoginResult {
                ok: false,
                nickname: String::new(),
                heybox_id: String::new(),
                avatar: String::new(),
                message: "扫码超时".into(),
            });
        }
        match poll_qr_state_once(&anon, &raw_query, &device_id).await {
            Ok(QrPollResult::Waiting { .. }) | Ok(QrPollResult::Scanned) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(QrPollResult::Success(mut s)) => {
                s.config.device_id = device_id.clone();
                s.config.save(None).map_err(|e| e.to_string())?;
                state.refresh().await?;
                let avatar = fetch_avatar(&state).await;
                tracing::info!(nickname = %s.nickname, heybox_id = %s.heybox_id, "扫码登录成功");
                return Ok(LoginResult {
                    ok: true,
                    nickname: s.nickname,
                    heybox_id: s.heybox_id,
                    avatar,
                    message: "登录成功".into(),
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "扫码登录失败");
                return Ok(LoginResult {
                    ok: false,
                    nickname: String::new(),
                    heybox_id: String::new(),
                    avatar: String::new(),
                    message: e.to_string(),
                });
            }
        }
    }
}

/// 检查当前登录态
#[tauri::command]
pub async fn auth_status(state: State<'_, AppState>) -> Result<LoginResult, String> {
    let cfg = xhh_core::config::Config::load(None).map_err(|e| e.to_string())?;
    if !cfg.has_credentials() {
        return Ok(LoginResult {
            ok: false,
            nickname: String::new(),
            heybox_id: String::new(),
            avatar: String::new(),
            message: String::new(),
        });
    }
    let avatar = fetch_avatar(&state).await;
    Ok(LoginResult {
        ok: true,
        nickname: cfg.nickname,
        heybox_id: cfg.heybox_id,
        avatar,
        message: String::new(),
    })
}

/// 登出
#[tauri::command]
pub async fn auth_logout(state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("用户登出");
    let empty = xhh_core::config::Config::default();
    empty.save(None).map_err(|e| e.to_string())?;
    state.clear().await;
    Ok(())
}

/// 拉取当前登录用户的头像 URL（失败返回空串，调用方回退到首字母占位）
async fn fetch_avatar(state: &AppState) -> String {
    let Ok(c) = state.require_client().await else {
        return String::new();
    };
    api_user::user_profile(&c, None)
        .await
        .ok()
        .and_then(|v| {
            v["result"]["account_detail"]["avatar"]
                .as_str()
                .map(String::from)
        })
        .unwrap_or_default()
}

// ─── Feeds ───────────────────────────────────────────────

/// 拉取帖子列表
#[tauri::command]
pub async fn feeds_list(
    state: State<'_, AppState>,
    page: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_feed::feeds(
        &c,
        api_feed::FeedsQuery {
            page: Some(page.unwrap_or(1)),
            limit: Some(limit.unwrap_or(20)),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| e.to_string())
}

/// 帖子详情（支持楼层分页）
#[tauri::command]
pub async fn post_detail(
    state: State<'_, AppState>,
    link_id: String,
    page: Option<u32>,
    index: Option<u32>,
    limit: Option<u32>,
    is_first: Option<u32>,
    owner_only: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_feed::post_detail(
        &c,
        &link_id,
        api_feed::PostDetailQuery {
            page,
            index,
            limit,
            is_first,
            owner_only,
        },
    )
    .await
    .map_err(|e| e.to_string())
}

/// 社区帖子列表
#[tauri::command]
pub async fn community_feeds(
    state: State<'_, AppState>,
    topic_id: u32,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_feed::community_feeds(
        &c,
        topic_id,
        api_feed::CommunityFeedsQuery {
            limit: Some(limit.unwrap_or(20)),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| e.to_string())
}

// ─── Post ────────────────────────────────────────────────

/// 发帖
#[tauri::command]
pub async fn post_create(
    state: State<'_, AppState>,
    title: String,
    content: String,
    hashtags: Vec<String>,
    community_topic_id: Option<String>,
    images: Option<Vec<serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    let (topic_ids, link_tag) = match community_topic_id {
        Some(t) if !t.is_empty() => (vec![t], 27i64),
        _ => (vec!["58144".into()], 28i64),
    };
    let content_input = match images {
        Some(ref imgs) if !imgs.is_empty() => {
            tracing::debug!(images_count = imgs.len(), "发帖带图片");
            let mut blocks = vec![api_post::ContentBlock::Text { text: content }];
            for img in imgs {
                let url = img.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let width = img
                    .get("width")
                    .and_then(|v| v.as_u64())
                    .map(|w| w.to_string());
                let height = img
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .map(|h| h.to_string());
                blocks.push(api_post::ContentBlock::Img {
                    url: url.into(),
                    text: Some(url.into()),
                    width,
                    height,
                });
            }
            api_post::ContentInput::Blocks(blocks)
        }
        _ => {
            tracing::debug!("发帖纯文本");
            api_post::ContentInput::Plain(content)
        }
    };
    let req = api_post::CreatePostReq {
        title,
        content: content_input,
        topic_ids,
        hashtags,
        link_tag,
        ..Default::default()
    };
    let resp = api_post::create_post(&c, req)
        .await
        .map_err(|e| e.to_string())?;
    tracing::debug!(response = %serde_json::to_string(&resp).unwrap_or_default(), "发帖 API 响应");
    Ok(resp)
}

/// 删帖
#[tauri::command]
pub async fn post_delete(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_post::delete_post(&c, &link_id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Comment ─────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CommentReq {
    pub link_id: String,
    pub text: String,
    pub reply_id: Option<String>,
    pub root_id: Option<String>,
}

/// 发评论
#[tauri::command]
pub async fn comment_create(
    state: State<'_, AppState>,
    req: CommentReq,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    let cr = match req.reply_id {
        Some(rid) if !rid.is_empty() => {
            let root = req.root_id.unwrap_or_else(|| rid.clone());
            api_comment::CreateCommentReq::reply(&req.link_id, req.text, &rid, &root)
        }
        _ => api_comment::CreateCommentReq::top_level(&req.link_id, req.text),
    };
    api_comment::create_comment(&c, cr)
        .await
        .map_err(|e| e.to_string())
}

/// 评论列表（已废弃，建议使用 post_detail 获取评论）
#[allow(deprecated)]
#[tauri::command]
pub async fn comment_list(
    state: State<'_, AppState>,
    link_id: String,
    page: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_comment::list_comments(&c, &link_id, page.unwrap_or(1), limit.unwrap_or(20))
        .await
        .map_err(|e| e.to_string())
}

/// 子评论列表
#[tauri::command]
pub async fn sub_comments(
    state: State<'_, AppState>,
    root_comment_id: String,
    lastval: Option<String>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_comment::list_sub_comments(&c, &root_comment_id, lastval.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ─── Interaction ─────────────────────────────────────────

/// 帖子点赞 / 取消（award_type: 1=点赞, 0=取消，默认 1）
#[tauri::command]
pub async fn like_post(
    state: State<'_, AppState>,
    link_id: String,
    award_type: Option<i64>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_inter::like_post(&c, &link_id, award_type.unwrap_or(1))
        .await
        .map_err(|e| e.to_string())
}

/// 评论点赞（toggle）
#[tauri::command]
pub async fn like_comment(
    state: State<'_, AppState>,
    comment_id: String,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_inter::toggle_like_comment(&c, &comment_id)
        .await
        .map_err(|e| e.to_string())
}

/// 收藏（toggle）
#[tauri::command]
pub async fn favourite(
    state: State<'_, AppState>,
    link_id: String,
    folder_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_inter::toggle_favourite(&c, &link_id, folder_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ─── Search / User ───────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct SearchReq {
    pub q: String,
    pub search_type: Option<String>,
    pub topic_id: Option<u32>,
    pub limit: Option<u32>,
}

/// 通用搜索
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    req: SearchReq,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    let st = match req.search_type.as_deref() {
        Some("内容") | Some("content") => api_search::SearchType::Content,
        Some("用户") | Some("user") => api_search::SearchType::User,
        Some("游戏") | Some("game") => api_search::SearchType::Game,
        Some("话题") | Some("topic") => api_search::SearchType::Topic,
        Some("商品") | Some("product") => api_search::SearchType::Product,
        _ => api_search::SearchType::Comprehensive,
    };
    tracing::debug!(search_type = ?req.search_type, q = %req.q, "搜索请求");
    let result = api_search::search(
        &c,
        api_search::SearchReq {
            q: req.q,
            search_type: st,
            offset: 0,
            limit: req.limit.unwrap_or(20),
            topic_id: req.topic_id,
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    tracing::debug!(items = ?result.pointer("/result/items").and_then(|v| v.as_array()).map_or(0, |a| a.len()), "搜索结果数");
    Ok(result)
}

/// 社区搜索
#[tauri::command]
pub async fn search_community(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_search::search_community(&c, &keyword)
        .await
        .map_err(|e| e.to_string())
}

/// 用户主页
#[tauri::command]
pub async fn user_profile(
    state: State<'_, AppState>,
    userid: Option<String>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_user::user_profile(&c, userid.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ─── Agent ───────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AgentToolConfirmationDecision {
    pub tool_name: String,
    pub arguments_json: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentToolConfirmationRequest {
    pub tool_name: String,
    pub risk_level: xhh_agent::tool::RiskLevel,
    pub summary: String,
    pub arguments_json: String,
}

#[derive(Serialize)]
pub struct AgentResultDto {
    pub final_output: String,
    pub tool_calls: Vec<String>,
    pub loops_used: u32,
}

fn build_agent_runner(_state: &AppState) -> Result<xhh_agent::runner::AgentRunner, String> {
    let cfg = xhh_core::config::Config::load(None).map_err(|e| e.to_string())?;
    if !cfg.has_credentials() {
        return Err("未登录".into());
    }
    let client = XhhClient::new(cfg).map_err(|e| e.to_string())?;
    let ac = xhh_agent::config::AgentConfig::load(None).map_err(|e| e.to_string())?;
    xhh_agent::runner::AgentRunner::from_config(ac, client).map_err(|e| e.to_string())
}

/// Agent 多轮对话（复用持久会话，保留完整上下文）
#[tauri::command]
pub async fn agent_chat(
    state: State<'_, AppState>,
    message: String,
) -> Result<AgentResultDto, String> {
    ensure_agent_runtime(&state).await?;
    // 持有 runner + sessions 双锁直到完成（同步阻塞型，预期短）
    let mut runner_guard = state.agent_runner.lock().await;
    let runner = runner_guard
        .as_mut()
        .ok_or_else(|| "Agent runner 未初始化".to_string())?;
    let mut sessions = state.agent_sessions.lock().await;
    let session = sessions.active_mut().ok_or("Agent 会话未初始化")?;
    let r = runner
        .chat_with_history(&mut session.messages, &message)
        .await
        .map_err(|e| e.to_string())?;
    session.touch();
    let snapshot = snapshot_active(&sessions);
    drop(sessions);
    drop(runner_guard);
    if let Some(snap) = snapshot {
        persist_session(&snap);
    }
    Ok(AgentResultDto {
        final_output: r.final_output,
        tool_calls: r.tool_calls,
        loops_used: r.loops_used,
    })
}

/// 重置当前活跃会话（清空上下文，但保留会话条目）
#[tauri::command]
pub async fn agent_reset(state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.agent_sessions.lock().await;
    if let Some(session) = sessions.active_mut() {
        let title = session.title.clone();
        let id = session.id.clone();
        let created_at = session.created_at;
        *session = SessionData::new(id);
        session.title = title;
        session.created_at = created_at;
        session.touch();
        let snapshot = snapshot_active(&sessions);
        drop(sessions);
        if let Some(snap) = snapshot {
            persist_session(&snap);
        }
    }
    Ok(())
}

/// Agent 自动发帖
#[tauri::command]
pub async fn agent_auto_post(
    _state: State<'_, AppState>,
    intent: String,
    hashtags: Vec<String>,
) -> Result<AgentResultDto, String> {
    let mut runner = build_agent_runner(&_state)?;
    let r = runner
        .auto_post(&intent, &hashtags)
        .await
        .map_err(|e| e.to_string())?;
    Ok(AgentResultDto {
        final_output: r.final_output,
        tool_calls: r.tool_calls,
        loops_used: r.loops_used,
    })
}

/// 读取 Agent 配置
#[tauri::command]
pub async fn agent_get_config() -> Result<serde_json::Value, String> {
    let cfg = xhh_agent::config::AgentConfig::load(None).map_err(|e| e.to_string())?;
    serde_json::to_value(cfg).map_err(|e| e.to_string())
}

/// 保存 Agent 配置
#[tauri::command]
pub async fn agent_save_config(config: serde_json::Value) -> Result<(), String> {
    let cfg: xhh_agent::config::AgentConfig =
        serde_json::from_value(config).map_err(|e| format!("配置格式错误: {}", e))?;
    cfg.save(None).map_err(|e| e.to_string())
}

// ─── AI Cache ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiCacheItem {
    pub link_id: String,
    pub kind: String,
    pub content: String,
    pub updated_at: i64,
}

type AiCacheMap =
    std::collections::BTreeMap<String, std::collections::BTreeMap<String, AiCacheItem>>;

fn ai_cache_path() -> std::path::PathBuf {
    let dirs = directories::BaseDirs::new().expect("无法解析用户目录");
    dirs.config_dir().join("xhh").join("ai_cache.json")
}

fn load_ai_cache() -> Result<AiCacheMap, String> {
    let path = ai_cache_path();
    if !path.exists() {
        return Ok(AiCacheMap::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if content.trim().is_empty() {
        return Ok(AiCacheMap::new());
    }
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_ai_cache_map(map: &AiCacheMap) -> Result<(), String> {
    let path = ai_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// 读取某个帖子的 AI 生成结果缓存
#[tauri::command]
pub async fn ai_cache_get(link_id: String) -> Result<Vec<AiCacheItem>, String> {
    let map = load_ai_cache()?;
    let items = map
        .get(&link_id)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    Ok(items)
}

/// 按帖子 ID 持久化 AI 生成结果
#[tauri::command]
pub async fn ai_cache_save(
    link_id: String,
    kind: String,
    content: String,
) -> Result<AiCacheItem, String> {
    let mut map = load_ai_cache()?;
    let item = AiCacheItem {
        link_id: link_id.clone(),
        kind: kind.clone(),
        content,
        updated_at: chrono::Utc::now().timestamp(),
    };
    map.entry(link_id).or_default().insert(kind, item.clone());
    save_ai_cache_map(&map)?;
    Ok(item)
}

// ─── Agent History ───────────────────────────────────────

/// 前端 UI 消息 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUiMsg {
    pub role: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loops: Option<u32>,
}

// ─── Agent Sessions 持久化 ───────────────────────────────

fn xhh_config_root() -> std::path::PathBuf {
    let dirs = directories::BaseDirs::new().expect("无法解析用户目录");
    dirs.config_dir().join("xhh")
}

/// 旧版（v0.1.x）单文件历史，仅用于一次性迁移到多会话目录
fn legacy_agent_history_path() -> std::path::PathBuf {
    xhh_config_root().join("agent_history.json")
}

fn legacy_agent_llm_history_path() -> std::path::PathBuf {
    xhh_config_root().join("agent_llm_history.json")
}

/// 多会话目录：每会话一个 JSON 文件
fn agent_sessions_dir() -> std::path::PathBuf {
    xhh_config_root().join("agent_sessions")
}

fn agent_session_path(id: &str) -> std::path::PathBuf {
    agent_sessions_dir().join(format!("{}.json", id))
}

/// 任务模板：单文件数组
fn agent_templates_path() -> std::path::PathBuf {
    xhh_config_root().join("agent_templates.json")
}

fn load_json_file<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T, String> {
    if !path.exists() {
        return serde_json::from_str("null").map_err(|e| e.to_string());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        return serde_json::from_str("null").map_err(|e| e.to_string());
    }
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_json_file<T: serde::Serialize>(path: &std::path::Path, val: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(val).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// 生成新会话 ID（时间戳 + 短随机后缀，避免依赖 uuid crate）
fn new_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let suffix = (ts % 0x100_0000) as u32;
    format!("s_{:x}_{:06x}", (ts >> 24) as u64, suffix)
}

/// 从磁盘加载所有会话；首次启动时执行旧文件迁移
fn load_all_sessions_from_disk() -> Vec<SessionData> {
    let dir = agent_sessions_dir();
    if !dir.exists() {
        // 首次启动：尝试迁移旧 history
        if let Some(migrated) = migrate_legacy_history() {
            return migrated;
        }
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            match load_json_file::<SessionData>(&path) {
                Ok(s) => out.push(s),
                Err(e) => tracing::warn!(path = %path.display(), err = %e, "跳过损坏的会话文件"),
            }
        }
    }
    out
}

/// 把旧版单文件历史迁移成默认会话；迁移成功后删除旧文件
fn migrate_legacy_history() -> Option<Vec<SessionData>> {
    let ui_path = legacy_agent_history_path();
    let llm_path = legacy_agent_llm_history_path();
    if !ui_path.exists() && !llm_path.exists() {
        return None;
    }
    tracing::info!("检测到旧版 Agent 历史，开始迁移到多会话目录");
    let ui_msgs: Vec<AgentUiMsg> = load_json_file(&ui_path).unwrap_or_default();
    let llm_msgs: Vec<xhh_agent::provider::ChatMessage> = load_json_file(&llm_path).unwrap_or_default();
    let id = new_session_id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let title = ui_msgs
        .iter()
        .find(|m| m.role == "user")
        .map(|m| truncate_text(&m.text, 20))
        .unwrap_or_else(|| "迁移的会话".to_string());
    let session = SessionData {
        id: id.clone(),
        title,
        messages: if llm_msgs.is_empty() {
            vec![xhh_agent::provider::ChatMessage::system(xhh_agent::prompt::SYSTEM_PROMPT)]
        } else {
            llm_msgs
        },
        ui_messages: ui_msgs,
        created_at: now,
        updated_at: now,
        pending_resume: None,
    };
    let _ = save_json_file(&agent_session_path(&id), &session);
    let _ = std::fs::remove_file(&ui_path);
    let _ = std::fs::remove_file(&llm_path);
    Some(vec![session])
}

/// 持久化单个会话到磁盘
fn persist_session(session: &SessionData) {
    let path = agent_session_path(&session.id);
    if let Err(e) = save_json_file(&path, session) {
        tracing::warn!(session_id = %session.id, err = %e, "保存会话失败");
    }
}

/// 删除磁盘上的会话文件
fn delete_session_file(id: &str) {
    let path = agent_session_path(id);
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
}

/// 取活跃会话的快照（避免在锁外读路径）
fn snapshot_active(sessions: &AgentSessions) -> Option<SessionData> {
    sessions.active().cloned()
}

fn truncate_text(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

// ─── Agent Sessions：运行时懒加载 ────────────────────────

/// 确保 runner 已构建 + sessions 已加载（首次调用时执行）
async fn ensure_agent_runtime(state: &AppState) -> Result<(), String> {
    // runner 懒加载
    let mut runner_guard = state.agent_runner.lock().await;
    if runner_guard.is_none() {
        let runner = build_agent_runner(state)?;
        *runner_guard = Some(runner);
    }
    drop(runner_guard);

    // sessions 加载（仅当内存为空时）
    let mut sessions = state.agent_sessions.lock().await;
    if sessions.sessions.is_empty() {
        let loaded = load_all_sessions_from_disk();
        for s in loaded {
            sessions.sessions.insert(s.id.clone(), s);
        }
        if sessions.sessions.is_empty() {
            // 完全新用户：创建一个默认空会话
            let s = SessionData::new(new_session_id());
            sessions.insert(s);
        } else if sessions.active_id.is_empty() {
            // 选取 updated_at 最大的作为活跃
            sessions.active_id = sessions
                .sessions
                .values()
                .max_by_key(|s| s.updated_at)
                .map(|s| s.id.clone())
                .unwrap_or_default();
        }
    }
    if sessions.active_id.is_empty() {
        let s = SessionData::new(new_session_id());
        sessions.insert(s);
        let snap = sessions.active().cloned();
        drop(sessions);
        if let Some(snap) = snap {
            persist_session(&snap);
        }
    }
    Ok(())
}

// ─── Agent Sessions：Tauri 命令 ──────────────────────────

/// 列出所有会话（按 updated_at 倒序）
#[tauri::command]
pub async fn agent_session_list(
    state: State<'_, AppState>,
) -> Result<Vec<SessionMeta>, String> {
    ensure_agent_runtime(&state).await?;
    let sessions = state.agent_sessions.lock().await;
    Ok(sessions.list_meta())
}

/// 当前活跃会话 ID
#[tauri::command]
pub async fn agent_session_active(state: State<'_, AppState>) -> Result<String, String> {
    ensure_agent_runtime(&state).await?;
    let sessions = state.agent_sessions.lock().await;
    Ok(sessions.active_id.clone())
}

/// 新建空会话，返回 id（同时切到该会话）
#[tauri::command]
pub async fn agent_session_create(state: State<'_, AppState>) -> Result<String, String> {
    ensure_agent_runtime(&state).await?;
    let mut sessions = state.agent_sessions.lock().await;
    let session = SessionData::new(new_session_id());
    let id = session.id.clone();
    sessions.insert(session);
    let snap = sessions.active().cloned();
    drop(sessions);
    if let Some(snap) = snap {
        persist_session(&snap);
    }
    Ok(id)
}

/// 切换活跃会话；返回该会话的 UI 消息列表
#[tauri::command]
pub async fn agent_session_switch(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<AgentUiMsg>, String> {
    ensure_agent_runtime(&state).await?;
    let mut sessions = state.agent_sessions.lock().await;
    sessions.switch(&id)?;
    Ok(sessions.active().map(|s| s.ui_messages.clone()).unwrap_or_default())
}

/// 重命名会话
#[tauri::command]
pub async fn agent_session_rename(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<(), String> {
    let mut sessions = state.agent_sessions.lock().await;
    let session = sessions
        .sessions
        .get_mut(&id)
        .ok_or_else(|| format!("会话不存在: {}", id))?;
    session.title = title;
    session.touch();
    let snap = session.clone();
    drop(sessions);
    persist_session(&snap);
    Ok(())
}

/// 删除会话（删当前活跃会话时自动切到下一个；全删完则补一个新空会话）
#[tauri::command]
pub async fn agent_session_delete(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let mut sessions = state.agent_sessions.lock().await;
    let removed = sessions.remove(&id).ok_or_else(|| format!("会话不存在: {}", id))?;
    delete_session_file(&removed.id);
    if sessions.sessions.is_empty() {
        let s = SessionData::new(new_session_id());
        sessions.insert(s);
    }
    let snap = sessions.active().cloned();
    let active_id = sessions.active_id.clone();
    drop(sessions);
    if let Some(snap) = snap {
        persist_session(&snap);
    }
    Ok(active_id)
}

// ─── Agent History（向后兼容：操作活跃会话的 ui_messages） ───

/// 读取当前活跃会话的 UI 消息
#[tauri::command]
pub async fn agent_history_get(state: State<'_, AppState>) -> Result<Vec<AgentUiMsg>, String> {
    ensure_agent_runtime(&state).await?;
    let sessions = state.agent_sessions.lock().await;
    Ok(sessions.active().map(|s| s.ui_messages.clone()).unwrap_or_default())
}

/// 保存当前活跃会话的 UI 消息；同时根据首条用户消息更新会话标题
#[tauri::command]
pub async fn agent_history_save(
    state: State<'_, AppState>,
    messages: Vec<AgentUiMsg>,
) -> Result<(), String> {
    let mut sessions = state.agent_sessions.lock().await;
    let session = sessions
        .active_mut()
        .ok_or_else(|| "无活跃会话".to_string())?;
    let needs_rename = session.title == "新会话";
    let new_title = if needs_rename {
        messages
            .iter()
            .find(|m| m.role == "user")
            .map(|m| truncate_text(&m.text, 20))
    } else {
        None
    };
    session.ui_messages = messages;
    if let Some(t) = new_title {
        session.title = t;
    }
    session.touch();
    let snap = session.clone();
    drop(sessions);
    persist_session(&snap);
    Ok(())
}

/// 清空当前活跃会话的 UI 消息（保留会话本身；LLM 消息保留）
#[tauri::command]
pub async fn agent_history_clear(state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.agent_sessions.lock().await;
    if let Some(session) = sessions.active_mut() {
        session.ui_messages.clear();
        session.touch();
        let snap = session.clone();
        drop(sessions);
        persist_session(&snap);
    }
    Ok(())
}

// ─── Agent Templates ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub title: String,
    pub content: String,
    pub is_builtin: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

/// 内置模板种子（首次启动写入 JSON 文件，用户后续可编辑/删除）
fn builtin_templates() -> Vec<AgentTemplate> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let items: &[(&str, &str)] = &[
        (
            "分析指定板块热帖并总结互动切入点",
            "请用 search_community 找到「原神」板块，然后用 community_feeds 拉取最近热帖，最后总结 3-5 个适合我参与互动的切入点。",
        ),
        (
            "帮我在原神板块发一条 100 字内的日常帖",
            "帮我在原神板块发一条 100 字内的日常帖子，标题和内容由你拟定，语气要像一个真实玩家，不要列要点。",
        ),
        (
            "拉取我的通知，总结今天谁@了我",
            "拉取我最近的通知列表，把今天 @ 我的人和他们说的话总结成简短列表。",
        ),
        (
            "查看我最近的帖子，把零评论的列出来",
            "查看我最近发布的 10 篇帖子，把还没有评论的帖子标题和链接列出来。",
        ),
        (
            "搜索『黑神话』相关帖子，挑 3 条适合点赞的",
            "搜索关键词『黑神话』，从结果里挑 3 条内容质量较高的帖子，告诉我标题和理由，然后帮我点赞。",
        ),
    ];
    items
        .iter()
        .enumerate()
        .map(|(i, (title, content))| AgentTemplate {
            id: format!("builtin-{}", i + 1),
            title: title.to_string(),
            content: content.to_string(),
            is_builtin: true,
            created_at: now,
            updated_at: now,
        })
        .collect()
}

fn load_templates_from_disk() -> Vec<AgentTemplate> {
    let path = agent_templates_path();
    if !path.exists() {
        let seed = builtin_templates();
        let _ = save_json_file(&path, &seed);
        return seed;
    }
    load_json_file::<Vec<AgentTemplate>>(&path).unwrap_or_default()
}

fn save_templates_to_disk(templates: &[AgentTemplate]) -> Result<(), String> {
    save_json_file(&agent_templates_path(), &templates.to_vec())
}

/// 列出所有模板（按 updated_at 倒序，内置在前）
#[tauri::command]
pub async fn agent_template_list() -> Result<Vec<AgentTemplate>, String> {
    let mut templates = load_templates_from_disk();
    templates.sort_by(|a, b| {
        b.is_builtin
            .cmp(&a.is_builtin)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    Ok(templates)
}

/// 新建或更新模板：
/// - 不传 id（或 id 为空）= 新建，自动生成 id；title 为空时用 content 前 20 字
/// - 传 id = 更新已存在模板
#[tauri::command]
pub async fn agent_template_save(
    id: Option<String>,
    title: String,
    content: String,
) -> Result<AgentTemplate, String> {
    let mut templates = load_templates_from_disk();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let resolved_title = if title.trim().is_empty() {
        truncate_text(content.trim(), 20)
    } else {
        title.trim().to_string()
    };
    if let Some(id) = id.filter(|s| !s.is_empty()) {
        let t = templates
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("模板不存在: {}", id))?;
        t.title = resolved_title;
        t.content = content;
        t.updated_at = now;
        let snapshot = t.clone();
        save_templates_to_disk(&templates)?;
        return Ok(snapshot);
    }
    let new_id = format!("t_{:x}", now);
    let template = AgentTemplate {
        id: new_id,
        title: resolved_title,
        content,
        is_builtin: false,
        created_at: now,
        updated_at: now,
    };
    templates.push(template.clone());
    save_templates_to_disk(&templates)?;
    Ok(template)
}

/// 删除模板（含内置）
#[tauri::command]
pub async fn agent_template_delete(id: String) -> Result<(), String> {
    let mut templates = load_templates_from_disk();
    let before = templates.len();
    templates.retain(|t| t.id != id);
    if templates.len() == before {
        return Err(format!("模板不存在: {}", id));
    }
    save_templates_to_disk(&templates)
}

/// AI 分析帖子（总结/图片识别/评论概览），流式输出
/// 通过 Tauri 事件推送：ai-chunk / ai-done / ai-error
#[tauri::command]
pub async fn ai_analyze_stream(
    app: AppHandle,
    prompt: String,
    images: Option<Vec<String>>,
) -> Result<(), String> {
    use xhh_agent::config::ProviderKind;
    use xhh_agent::provider::ChatMessage;

    let ac = xhh_agent::config::AgentConfig::load(None).map_err(|e| e.to_string())?;
    let provider_kind = ac.build_provider_config().map_err(|e| e.to_string())?;

    // 构建消息
    let image_list = images.unwrap_or_default();
    let (final_prompt, final_images) = match (&provider_kind, image_list) {
        (ProviderKind::Ollama(_), imgs) if !imgs.is_empty() => {
            let mut data_uris = Vec::new();
            for url in imgs.iter().take(5) {
                match download_to_data_uri(url).await {
                    Ok(du) => data_uris.push(du),
                    Err(e) => tracing::warn!(url = %url, error = %e, "下载图片失败"),
                }
            }
            if data_uris.is_empty() {
                (prompt, vec![])
            } else {
                (prompt, data_uris)
            }
        }
        (_, imgs) => (prompt, imgs),
    };

    let msgs = if final_images.is_empty() {
        vec![ChatMessage::user(final_prompt)]
    } else {
        vec![ChatMessage::user_with_images(final_prompt, final_images)]
    };

    match &provider_kind {
        ProviderKind::OpenAi(c) => stream_openai(&app, c, msgs, ac.temperature).await,
        ProviderKind::Anthropic(c) => stream_anthropic(&app, c, msgs, ac.temperature).await,
        ProviderKind::Ollama(c) => stream_ollama(&app, c, msgs, ac.temperature).await,
    }
}

/// OpenAI SSE 流式
async fn stream_openai(
    app: &AppHandle,
    cfg: &xhh_agent::provider::openai::OpenAiConfig,
    messages: Vec<xhh_agent::provider::ChatMessage>,
    temperature: Option<f32>,
) -> Result<(), String> {
    use serde_json::json;

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": true,
    });
    if let Some(t) = temperature {
        body["temperature"] = json!(t);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let err = format!("HTTP {} - {}", status, truncate(&text, 300));
        let _ = app.emit("ai-error", &err);
        return Err(err);
    }

    let mut resp = resp;
    let mut buf = String::new();

    loop {
        let chunk = resp.chunk().await.map_err(|e| e.to_string())?;
        let Some(chunk) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find("\n\n") {
            let event_block = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();

            for line in event_block.lines() {
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    let _ = app.emit("ai-done", ());
                    return Ok(());
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = v
                        .pointer("/choices/0/delta/content")
                        .and_then(|v| v.as_str())
                    {
                        let _ = app.emit("ai-chunk", content);
                    }
                }
            }
        }
    }

    let _ = app.emit("ai-done", ());
    Ok(())
}

/// Anthropic SSE 流式
async fn stream_anthropic(
    app: &AppHandle,
    cfg: &xhh_agent::provider::anthropic::AnthropicConfig,
    messages: Vec<xhh_agent::provider::ChatMessage>,
    temperature: Option<f32>,
) -> Result<(), String> {
    use serde_json::json;
    use xhh_agent::provider::Role;

    let url = format!("{}/v1/messages", cfg.base_url.trim_end_matches('/'));

    // system 消息单独提取
    let system_text: String = messages
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n\n");

    let mapped: Vec<serde_json::Value> = messages
        .into_iter()
        .filter(|m| m.role != Role::System)
        .map(|m| {
            if m.role == Role::User && !m.images.is_empty() {
                let mut blocks = Vec::new();
                for img in &m.images {
                    blocks.push(json!({"type": "image", "source": {"type": "url", "url": img}}));
                }
                if !m.content.is_empty() {
                    blocks.push(json!({"type": "text", "text": &m.content}));
                }
                json!({"role": "user", "content": blocks})
            } else {
                json!({"role": "user", "content": m.content})
            }
        })
        .collect();

    let mut body = json!({
        "model": cfg.model,
        "max_tokens": cfg.max_tokens,
        "messages": mapped,
        "stream": true,
    });
    if !system_text.is_empty() {
        body["system"] = json!(system_text);
    }
    if let Some(t) = temperature {
        body["temperature"] = json!(t);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .header("x-api-key", &cfg.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let err = format!("HTTP {} - {}", status, truncate(&text, 300));
        let _ = app.emit("ai-error", &err);
        return Err(err);
    }

    let mut resp = resp;
    let mut buf = String::new();

    loop {
        let chunk = resp.chunk().await.map_err(|e| e.to_string())?;
        let Some(chunk) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find("\n\n") {
            let event_block = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();

            let mut event_type = String::new();
            let mut data = String::new();

            for line in event_block.lines() {
                if let Some(e) = line.strip_prefix("event: ") {
                    event_type = e.trim().to_string();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    data = d.trim().to_string();
                }
            }

            match event_type.as_str() {
                "content_block_delta" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Some(text) = v.pointer("/delta/text").and_then(|v| v.as_str()) {
                            let _ = app.emit("ai-chunk", text);
                        }
                    }
                }
                "message_stop" => {
                    let _ = app.emit("ai-done", ());
                    return Ok(());
                }
                "error" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        let msg = v
                            .pointer("/error/message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        let _ = app.emit("ai-error", msg);
                        return Err(msg.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let _ = app.emit("ai-done", ());
    Ok(())
}

/// Ollama NDJSON 流式
async fn stream_ollama(
    app: &AppHandle,
    cfg: &xhh_agent::provider::ollama::OllamaConfig,
    messages: Vec<xhh_agent::provider::ChatMessage>,
    temperature: Option<f32>,
) -> Result<(), String> {
    use serde_json::json;
    use xhh_agent::provider::Role;

    let url = format!("{}/api/chat", cfg.base_url.trim_end_matches('/'));

    let messages_val: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|m| {
            let role_str = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            let mut msg = json!({"role": role_str, "content": m.content});
            if !m.images.is_empty() {
                let imgs: Vec<String> = m
                    .images
                    .iter()
                    .filter_map(|img| img.find(";base64,").map(|pos| img[pos + 8..].to_string()))
                    .collect();
                if !imgs.is_empty() {
                    msg["images"] = json!(imgs);
                }
            }
            msg
        })
        .collect();

    let mut body = json!({
        "model": cfg.model,
        "messages": messages_val,
        "stream": true,
    });
    if let Some(t) = temperature {
        body["options"] = json!({ "temperature": t });
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let err = format!("HTTP {} - {}", status, truncate(&text, 300));
        let _ = app.emit("ai-error", &err);
        return Err(err);
    }

    let mut resp = resp;
    let mut buf = String::new();

    loop {
        let chunk = resp.chunk().await.map_err(|e| e.to_string())?;
        let Some(chunk) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                // Ollama 错误响应
                if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                    let _ = app.emit("ai-error", err);
                    return Err(err.to_string());
                }
                // 完成
                if v.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    let _ = app.emit("ai-done", ());
                    return Ok(());
                }
                // 内容块
                if let Some(content) = v.pointer("/message/content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        let _ = app.emit("ai-chunk", content);
                    }
                }
            }
        }
    }

    let _ = app.emit("ai-done", ());
    Ok(())
}

// ─── Agent Chat (streaming) ─────────────────────────────

use std::collections::BTreeMap;

/// 流式 LLM 调用的返回结果（文本 + 工具调用）
struct StreamOutput {
    content: String,
    tool_calls: Vec<xhh_agent::provider::ToolCall>,
}

/// OpenAI SSE 流式（支持 tool_calls 收集）
async fn stream_agent_openai(
    app: &AppHandle,
    cfg: &xhh_agent::provider::openai::OpenAiConfig,
    messages: Vec<xhh_agent::provider::ChatMessage>,
    tools: Vec<xhh_agent::provider::ToolSpec>,
    temperature: Option<f32>,
) -> Result<StreamOutput, String> {
    use serde_json::json;

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": true,
    });
    if let Some(t) = temperature {
        body["temperature"] = json!(t);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools.iter().map(|t| json!({
            "type": "function",
            "function": { "name": t.name, "description": t.description, "parameters": t.parameters }
        })).collect::<Vec<_>>());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let err = format!("HTTP {} - {}", status, truncate(&text, 300));
        let _ = app.emit("agent-error", &err);
        return Err(err);
    }

    let mut resp = resp;
    let mut buf = String::new();
    let mut content = String::new();
    let mut tc_map: BTreeMap<usize, (String, String, String)> = BTreeMap::new(); // idx -> (id, name, args)

    loop {
        let chunk = resp.chunk().await.map_err(|e| e.to_string())?;
        let Some(chunk) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find("\n\n") {
            let event_block = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();

            for line in event_block.lines() {
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    let tool_calls = tc_map
                        .into_values()
                        .map(|(id, name, args)| xhh_agent::provider::ToolCall {
                            id,
                            name,
                            arguments: args,
                        })
                        .collect();
                    return Ok(StreamOutput {
                        content,
                        tool_calls,
                    });
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(text) = v
                        .pointer("/choices/0/delta/content")
                        .and_then(|v| v.as_str())
                    {
                        content.push_str(text);
                        let _ = app.emit("agent-chunk", text);
                    }
                    // 收集 tool_calls 增量
                    if let Some(tcs) = v
                        .pointer("/choices/0/delta/tool_calls")
                        .and_then(|v| v.as_array())
                    {
                        for tc in tcs {
                            let idx =
                                tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            let entry = tc_map.entry(idx).or_insert((
                                String::new(),
                                String::new(),
                                String::new(),
                            ));
                            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                entry.0 = id.to_string();
                            }
                            if let Some(func) = tc.get("function") {
                                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                    entry.1 = name.to_string();
                                }
                                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                    entry.2.push_str(args);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let tool_calls = tc_map
        .into_values()
        .map(|(id, name, args)| xhh_agent::provider::ToolCall {
            id,
            name,
            arguments: args,
        })
        .collect();
    Ok(StreamOutput {
        content,
        tool_calls,
    })
}

/// Anthropic SSE 流式（支持 tool_use 收集）
async fn stream_agent_anthropic(
    app: &AppHandle,
    cfg: &xhh_agent::provider::anthropic::AnthropicConfig,
    messages: Vec<xhh_agent::provider::ChatMessage>,
    tools: Vec<xhh_agent::provider::ToolSpec>,
    temperature: Option<f32>,
) -> Result<StreamOutput, String> {
    use serde_json::json;
    use xhh_agent::provider::Role;

    let url = format!("{}/v1/messages", cfg.base_url.trim_end_matches('/'));
    let system_text: String = messages
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n\n");

    // 消息映射（含 vision 支持）
    let mapped: Vec<serde_json::Value> = messages
        .into_iter()
        .filter(|m| m.role != Role::System)
        .map(|m| {
            if m.role == Role::User && !m.images.is_empty() {
                let mut blocks: Vec<serde_json::Value> = m
                    .images
                    .iter()
                    .map(|img| json!({"type": "image", "source": {"type": "url", "url": img}}))
                    .collect();
                if !m.content.is_empty() {
                    blocks.push(json!({"type": "text", "text": &m.content}));
                }
                json!({"role": "user", "content": blocks})
            } else {
                json!({"role": "user", "content": m.content})
            }
        })
        .collect();

    let mut body = json!({ "model": cfg.model, "max_tokens": cfg.max_tokens, "messages": mapped, "stream": true });
    if !system_text.is_empty() {
        body["system"] = json!(system_text);
    }
    if let Some(t) = temperature {
        body["temperature"] = json!(t);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools.iter().map(|t| json!({ "name": t.name, "description": t.description, "input_schema": t.parameters })).collect::<Vec<_>>());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(&url)
        .header("x-api-key", &cfg.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let err = format!("HTTP {} - {}", status, truncate(&text, 300));
        let _ = app.emit("agent-error", &err);
        return Err(err);
    }

    let mut resp = resp;
    let mut buf = String::new();
    let mut content = String::new();
    let mut tool_calls: Vec<xhh_agent::provider::ToolCall> = Vec::new();
    let mut cur_tc_id = String::new();
    let mut cur_tc_name = String::new();
    let mut cur_tc_args = String::new();

    loop {
        let chunk = resp.chunk().await.map_err(|e| e.to_string())?;
        let Some(chunk) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find("\n\n") {
            let event_block = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            let mut event_type = String::new();
            let mut data = String::new();
            for line in event_block.lines() {
                if let Some(e) = line.strip_prefix("event: ") {
                    event_type = e.trim().to_string();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    data = d.trim().to_string();
                }
            }

            match event_type.as_str() {
                "content_block_delta" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Some(text) = v.pointer("/delta/text").and_then(|v| v.as_str()) {
                            content.push_str(text);
                            let _ = app.emit("agent-chunk", text);
                        }
                        if let Some(pj) = v.pointer("/delta/partial_json").and_then(|v| v.as_str())
                        {
                            cur_tc_args.push_str(pj);
                        }
                    }
                }
                "content_block_start" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Some(cb) = v.get("content_block") {
                            if cb.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                cur_tc_id = cb
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                cur_tc_name = cb
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                cur_tc_args = String::new();
                            }
                        }
                    }
                }
                "content_block_stop" => {
                    if !cur_tc_id.is_empty() {
                        tool_calls.push(xhh_agent::provider::ToolCall {
                            id: std::mem::take(&mut cur_tc_id),
                            name: std::mem::take(&mut cur_tc_name),
                            arguments: std::mem::take(&mut cur_tc_args),
                        });
                    }
                }
                "message_stop" => {
                    return Ok(StreamOutput {
                        content,
                        tool_calls,
                    });
                }
                "error" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        let msg = v
                            .pointer("/error/message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        let _ = app.emit("agent-error", msg);
                        return Err(msg.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    Ok(StreamOutput {
        content,
        tool_calls,
    })
}

/// Ollama NDJSON 流式（支持 tool_calls）
async fn stream_agent_ollama(
    app: &AppHandle,
    cfg: &xhh_agent::provider::ollama::OllamaConfig,
    messages: Vec<xhh_agent::provider::ChatMessage>,
    tools: Vec<xhh_agent::provider::ToolSpec>,
    temperature: Option<f32>,
) -> Result<StreamOutput, String> {
    use serde_json::json;
    use xhh_agent::provider::Role;

    let url = format!("{}/api/chat", cfg.base_url.trim_end_matches('/'));
    let messages_val: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|m| {
            let role_str = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            let mut msg = json!({"role": role_str, "content": m.content});
            if !m.images.is_empty() {
                let imgs: Vec<String> = m
                    .images
                    .iter()
                    .filter_map(|img| img.find(";base64,").map(|pos| img[pos + 8..].to_string()))
                    .collect();
                if !imgs.is_empty() {
                    msg["images"] = json!(imgs);
                }
            }
            msg
        })
        .collect();

    let mut body = json!({ "model": cfg.model, "messages": messages_val, "stream": true });
    if let Some(t) = temperature {
        body["options"] = json!({ "temperature": t });
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools.iter().map(|t| json!({
            "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters }
        })).collect::<Vec<_>>());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let err = format!("HTTP {} - {}", status, truncate(&text, 300));
        let _ = app.emit("agent-error", &err);
        return Err(err);
    }

    let mut resp = resp;
    let mut buf = String::new();
    let mut content = String::new();
    let mut tool_calls: Vec<xhh_agent::provider::ToolCall> = Vec::new();

    loop {
        let chunk = resp.chunk().await.map_err(|e| e.to_string())?;
        let Some(chunk) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                    let _ = app.emit("agent-error", err);
                    return Err(err.to_string());
                }
                if v.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    return Ok(StreamOutput {
                        content,
                        tool_calls,
                    });
                }
                if let Some(text) = v.pointer("/message/content").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        content.push_str(text);
                        let _ = app.emit("agent-chunk", text);
                    }
                }
                // Ollama 在最终消息或每行中可能携带 tool_calls
                if let Some(arr) = v.pointer("/message/tool_calls").and_then(|v| v.as_array()) {
                    for tc in arr {
                        let id = tc
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let func = tc
                            .get("function")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let name = func
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let arguments = func
                            .get("arguments")
                            .map(|a| match a {
                                serde_json::Value::String(s) => s.clone(),
                                _ => a.to_string(),
                            })
                            .unwrap_or_else(|| "{}".into());
                        if !name.is_empty() {
                            tool_calls.push(xhh_agent::provider::ToolCall {
                                id,
                                name,
                                arguments,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(StreamOutput {
        content,
        tool_calls,
    })
}

/// Agent 流式多轮对话：真正的 SSE/NDJSON 流式 + 工具调用自动执行
/// 事件：agent-chunk / agent-tool / agent-done / agent-error
#[tauri::command]
pub async fn agent_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    message: String,
    confirmations: Option<Vec<AgentToolConfirmationDecision>>,
) -> Result<(), String> {
    use xhh_agent::config::ProviderKind;
    use xhh_agent::provider::ChatMessage;

    ensure_agent_runtime(&state).await?;

    let ac = xhh_agent::config::AgentConfig::load(None).map_err(|e| e.to_string())?;
    let confirmations = confirmations.unwrap_or_default();
    let has_confirmation_decisions = !confirmations.is_empty();
    let provider_kind = ac.build_provider_config().map_err(|e| e.to_string())?;

    let tool_reg = xhh_agent::tool::ToolRegistry::with_defaults();
    let specs = tool_reg.specs();
    let c = state.require_client().await?;
    let max_loops = ac.max_loops.max(1);

    let mut sessions = state.agent_sessions.lock().await;
    let session = sessions
        .active_mut()
        .ok_or("Agent 会话未初始化")?;

    let mut resume_pending = if has_confirmation_decisions {
        session.pending_resume.take()
    } else {
        session.pending_resume = None;
        None
    };
    if has_confirmation_decisions && resume_pending.is_none() {
        return Err("没有等待确认的工具调用".into());
    }

    if resume_pending.is_none() {
        session.messages.push(ChatMessage::user(message));
    }

    let mut loops_used = resume_pending
        .as_ref()
        .map_or(0, |pending| pending.loops_used);
    let mut all_tool_calls = resume_pending
        .as_ref()
        .map_or_else(Vec::new, |pending| pending.completed_tool_calls.clone());

    loop {
        let is_resuming = resume_pending.is_some();
        let output = if let Some(pending) = resume_pending.take() {
            StreamOutput {
                content: String::new(),
                tool_calls: pending.tool_calls,
            }
        } else {
            if loops_used >= max_loops {
                break;
            }
            loops_used += 1;
            let out = match &provider_kind {
                ProviderKind::OpenAi(cfg) => {
                    stream_agent_openai(
                        &app,
                        cfg,
                        session.messages.clone(),
                        specs.clone(),
                        ac.temperature,
                    )
                    .await
                }
                ProviderKind::Anthropic(cfg) => {
                    stream_agent_anthropic(
                        &app,
                        cfg,
                        session.messages.clone(),
                        specs.clone(),
                        ac.temperature,
                    )
                    .await
                }
                ProviderKind::Ollama(cfg) => {
                    stream_agent_ollama(
                        &app,
                        cfg,
                        session.messages.clone(),
                        specs.clone(),
                        ac.temperature,
                    )
                    .await
                }
            };
            match out {
                Ok(o) => o,
                Err(e) => {
                    let snap = session.clone();
                    drop(sessions);
                    persist_session(&snap);
                    let _ = app.emit("agent-error", &e);
                    return Err(e);
                }
            }
        };

        if !is_resuming && (!output.content.is_empty() || !output.tool_calls.is_empty()) {
            session.messages.push(ChatMessage::assistant_with_tools(
                output.content.clone(),
                output.tool_calls.clone(),
            ));
        }

        if !output.tool_calls.is_empty() {
            if ac.confirm_dangerous_tools {
                for tc in &output.tool_calls {
                    let Some(t) = tool_reg.get(&tc.name) else {
                        continue;
                    };
                    if !t.requires_confirmation() {
                        continue;
                    }
                    let decision = confirmations
                        .iter()
                        .find(|c| c.tool_name == tc.name && c.arguments_json == tc.arguments);
                    if decision.is_some() {
                        continue;
                    }

                    let confirmation = t.confirmation(&tc.arguments);
                    let request = AgentToolConfirmationRequest {
                        tool_name: confirmation.tool_name.to_string(),
                        risk_level: confirmation.risk_level,
                        summary: confirmation.summary,
                        arguments_json: confirmation.arguments_json,
                    };
                    session.pending_resume = Some(crate::state::PendingResume {
                        tool_calls: output.tool_calls.clone(),
                        loops_used,
                        completed_tool_calls: all_tool_calls.clone(),
                    });
                    session.touch();
                    let snap = session.clone();
                    drop(sessions);
                    persist_session(&snap);
                    let _ = app.emit("agent-tool-confirmation", &request);
                    let _ = app.emit(
                        "agent-error",
                        "危险操作等待确认，请在确认后重新发送本轮消息",
                    );
                    return Err("危险操作等待确认".into());
                }
            }

            for tc in &output.tool_calls {
                all_tool_calls.push(tc.name.clone());
                let _ = app.emit("agent-tool", &tc.name);
                let tool_result = match tool_reg.get(&tc.name) {
                    Some(t)
                        if ac.confirm_dangerous_tools
                            && t.requires_confirmation()
                            && confirmations.iter().any(|decision| {
                                !decision.approved
                                    && decision.tool_name == tc.name
                                    && decision.arguments_json == tc.arguments
                            }) =>
                    {
                        serde_json::json!({
                            "ok": false,
                            "denied": true,
                            "message": "用户取消了此工具调用"
                        })
                        .to_string()
                    }
                    Some(t) => t
                        .execute(&c, &tc.arguments)
                        .await
                        .unwrap_or_else(|e| format!("工具调用失败: {}", e)),
                    None => format!("工具 {} 不存在", tc.name),
                };
                session
                    .messages
                    .push(ChatMessage::tool(&tc.id, &tc.name, &tool_result));
            }
            session.touch();
            persist_session(session);
            // 工具调用完成，进入下一轮让 LLM 处理工具结果
            continue;
        }

        session.touch();
        persist_session(session);
        let _ = app.emit(
            "agent-done",
            serde_json::json!({ "tool_calls": all_tool_calls, "loops_used": loops_used }),
        );
        return Ok(());
    }

    session.touch();
    persist_session(session);
    let _ = app.emit("agent-error", "达到最大循环次数");
    Err("达到最大循环次数".into())
}

/// 下载远程图片并转为 data URI
async fn download_to_data_uri(url: &str) -> Result<String, String> {
    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("下载失败: {}", e))?;
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = resp.bytes().await.map_err(|e| format!("读取失败: {}", e))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(format!("data:{};base64,{}", content_type, b64))
}

// ─── Image Download ──────────────────────────────────────

/// 截断字符串，用于错误信息
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}

/// 下载图片到用户选择的路径（弹出系统保存对话框）
#[tauri::command]
pub async fn save_image(url: String, title: Option<String>) -> Result<String, String> {
    // 用帖子标题作为文件名，fallback 到 URL 提取
    let default_name = match title {
        Some(ref t) if !t.is_empty() => {
            // 清理文件名中的非法字符
            let safe: String = t
                .chars()
                .map(|c| {
                    if c == '\\'
                        || c == '/'
                        || c == ':'
                        || c == '*'
                        || c == '?'
                        || c == '"'
                        || c == '<'
                        || c == '>'
                        || c == '|'
                    {
                        '_'
                    } else {
                        c
                    }
                })
                .collect::<String>()
                .chars()
                .take(80)
                .collect();
            format!("{}.jpg", safe)
        }
        _ => {
            let raw = url
                .split('/')
                .next_back()
                .unwrap_or("image.jpg")
                .split('?')
                .next()
                .unwrap_or("image.jpg");
            if raw.contains('.') {
                raw.to_string()
            } else {
                format!("{}.jpg", raw)
            }
        }
    };

    let path = rfd::AsyncFileDialog::new()
        .set_file_name(&default_name)
        .save_file()
        .await
        .ok_or("取消了保存")?;

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("下载失败: {}", e))?;
    let bytes: Vec<u8> = resp
        .bytes()
        .await
        .map_err(|e| format!("读取失败: {}", e))?
        .into();

    path.write(&bytes)
        .await
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(path.file_name())
}

// ─── Upload ──────────────────────────────────────────────

/// 选择本地图片并上传到 COS，返回 CDN URL 和尺寸
#[tauri::command]
pub async fn upload_image(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("图片", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
        .pick_file()
        .await
        .ok_or("取消了选择")?;

    let name = file.file_name();
    let bytes = file.read().await;

    // 推断 MIME
    let mimetype = match name.rsplit('.').next() {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        _ => "image/jpeg",
    };

    // 解码获取宽高
    let (width, height) =
        match image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format() {
            Ok(reader) => {
                let dims: Option<(u32, u32)> = reader.into_dimensions().ok();
                dims.unwrap_or((800, 600))
            }
            Err(_) => (800, 600),
        };

    let c = state.require_client().await?;
    let result =
        xhh_core::api::upload::upload_image_bytes(&c, &bytes, &name, mimetype, width, height)
            .await
            .map_err(|e| e.to_string())?;
    tracing::debug!(preview_url = %result.preview_url, key = %result.key, width = width, height = height, "图片上传完成");

    Ok(serde_json::json!({
        "url": result.preview_url,
        "width": width,
        "height": height,
    }))
}

// ─── Topic Search ────────────────────────────────────────

/// 搜索话题/社区（用于发帖时选择话题）
#[tauri::command]
pub async fn search_topic(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_search::search_topic(&c, &keyword)
        .await
        .map_err(|e| e.to_string())
}

// ─── Emoji ──────────────────────────────────────────────

/// Emoji 列表
#[tauri::command]
pub async fn emoji_list(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_emoji::list_emojis(&c).await.map_err(|e| e.to_string())
}

// ─── Notifications ────────────────────────────────────────

/// 消息/通知列表
#[tauri::command]
pub async fn notifications(
    state: State<'_, AppState>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    xhh_core::api::notification::list_all_messages(&c, offset.unwrap_or(0), limit.unwrap_or(20))
        .await
        .map_err(|e| e.to_string())
}

// ─── Favourites ───────────────────────────────────────────

/// 收藏夹列表
#[tauri::command]
pub async fn favour_folders(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_inter::favourite_folders(&c)
        .await
        .map_err(|e| e.to_string())
}

/// 收藏夹内帖子（offset 分页）
#[tauri::command]
pub async fn favour_folder(
    state: State<'_, AppState>,
    folder_id: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_inter::favourite_folder_links(
        &c,
        folder_id.as_deref(),
        offset.unwrap_or(0),
        limit.unwrap_or(30),
    )
    .await
    .map_err(|e| e.to_string())
}

// ─── Follow / User ────────────────────────────────────────

/// 关注用户
#[tauri::command]
pub async fn follow_user(
    state: State<'_, AppState>,
    userid: String,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_user::follow_user(&c, &userid)
        .await
        .map_err(|e| e.to_string())
}

/// 取关用户
#[tauri::command]
pub async fn unfollow_user(
    state: State<'_, AppState>,
    userid: String,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_user::unfollow_user(&c, &userid)
        .await
        .map_err(|e| e.to_string())
}

/// 关注列表
#[tauri::command]
pub async fn following_list(
    state: State<'_, AppState>,
    userid: String,
    _offset: Option<u32>,
    _limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_user::following_list(&c, &userid)
        .await
        .map_err(|e| e.to_string())
}

/// 粉丝列表
#[tauri::command]
pub async fn follower_list(
    state: State<'_, AppState>,
    userid: String,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_user::follower_list(&c, &userid, offset.unwrap_or(0), limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

/// 用户动态（帖子列表）
#[tauri::command]
pub async fn user_events(
    state: State<'_, AppState>,
    userid: Option<String>,
    lastval: Option<String>,
) -> Result<serde_json::Value, String> {
    let c = state.require_client().await?;
    api_feed::user_events(
        &c,
        userid.as_deref(),
        Some(lastval.as_deref().unwrap_or("")),
    )
    .await
    .map_err(|e| e.to_string())
}

// ─── Window ──────────────────────────────────────────────

#[tauri::command]
pub fn window_effect_get() -> crate::prefs::WindowEffect {
    crate::prefs::load_effect()
}

#[tauri::command]
pub fn window_effect_set(
    app: AppHandle,
    effect: crate::prefs::WindowEffect,
) -> Result<(), String> {
    crate::prefs::save_effect(effect)?;
    if let Some(window) = app.get_webview_window("main") {
        crate::apply_window_effect(&window, effect)?;
    }
    Ok(())
}
