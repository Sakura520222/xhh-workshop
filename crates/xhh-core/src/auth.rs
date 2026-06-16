//! 扫码登录模块
//!
//! 三阶段：
//! 1. [`get_qr_code`] — 获取二维码 URL
//! 2. [`poll_qr_state_once`] — 单次轮询（UI/Tauri 用 tokio::spawn 周期调用）
//! 3. [`extract_credentials`] — 从响应 body + Set-Cookie 提取凭据
//!
//! 同时提供 [`qr_login_blocking`] — 一次性 CLI 风格的同步阻塞流程。

use std::time::{Duration, Instant};

use serde_json::Value;
use url::Url;

use crate::client::XhhClient;
use crate::config::Config;
use crate::crypto::generate_token_id;
use crate::error::{Error, Result};
use crate::hkey::build_query_params;

/// 扫码登录返回的二维码信息
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct QrCodeResp {
    /// 二维码图片背后的 URL，可用 `qrcode` crate 渲染为图像
    pub qr_url: String,
    /// 二维码 URL 自身的查询参数（`qr=xxx&app=web`），后续轮询要原样拼接
    pub raw_query: String,
    /// 过期时间（秒）
    pub expire: u64,
}

/// 单次轮询的状态
// 仅在轮询循环中瞬态返回、从不批量存储，大体积 Success 变体无内存浪费问题
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum QrPollResult {
    /// 等待用户扫码 / 确认登录
    Waiting { msg: String },
    /// 已扫码，等待 APP 确认
    Scanned,
    /// 登录成功，包含完整响应（Set-Cookie 在其中）
    Success(QrLoginSuccess),
}

/// 登录成功后解析出的凭据
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct QrLoginSuccess {
    pub pkey: String,
    pub heybox_id: String,
    pub nickname: String,
    /// 已经填好 cookie 的 Config，可直接保存
    pub config: Config,
}

/// 获取二维码
///
/// 对应 `/account/get_qrcode_url/`。注意此接口不需要 `heybox_id`，
/// 因此 [`XhhClient`] 应使用 [`XhhClient::anonymous`] 创建。
pub async fn get_qr_code(client: &XhhClient) -> Result<QrCodeResp> {
    let path = "/account/get_qrcode_url/";
    tracing::debug!(path = %path, "获取二维码");
    let value = client.get(path, &[]).await?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        tracing::warn!(response = %value, "获取二维码失败");
        return Err(Error::ApiError {
            status: "qr_failed".into(),
            msg: value.to_string(),
        });
    }

    let result = value
        .get("result")
        .ok_or_else(|| Error::MissingCredential("result"))?;
    let qr_url = result
        .get("qr_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MissingCredential("qr_url"))?
        .trim()
        .to_string();
    let expire = result.get("expire").and_then(|v| v.as_u64()).unwrap_or(300);

    let parsed = Url::parse(&qr_url)?;
    let raw_query = parsed.query().unwrap_or("").to_string();

    tracing::info!(expire = expire, "二维码获取成功");
    Ok(QrCodeResp {
        qr_url,
        raw_query,
        expire,
    })
}

/// 单次轮询扫码状态
///
/// - `raw_query`: 来自 [`QrCodeResp::raw_query`]
/// - `device_id`: 与获取二维码时同一个设备 ID
///
/// 返回值让调用方决定是否继续轮询。建议每 1 秒调一次。
pub async fn poll_qr_state_once(
    client: &XhhClient,
    raw_query: &str,
    device_id: &str,
) -> Result<QrPollResult> {
    let path = "/account/qr_state/";
    let params = build_query_params(path, "", device_id, 0, "web");
    let url = format!(
        "{}{}?{}&{}",
        crate::BASE_URL,
        path,
        raw_query,
        encode_params(&params)
    );

    let resp = client
        .inner()
        .get(&url)
        .header(reqwest::header::HOST, "api.xiaoheihe.cn")
        .header(reqwest::header::REFERER, "https://www.xiaoheihe.cn/")
        .header(
            reqwest::header::USER_AGENT,
            crate::client::DEFAULT_USER_AGENT,
        )
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %body, "轮询扫码状态 HTTP 错误");
        return Err(Error::ApiError {
            status: status.as_u16().to_string(),
            msg: body,
        });
    }

    // 在消费 response 前先收集所有 Set-Cookie 原始字符串
    // reqwest 的 .cookies() 在某些情况下可能漏 cookie（HttpOnly/特殊字符）
    let raw_set_cookies: Vec<String> = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(String::from))
        .collect();

    let value: Value = resp.json().await?;
    let result = value.get("result").cloned().unwrap_or_default();
    let err = result.get("error").and_then(|v| v.as_str()).unwrap_or("");
    let err_msg = result
        .get("error_msg")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("errmsg").and_then(|v| v.as_str()))
        .or_else(|| result.get("msg").and_then(|v| v.as_str()))
        .unwrap_or("");

    // 直接从原始 Set-Cookie 头中提取 user_pkey 值（更稳健，不依赖 reqwest::cookies）
    let pkey = extract_cookie_value(&raw_set_cookies, "user_pkey");
    let heybox_id_from_cookie = extract_cookie_value(&raw_set_cookies, "user_heybox_id");

    // 成功判定：必须有 user_pkey，这是唯一可靠的成功标志
    if !pkey.is_empty() {
        let heybox_id = if !heybox_id_from_cookie.is_empty() {
            heybox_id_from_cookie
        } else {
            result
                .get("heyboxid")
                .and_then(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
                .unwrap_or_default()
        };
        let nickname = result
            .get("nickname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if heybox_id.is_empty() {
            return Err(Error::MissingCredential("heybox_id"));
        }

        tracing::debug!(pkey_len = pkey.len(), heybox_id = %heybox_id, "检测到登录凭据，提取成功");
        let cfg = build_login_config(&pkey, &heybox_id, &nickname);
        return Ok(QrPollResult::Success(QrLoginSuccess {
            pkey,
            heybox_id,
            nickname,
            config: cfg,
        }));
    }

    let msg = if !err_msg.is_empty() {
        err_msg.to_string()
    } else {
        err.to_string()
    };

    // 状态判断：wait = 等待扫码，其他 = 已扫码待确认
    if err == "wait" || err.is_empty() {
        Ok(QrPollResult::Waiting { msg })
    } else {
        Ok(QrPollResult::Scanned)
    }
}

/// 从一组 Set-Cookie 原始字符串中提取指定 cookie name 的值
///
/// 形如：`user_pkey=ABC; Domain=.xiaoheihe.cn; HttpOnly; Max-Age=604800`
fn extract_cookie_value(set_cookies: &[String], name: &str) -> String {
    let prefix = format!("{}=", name);
    for raw in set_cookies {
        if let Some(idx) = raw.find(&prefix) {
            let after = &raw[idx + prefix.len()..];
            // cookie value 持续到 ';' 或字符串末尾
            let end = after.find(';').unwrap_or(after.len());
            return after[..end].trim_matches('"').to_string();
        }
    }
    String::new()
}

/// 把响应 JSON 中的 heyboxid / nickname 与 Set-Cookie 中的 user_pkey 提取出来
///
/// 注意：由于 [`poll_qr_state_once`] 已经消费了 response，本函数不再访问 Set-Cookie。
/// 上层若要单独走 HTTP 流程，请使用 [`extract_credentials_from_response`]。
pub fn extract_credentials_from_value(value: &Value) -> Result<QrLoginSuccess> {
    let result = value
        .get("result")
        .ok_or_else(|| Error::MissingCredential("result"))?;

    let mut heybox_id = result
        .get("heyboxid")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        })
        .unwrap_or_default();
    let nickname = result
        .get("nickname")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // pkey 通常在 Set-Cookie 中，但 body 里偶尔也有
    let pkey = result
        .get("pkey")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // heybox_id 也可能在 Set-Cookie user_heybox_id 中
    if heybox_id.is_empty() {
        if let Some(h) = result.get("user_heybox_id") {
            heybox_id = h
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| h.as_i64().map(|n| n.to_string()))
                .unwrap_or_default();
        }
    }

    if pkey.is_empty() {
        return Err(Error::MissingCredential("user_pkey"));
    }
    if heybox_id.is_empty() {
        return Err(Error::MissingCredential("heybox_id"));
    }

    Ok(QrLoginSuccess {
        pkey,
        heybox_id,
        nickname,
        config: Config::default(),
    })
}

/// 从完整 `reqwest::Response` 中提取凭据（含 Set-Cookie）
pub async fn extract_credentials_from_response(resp: reqwest::Response) -> Result<QrLoginSuccess> {
    let mut pkey = String::new();
    let mut heybox_id_from_cookie = String::new();
    for cookie in resp.cookies() {
        if cookie.name() == "user_pkey" {
            pkey = cookie.value().to_string();
        } else if cookie.name() == "user_heybox_id" {
            heybox_id_from_cookie = cookie.value().to_string();
        }
    }
    let value: Value = resp.json().await?;
    let mut success = extract_credentials_from_value(&value)?;
    if success.pkey.is_empty() {
        success.pkey = pkey;
    }
    if success.heybox_id.is_empty() {
        success.heybox_id = heybox_id_from_cookie;
    }
    if success.pkey.is_empty() {
        return Err(Error::MissingCredential("user_pkey"));
    }
    if success.heybox_id.is_empty() {
        return Err(Error::MissingCredential("heybox_id"));
    }
    success.config = build_login_config(&success.pkey, &success.heybox_id, &success.nickname);
    Ok(success)
}

/// 把 pkey + heybox_id + nickname 装配成可持久化的 [`Config`]
///
/// Cookie 字段格式：
/// `user_pkey=...; user_heybox_id=...; x_xhh_tokenid=...`
pub fn build_login_config(pkey: &str, heybox_id: &str, nickname: &str) -> Config {
    let token_id = generate_token_id();
    Config {
        pkey: pkey.into(),
        heybox_id: heybox_id.into(),
        nickname: nickname.into(),
        x_xhh_tokenid: token_id.clone(),
        login_time: chrono::Utc::now().timestamp(),
        cookie: format!(
            "user_pkey={}; user_heybox_id={}; x_xhh_tokenid={}",
            pkey, heybox_id, token_id
        ),
        ..Default::default()
    }
}

/// 阻塞式（异步但自包含）的完整登录流程，CLI 用
pub async fn qr_login_blocking(force: bool, existing_config: Option<Config>) -> Result<Config> {
    if !force {
        if let Some(cfg) = &existing_config {
            if cfg.has_credentials() {
                return Ok(cfg.clone());
            }
        }
    }

    // 生成或复用 device_id
    let mut device_id = String::new();
    if let Some(cfg) = &existing_config {
        device_id = cfg.device_id.clone();
    }
    if device_id.is_empty() {
        device_id = uuid::Uuid::new_v4().simple().to_string();
    }

    let anon_client = XhhClient::anonymous(Some(device_id.clone()))?;

    tracing::info!("正在获取二维码...");
    let qr = get_qr_code(&anon_client).await?;
    tracing::info!(qr_url = %qr.qr_url, "请使用小黑盒 APP 扫码");

    // 用同样的 device_id 但创建一个能拿 Set-Cookie 的 client
    let poll_client = XhhClient::anonymous(Some(device_id.clone()))?;
    let deadline = Instant::now() + Duration::from_secs(qr.expire);
    let mut last_log = Instant::now();

    loop {
        if Instant::now() > deadline {
            return Err(Error::QrTimeout { timeout: qr.expire });
        }

        match poll_qr_state_once(&poll_client, &qr.raw_query, &device_id).await? {
            QrPollResult::Waiting { msg } => {
                if last_log.elapsed() > Duration::from_millis(1500) {
                    tracing::info!(msg = %msg, "等待扫码...");
                    last_log = Instant::now();
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            QrPollResult::Scanned => {
                tracing::info!("已扫码，请在 APP 内确认登录");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            QrPollResult::Success(mut success) => {
                // 拼回 device_id
                success.config.device_id = device_id.clone();
                success.config.save(None)?;
                tracing::info!(nickname = %success.nickname, heybox_id = %success.heybox_id, "登录成功");
                return Ok(success.config);
            }
        }
    }
}

/// 用 `form_urlencoded` 把签名参数拼成 `k=v&k=v` 字符串
fn encode_params(params: &[(String, String)]) -> String {
    let mut s = form_urlencoded::Serializer::new(String::new());
    for (k, v) in params {
        s.append_pair(k, v);
    }
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_creds_from_value_basic() {
        let v = json!({
            "status": "ok",
            "result": {
                "heyboxid": "12345",
                "nickname": "tester",
                "pkey": "ABC"
            }
        });
        let s = extract_credentials_from_value(&v).unwrap();
        assert_eq!(s.heybox_id, "12345");
        assert_eq!(s.nickname, "tester");
        assert_eq!(s.pkey, "ABC");
    }

    #[test]
    fn extract_creds_int_heyboxid() {
        let v = json!({
            "status": "ok",
            "result": { "heyboxid": 99, "pkey": "X" }
        });
        let s = extract_credentials_from_value(&v).unwrap();
        assert_eq!(s.heybox_id, "99");
    }

    #[test]
    fn extract_creds_missing_pkey() {
        let v = json!({"status": "ok", "result": {"heyboxid": "1"}});
        let r = extract_credentials_from_value(&v);
        assert!(matches!(r, Err(Error::MissingCredential(_))));
    }

    #[test]
    fn build_config_cookie_format() {
        let cfg = build_login_config("P", "9", "n");
        assert!(cfg.cookie.contains("user_pkey=P"));
        assert!(cfg.cookie.contains("user_heybox_id=9"));
        assert!(cfg.cookie.starts_with("user_pkey=P"));
        assert!(!cfg.x_xhh_tokenid.is_empty());
        assert_eq!(cfg.nickname, "n");
    }

    #[test]
    fn encode_params_roundtrip() {
        let p = vec![("a".into(), "1".into()), ("b".into(), "x y".into())];
        let s = encode_params(&p);
        assert!(s.contains("a=1"));
        assert!(s.contains("b=x+y"));
    }

    #[test]
    fn extract_cookie_value_basic() {
        let raw =
            vec!["user_pkey=ABCxyz; Domain=.xiaoheihe.cn; HttpOnly; Max-Age=604800".to_string()];
        assert_eq!(extract_cookie_value(&raw, "user_pkey"), "ABCxyz");
        assert_eq!(extract_cookie_value(&raw, "user_heybox_id"), "");
    }

    #[test]
    fn extract_cookie_value_quoted() {
        let raw = vec!["user_pkey=\"quoted_value\"; Path=/".to_string()];
        assert_eq!(extract_cookie_value(&raw, "user_pkey"), "quoted_value");
    }

    #[test]
    fn extract_cookie_value_multiple_in_array() {
        let raw = vec![
            "lang=zh; Path=/".to_string(),
            "user_pkey=P; HttpOnly".to_string(),
            "user_heybox_id=12345; Path=/".to_string(),
        ];
        assert_eq!(extract_cookie_value(&raw, "user_pkey"), "P");
        assert_eq!(extract_cookie_value(&raw, "user_heybox_id"), "12345");
    }

    #[test]
    fn extract_cookie_value_no_semicolon_at_end() {
        let raw = vec!["user_pkey=just_value".to_string()];
        assert_eq!(extract_cookie_value(&raw, "user_pkey"), "just_value");
    }
}
