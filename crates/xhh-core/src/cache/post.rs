//! 帖子文本缓存：按 link_id 落盘完整首屏响应

use std::path::PathBuf;

use serde_json::Value;

use crate::cache::{CacheAreaStats, CacheConfig, IndexEntry};
use crate::error::Result;

pub struct PostCache {
    cfg: CacheConfig,
}

impl PostCache {
    pub fn new(cfg: CacheConfig) -> Self {
        Self { cfg }
    }

    fn dir(&self) -> PathBuf {
        self.cfg.root_dir.join("posts")
    }

    fn data_path(&self, link_id: &str) -> PathBuf {
        self.dir().join(format!("{}.json", sanitize(link_id)))
    }

    fn index_path(&self) -> PathBuf {
        self.dir().join("_index.json")
    }

    pub fn enabled(&self) -> bool {
        self.cfg.enabled
    }

    /// 命中返回缓存的首屏响应；未启用 / 未命中返回 None
    pub fn get(&self, link_id: &str) -> Result<Option<Value>> {
        if !self.cfg.enabled {
            return Ok(None);
        }
        let path = self.data_path(link_id);
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)?;
        let v: Value = serde_json::from_str(&text)?;
        Ok(Some(v))
    }

    /// 写入首屏响应并更新索引（不触发淘汰，由 CacheManager 统一处理）
    pub fn put(&self, link_id: &str, response: Value) -> Result<()> {
        if !self.cfg.enabled {
            return Ok(());
        }
        let path = self.data_path(link_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(&response)?;
        std::fs::write(&path, &bytes)?;
        let mut idx = self.read_index()?;
        idx.insert(
            link_id.to_string(),
            IndexEntry {
                size_bytes: bytes.len() as u64,
                fetched_at: super::now_ts(),
            },
        );
        self.write_index(&idx)?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let dir = self.dir();
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::create_dir_all(&dir)?;
        self.write_index(&Default::default())?;
        Ok(())
    }

    pub fn stats(&self) -> Result<CacheAreaStats> {
        let idx = self.read_index()?;
        let bytes = idx.values().map(|e| e.size_bytes).sum();
        Ok(CacheAreaStats {
            count: idx.len(),
            bytes,
        })
    }

    pub(crate) fn list_entries(&self) -> Result<Vec<(String, IndexEntry)>> {
        Ok(self.read_index()?.into_iter().collect())
    }

    pub(crate) fn remove(&self, link_id: &str) -> Result<()> {
        let path = self.data_path(link_id);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let mut idx = self.read_index()?;
        if idx.remove(link_id).is_some() {
            self.write_index(&idx)?;
        }
        Ok(())
    }

    fn read_index(&self) -> Result<std::collections::BTreeMap<String, IndexEntry>> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(Default::default());
        }
        let text = std::fs::read_to_string(&path)?;
        if text.trim().is_empty() {
            return Ok(Default::default());
        }
        Ok(serde_json::from_str(&text)?)
    }

    fn write_index(&self, idx: &std::collections::BTreeMap<String, IndexEntry>) -> Result<()> {
        let path = self.index_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(idx)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}

/// 把 key 收敛为安全文件名片段，防路径穿越
pub(crate) fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_root() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("xhh_postcache_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn cache() -> PostCache {
        PostCache::new(CacheConfig::for_root(tmp_root()))
    }

    #[test]
    fn get_miss_returns_none() {
        let c = cache();
        assert!(c.get("999999").unwrap().is_none());
    }

    #[test]
    fn put_then_get_hits() {
        let c = cache();
        let v = json!({"result": {"link": {"linkid": "1", "title": "hi"}}});
        c.put("123", v.clone()).unwrap();
        assert_eq!(c.get("123").unwrap(), Some(v));
    }

    #[test]
    fn disabled_skips_cache() {
        let mut cfg = CacheConfig::for_root(tmp_root());
        cfg.enabled = false;
        let c = PostCache::new(cfg);
        c.put("1", json!({"a": 1})).unwrap();
        assert!(c.get("1").unwrap().is_none());
    }

    #[test]
    fn stats_tracks_count_and_bytes() {
        let c = cache();
        c.put("1", json!({"x": "aaaa"})).unwrap();
        c.put("2", json!({"y": "bbbb"})).unwrap();
        let s = c.stats().unwrap();
        assert_eq!(s.count, 2);
        assert!(s.bytes > 0);
    }

    #[test]
    fn put_overwrites_same_key() {
        let c = cache();
        c.put("1", json!({"v": 1})).unwrap();
        c.put("1", json!({"v": 2})).unwrap();
        assert_eq!(c.get("1").unwrap(), Some(json!({"v": 2})));
        assert_eq!(c.stats().unwrap().count, 1);
    }

    #[test]
    fn clear_wipes_entries() {
        let c = cache();
        c.put("1", json!({"x": 1})).unwrap();
        c.put("2", json!({"x": 2})).unwrap();
        c.clear().unwrap();
        assert!(c.get("1").unwrap().is_none());
        let s = c.stats().unwrap();
        assert_eq!(s.count, 0);
        assert_eq!(s.bytes, 0);
    }

    #[test]
    fn link_id_with_path_chars_is_sanitized() {
        let c = cache();
        c.put("..%2Fevil", json!({"ok": true})).unwrap();
        assert_eq!(c.get("..%2Fevil").unwrap(), Some(json!({"ok": true})));
        assert_eq!(c.stats().unwrap().count, 1);
    }
}
