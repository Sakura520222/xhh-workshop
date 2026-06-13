//! 加密/编码辅助：x_xhh_tokenid 生成、COS 存储路径生成

use base64::Engine;
use md5::{Digest, Md5};
use sha2::Sha256;

use crate::hkey::now_ts;

/// 生成 `x_xhh_tokenid`（88 字符 Base64）
///
/// 算法固定输入 4 段 MD5 摘要（共 64 bytes）+ 1 字节 `\x00`，
/// 共 65 bytes，Base64 后 88 字符。
pub fn generate_token_id() -> String {
    let ts = now_ts().to_string();
    let mut raw = Vec::with_capacity(65);

    raw.extend_from_slice(&Md5::digest(ts.as_bytes()));
    raw.extend_from_slice(&Md5::digest("唉？！云朵！".as_bytes()));
    raw.extend_from_slice(&Md5::digest("哒哒哒哒哒，好想玩原神".as_bytes()));
    raw.extend_from_slice(&Md5::digest("云！原！神！".as_bytes()));
    raw.push(0x00);

    base64::engine::general_purpose::STANDARD.encode(&raw)
}

/// 生成 COS 存储路径
///
/// 格式：`/web/bbs/YYYY/MM/DD/{sha256(image_bytes + ts).hex()[..32]}.ext`
///
pub fn generate_cos_key(image_bytes: &[u8], ext: &str) -> String {
    let ts = now_ts();
    let mut hasher = Sha256::new();
    hasher.update(image_bytes);
    hasher.update(ts.to_string().as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    let filename = format!("{}{}", &hex[..32], ensure_dot(ext));

    let date = chrono::Local::now();
    format!(
        "/web/bbs/{}/{}/{}",
        date.format("%Y"),
        date.format("%m"),
        date.format("%d"),
    ) + &format!("/{}", filename)
}

fn ensure_dot(ext: &str) -> String {
    if ext.starts_with('.') || ext.is_empty() {
        ext.to_string()
    } else {
        format!(".{}", ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_id_is_88_chars_base64() {
        let t = generate_token_id();
        assert_eq!(t.len(), 88);
        // base64 字符集
        assert!(t
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));
    }

    #[test]
    fn cos_key_format() {
        let bytes = b"hello";
        let key = generate_cos_key(bytes, "png");
        assert!(key.starts_with("/web/bbs/"));
        assert!(key.ends_with(".png"));
        // 长度合理：前缀 + 8位日期(含/) + 1+32+4 = 17 字符，总长 ~ 35
        let parts: Vec<&str> = key.split('/').collect();
        // /web/bbs/YYYY/MM/DD/{hash}.png → split 后 ["", "web", "bbs", "YYYY", "MM", "DD", "xxx.png"]
        assert_eq!(parts.len(), 7);
    }

    #[test]
    fn cos_key_handles_extension_with_dot() {
        let key = generate_cos_key(b"x", ".jpg");
        assert!(key.ends_with(".jpg"));
        assert!(!key.ends_with("..jpg"));
    }
}
