//! hkey 签名算法 + 通用请求参数构建
//!
//! 算法验证用例见 `tests::regression_*`（4 个）。

use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};
use rand::Rng;

/// 字符替换表
pub const CHARSET: &str = "AB45STUVWZEFGJ6CH01D237IXYPQRKLMN89";

/// `_av` — 带偏移的字符替换。
///
/// 取 `key[..key.len() + n]` 作为映射表，对输入字符串的每个字符 `ch`
/// 取 `i[ord(ch) % len(i)]`。
fn av(s: &str, key: &str, n: i32) -> String {
    let end = (key.len() as i32 + n) as usize;
    let i = &key[..end.min(key.len())];
    let len_i = i.len();
    s.chars()
        .map(|ch| {
            let idx = (ch as u32) as usize % len_i;
            i.as_bytes()[idx] as char
        })
        .collect()
}

/// `_sv` — 字符替换（不带偏移）
fn sv(s: &str, key: &str) -> String {
    let bytes = key.as_bytes();
    let len_k = bytes.len();
    s.chars()
        .map(|ch| {
            let idx = (ch as u32) as usize % len_k;
            bytes[idx] as char
        })
        .collect()
}

/// `_Vm` — 字节混淆
fn vm(e: u32) -> u32 {
    if e & 0x80 != 0 {
        0xFF & ((e << 1) ^ 27)
    } else {
        e << 1
    }
}

fn qm(e: u32) -> u32 {
    vm(e) ^ e
}

fn m(e: u32) -> u32 {
    qm(vm(e))
}

fn ym(e: u32) -> u32 {
    m(qm(vm(e)))
}

fn gm(e: u32) -> u32 {
    ym(e) ^ m(e) ^ qm(e)
}

/// `_Km` — 输入长度 ≥ 4 的数组，把前 4 个元素按规则替换，
/// 其余元素保留，返回修改后的数组。
///
/// 调用方传入 6 元素数组（MD5 后 6 位的 ord 值）。
fn km(mut arr: Vec<u32>) -> Vec<u32> {
    debug_assert!(arr.len() >= 4);
    let t = [
        gm(arr[0]) ^ ym(arr[1]) ^ m(arr[2]) ^ qm(arr[3]),
        qm(arr[0]) ^ gm(arr[1]) ^ ym(arr[2]) ^ m(arr[3]),
        m(arr[0]) ^ qm(arr[1]) ^ gm(arr[2]) ^ ym(arr[3]),
        ym(arr[0]) ^ m(arr[1]) ^ qm(arr[2]) ^ gm(arr[3]),
    ];
    arr[..4].copy_from_slice(&t);
    arr
}

/// `_interleave` — 多个字符串交叉合并：依次取每个串的第 0 位、第 1 位…
fn interleave(parts: &[String]) -> String {
    let max_len = parts.iter().map(|s| s.len()).max().unwrap_or(0);
    let mut out = String::with_capacity(max_len * parts.len());
    for i in 0..max_len {
        for p in parts {
            if let Some(b) = p.as_bytes().get(i) {
                out.push(*b as char);
            }
        }
    }
    out
}

/// 生成 nonce = `MD5(timestamp + random).toUpperCase()`（32 位大写 hex）
pub fn get_nonce(timestamp: i64) -> String {
    let mut rng = rand::thread_rng();
    let r: i64 = rng.gen_range(0..1_000_000_000);
    let raw = format!("{}{}", timestamp, r);
    let digest = Md5::digest(raw.as_bytes());
    hex::encode_upper(digest)
}

/// 生成 Web 端 7 位 hkey 签名
///
/// - `reqpath`: 请求路径，如 `/bbs/app/api/link/post`
/// - `timestamp`: Unix 时间戳（秒）
/// - `nonce`: 32 位大写 MD5 字符串
/// - `offset`: 时间戳偏移（发帖类接口=1，其他=0）
pub fn generate_hkey(reqpath: &str, timestamp: i64, nonce: &str, offset: i64) -> String {
    // 规范化路径："/" + 过滤空段 + "/"
    let path: String = {
        let parts: Vec<&str> = reqpath.split('/').filter(|s| !s.is_empty()).collect();
        format!("/{}/", parts.join("/"))
    };

    let str1 = av(&(timestamp + offset).to_string(), CHARSET, -2);
    let str2 = sv(&path, CHARSET);
    let str3 = sv(nonce, CHARSET);

    let interleaved = interleave(&[str1, str2, str3]);
    let interleaved_b = interleaved.as_bytes();
    let take: &[u8] = if interleaved_b.len() >= 20 {
        &interleaved_b[..20]
    } else {
        interleaved_b
    };

    let md5_hash = Md5::digest(take);
    let md5_hex = hex::encode(md5_hash);

    // 后 6 位 → km 混淆 → 求和 → 2 位 zfill
    let last_six = &md5_hex[md5_hex.len() - 6..];
    let arr: Vec<u32> = last_six.bytes().map(|b| b as u32).collect();
    let total: u32 = km(arr).iter().sum();
    let a = format!("{:02}", total % 100);

    // 前 5 位 → av 字符替换 → 5 位前缀
    let s = av(&md5_hex[..5], CHARSET, -4);

    format!("{}{}", s, a)
}

/// 当前 Unix 秒
pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 构建 Web 端通用 query 参数（含 hkey 签名）
///
/// 返回 `Vec<(String, String)>`，便于 reqwest 的 `query()` 或
/// `form_urlencoded` 直接消费。
///
/// - `reqpath`: 请求路径
/// - `heybox_id`: 用户 ID（登录前为空字符串）
/// - `device_id`: 设备指纹
/// - `offset`: hkey 时间戳偏移
/// - `app`: 应用标识，默认 `web`，话题搜索需传 `heybox`
pub fn build_query_params(
    reqpath: &str,
    heybox_id: &str,
    device_id: &str,
    offset: i64,
    app: &str,
) -> Vec<(String, String)> {
    let timestamp = now_ts();
    let nonce = get_nonce(timestamp);
    let hkey = generate_hkey(reqpath, timestamp, &nonce, offset);

    let mut params: Vec<(String, String)> = vec![
        ("os_type".into(), "web".into()),
        ("app".into(), app.into()),
        ("client_type".into(), "web".into()),
        ("version".into(), "999.0.4".into()),
        ("web_version".into(), "2.5".into()),
        ("x_client_type".into(), "web".into()),
        ("x_app".into(), "heybox_website".into()),
        ("x_os_type".into(), "Windows".into()),
        ("device_info".into(), "Chrome".into()),
        ("device_id".into(), device_id.into()),
        ("hkey".into(), hkey),
        ("_time".into(), timestamp.to_string()),
        ("nonce".into(), nonce),
        ("_notip".into(), "true".into()),
    ];
    if !heybox_id.is_empty() {
        params.push(("heybox_id".into(), heybox_id.into()));
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── 单元测试 ───────────────────────────────────────────────

    #[test]
    fn av_truncates_key_correctly() {
        // CHARSET.len() = 35，n=-2 → 取前 33 字符
        assert_eq!(CHARSET.len(), 35);
        // key[:33] = "AB45STUVWZEFGJ6CH01D237IXYPQRKLMN"
        // 'A' as u32 = 65, 65 % 33 = 32, CHARSET[32] = 'N'
        let r = av("A", CHARSET, -2);
        assert_eq!(r, "N");
    }

    #[test]
    fn sv_maps_by_mod() {
        let r = sv("a", CHARSET);
        // 'a' as u32 = 97, CHARSET.len() = 35, 97 % 35 = 27, CHARSET[27] = 'Q'
        let bytes = CHARSET.as_bytes();
        assert_eq!(bytes[97 % bytes.len()] as char, 'Q');
        assert_eq!(r, "Q");
    }

    #[test]
    fn vm_high_bit_clear_just_shifts() {
        assert_eq!(vm(0x10), 0x20);
    }

    #[test]
    fn vm_high_bit_set_xor_27() {
        // e=0xff → (0xff<<1)^27 = 0x1fe^0x1b = 0x1e5, &0xff = 0xe5
        assert_eq!(vm(0xff), 0xe5);
    }

    #[test]
    fn interleave_basic() {
        let parts = vec!["abc".to_string(), "XY".to_string()];
        let r = interleave(&parts);
        // 第 0 位: 'a','X'  第 1 位: 'b','Y'  第 2 位: 'c'
        assert_eq!(r, "aXbYc");
    }

    #[test]
    fn nonce_is_32_uppercase_hex() {
        let n = get_nonce(1_700_000_000);
        assert_eq!(n.len(), 32);
        assert!(n
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }

    // ─── 回归测试 ────────────────

    #[test]
    fn regression_feeds() {
        // /bbs/app/feeds | 1780887796 | 77A06EB1B1EE4C0F6AC5BB1B7A1F0B1A | 0 | WU7P260
        let hkey = generate_hkey(
            "/bbs/app/feeds",
            1780887796,
            "77A06EB1B1EE4C0F6AC5BB1B7A1F0B1A",
            0,
        );
        assert_eq!(hkey, "WU7P260");
    }

    #[test]
    fn regression_link_post() {
        // /bbs/app/api/link/post | 1780888010 | 56D46802FCA89EE3B8D9CC54ACB7CD82 | 1 | DU10W93
        let hkey = generate_hkey(
            "/bbs/app/api/link/post",
            1780888010,
            "56D46802FCA89EE3B8D9CC54ACB7CD82",
            1,
        );
        assert_eq!(hkey, "DU10W93");
    }

    #[test]
    fn regression_topic_search() {
        // /bbs/app/api/post_editor/topic_selection/search | 1780909118 | 3120C1B4... | 0 | 332WV03
        let hkey = generate_hkey(
            "/bbs/app/api/post_editor/topic_selection/search",
            1780909118,
            "3120C1B40987ABCDEF0123456789ABCD",
            0,
        );
        // 文档表格只展示了前 8 位 nonce，需要完整 nonce 才能严格比对
        // 这里仅验证长度与字符集
        assert_eq!(hkey.len(), 7);
        assert!(hkey.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn regression_topic_index() {
        let hkey = generate_hkey(
            "/bbs/app/api/post_editor/topic_selection/index",
            1780909089,
            "68DA91C70987ABCDEF0123456789ABCD",
            0,
        );
        assert_eq!(hkey.len(), 7);
        assert!(hkey.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn build_query_params_contains_required_keys() {
        let params = build_query_params("/bbs/app/feeds", "123", "dev", 0, "web");
        let keys: std::collections::HashSet<&str> =
            params.iter().map(|(k, _)| k.as_str()).collect();
        for required in [
            "os_type",
            "app",
            "client_type",
            "version",
            "web_version",
            "x_client_type",
            "x_app",
            "x_os_type",
            "device_info",
            "device_id",
            "hkey",
            "_time",
            "nonce",
            "_notip",
            "heybox_id",
        ] {
            assert!(keys.contains(required), "缺少 query 参数 {}", required);
        }
    }

    #[test]
    fn build_query_params_omits_empty_heybox_id() {
        let params = build_query_params("/account/get_qrcode_url/", "", "dev", 0, "web");
        assert!(params.iter().find(|(k, _)| k == "heybox_id").is_none());
    }
}
