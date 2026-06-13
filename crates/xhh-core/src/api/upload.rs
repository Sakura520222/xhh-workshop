//! COS 图片上传（四步流程）
//!
//!
//! 1. 获取上传信息 → keys / region / bucket / host
//! 2. 获取临时凭证 → tmpSecretId / tmpSecretKey / sessionToken
//! 3. PUT 二进制到 COS（用临时凭证签 Authorization）
//! 4. 回调确认 → preview_url
//!
//! 关键常量（§25.1）：
//! - bucket: `imgheybox-1251007209`
//! - region: `ap-shanghai`
//! - CDN:    `imgheybox.max-c.com`

use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::{Digest as Sha1Digest, Sha1};

use crate::client::XhhClient;
use crate::error::{Error, Result};

type HmacSha1 = Hmac<Sha1>;

const BUCKET: &str = "imgheybox-1251007209";
const HOST_CDN: &str = "imgheybox.max-c.com";
const COS_HOST: &str = "imgheybox-1251007209.cos.ap-shanghai.myqcloud.com";

const PATH_INFO: &str = "/bbs/app/api/qcloud/cos/upload/info/v2";
const PATH_TOKEN: &str = "/bbs/app/api/qcloud/cos/upload/token/v2";
const PATH_CALLBACK: &str = "/bbs/app/api/qcloud/cos/upload/callback/v2";

/// 单文件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub mimetype: String,
    pub fsize: u64,
    pub width: u32,
    pub height: u32,
}

/// Step 1 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInfo {
    /// COS 对象 key（如 `/web/bbs/2026/06/08/abcdef.png`）
    pub keys: Vec<String>,
    pub region: String,
    pub bucket: String,
    pub host: String,
}

/// Step 2 临时凭证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadCredentials {
    #[serde(rename = "tmpSecretId")]
    pub tmp_secret_id: String,
    #[serde(rename = "tmpSecretKey")]
    pub tmp_secret_key: String,
    #[serde(rename = "sessionToken")]
    pub session_token: String,
}

/// Step 2 完整响应（credentials 在 result 字段下）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadTokenResp {
    pub credentials: UploadCredentials,
    #[serde(rename = "startTime")]
    pub start_time: i64,
    #[serde(rename = "expiredTime")]
    pub expired_time: i64,
}

/// Step 4 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadCallback {
    pub preview_urls: Vec<String>,
    pub thumbs: Vec<String>,
}

/// 完整上传结果
#[derive(Debug, Clone, Serialize)]
pub struct UploadResult {
    pub preview_url: String,
    pub key: String,
}

/// Step 1: 获取上传信息
pub async fn get_upload_info(client: &XhhClient, file_infos: &[FileInfo]) -> Result<UploadInfo> {
    tracing::debug!(files_len = file_infos.len(), "上传 Step 1: 获取上传信息");
    let file_infos_json = serde_json::to_string(file_infos)?;
    let mut body = BTreeMap::new();
    body.insert("file_infos".into(), file_infos_json);
    body.insert("scope".into(), "bbs".into());
    body.insert("need_cache".into(), "0".into());

    let value = client.post(PATH_INFO, &body, 0).await?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(Error::UploadFailed(value.to_string()));
    }
    let result = value
        .get("result")
        .ok_or_else(|| Error::UploadFailed("missing result".into()))?;
    let info: UploadInfo = serde_json::from_value(result.clone())?;
    Ok(info)
}

/// Step 2: 获取上传临时凭证
pub async fn get_upload_token(
    client: &XhhClient,
    keys: &[String],
    mimetypes: &[String],
) -> Result<UploadTokenResp> {
    tracing::debug!(keys = ?keys, "上传 Step 2: 获取临时凭证");
    let keys_json = serde_json::to_string(keys)?;
    let mimetypes_json = serde_json::to_string(mimetypes)?;

    let mut body = BTreeMap::new();
    body.insert("bucket".into(), BUCKET.into());
    body.insert("keys".into(), keys_json);
    body.insert("mimetypes".into(), mimetypes_json);
    body.insert("is_multipart_upload".into(), "0".into());

    let value = client.post(PATH_TOKEN, &body, 0).await?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(Error::UploadFailed(value.to_string()));
    }
    let result = value
        .get("result")
        .ok_or_else(|| Error::UploadFailed("missing result".into()))?;
    let resp: UploadTokenResp = serde_json::from_value(result.clone())?;
    Ok(resp)
}

/// Step 3: PUT 文件到 COS
///
/// 使用临时凭证构建 COS V5 签名（基于 HMAC-SHA1）。
pub async fn put_to_cos(
    client: &XhhClient,
    creds: &UploadCredentials,
    key: &str,
    bytes: &[u8],
    mimetype: &str,
) -> Result<()> {
    tracing::debug!(key = %key, size = bytes.len(), "上传 Step 3: PUT 到 COS");
    let now = chrono::Utc::now().timestamp();
    let exp = now + 600;
    let key_time = format!("{};{}", now, exp);

    // SignKey = HMAC-SHA1(SecretKey, KeyTime)
    let sign_key = {
        let mut mac = HmacSha1::new_from_slice(creds.tmp_secret_key.as_bytes())
            .map_err(|e| Error::Other(format!("HMAC error: {}", e)))?;
        mac.update(key_time.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    // FormatString: "put\n{key}\n\nhost={cos_host}\n"
    let format_string = format!("put\n{}\n\nhost={}\n", key, COS_HOST);

    // StringToSign = "sha1\n{KeyTime}\n{SHA1(FormatString)}\n"
    let string_to_sign = format!(
        "sha1\n{}\n{}\n",
        key_time,
        hex::encode(Sha1::digest(format_string.as_bytes()))
    );

    // Signature = HMAC-SHA1(SignKey, StringToSign)
    let signature = {
        let mut mac = HmacSha1::new_from_slice(sign_key.as_bytes())
            .map_err(|e| Error::Other(format!("HMAC error: {}", e)))?;
        mac.update(string_to_sign.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    let auth = format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list=host&q-url-param-list=&q-signature={}",
        creds.tmp_secret_id, key_time, key_time, signature
    );

    let url = format!("https://{}{}", COS_HOST, key);
    let resp = client
        .inner()
        .put(&url)
        .header("Authorization", auth)
        .header("x-cos-security-token", &creds.session_token)
        .header("Content-Type", mimetype)
        .header("Host", COS_HOST)
        .body(bytes.to_vec())
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, url = %url, body = %body, "COS PUT 失败");
        return Err(Error::UploadFailed(format!(
            "COS PUT {} {} {}",
            status, url, body
        )));
    }
    Ok(())
}

/// Step 4: 回调确认
pub async fn callback(client: &XhhClient, keys: &[String]) -> Result<UploadCallback> {
    tracing::debug!(keys = ?keys, "上传 Step 4: 回调确认");
    let keys_json = serde_json::to_string(keys)?;
    let mut body = BTreeMap::new();
    body.insert("keys".into(), keys_json);

    let value = client
        .request(
            reqwest::Method::POST,
            PATH_CALLBACK,
            0,
            Some(&body),
            &[("is_finished", "true")],
        )
        .await?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(Error::UploadFailed(value.to_string()));
    }
    let result = value
        .get("result")
        .ok_or_else(|| Error::UploadFailed("missing result".into()))?;
    let cb: UploadCallback = serde_json::from_value(result.clone())?;
    Ok(cb)
}

/// 一键上传图片字节（自动跑完四步流程）
pub async fn upload_image_bytes(
    client: &XhhClient,
    bytes: &[u8],
    name: &str,
    mimetype: &str,
    width: u32,
    height: u32,
) -> Result<UploadResult> {
    let fsize = bytes.len() as u64;
    tracing::info!(name = %name, fsize = fsize, width = width, height = height, "一键上传图片");
    let info = FileInfo {
        name: name.into(),
        mimetype: mimetype.into(),
        fsize,
        width,
        height,
    };

    // Step 1
    let upload_info = get_upload_info(client, &[(info.clone())]).await?;
    let returned_key = upload_info
        .keys
        .first()
        .ok_or_else(|| Error::UploadFailed("no key returned".into()))?
        .clone();

    // Step 2
    let creds_resp = get_upload_token(client, &[returned_key.clone()], &[mimetype.into()]).await?;
    let creds = &creds_resp.credentials;

    // Step 3
    put_to_cos(client, creds, &returned_key, bytes, mimetype).await?;

    // Step 4
    let cb = callback(client, &[returned_key.clone()]).await?;
    tracing::debug!(preview_urls = ?cb.preview_urls, thumbs = ?cb.thumbs, "上传回调完成");
    let preview_url = cb
        .preview_urls
        .first()
        .cloned()
        .unwrap_or_else(|| format!("https://{}{}", HOST_CDN, returned_key));

    Ok(UploadResult {
        preview_url,
        key: returned_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fileinfo_json_roundtrip() {
        let f = FileInfo {
            name: "a.png".into(),
            mimetype: "image/png".into(),
            fsize: 1234,
            width: 800,
            height: 600,
        };
        let j = serde_json::to_string(&f).unwrap();
        assert!(j.contains("\"name\":\"a.png\""));
        assert!(j.contains("\"fsize\":1234"));
        let back: FileInfo = serde_json::from_str(&j).unwrap();
        assert_eq!(back.fsize, 1234);
    }

    #[test]
    fn cos_constants_correct() {
        assert_eq!(BUCKET, "imgheybox-1251007209");
        assert_eq!(HOST_CDN, "imgheybox.max-c.com");
        assert!(COS_HOST.contains("ap-shanghai"));
    }
}
