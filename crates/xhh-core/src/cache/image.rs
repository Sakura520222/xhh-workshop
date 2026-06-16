//! 图片字节缓存：按 URL 的 SHA-1 分桶落盘，提供 data URI 供 Ollama 等消费

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::cache::{CacheAreaStats, CacheConfig, IndexEntry};
use crate::error::Result;

pub struct ImageCache {
    cfg: CacheConfig,
}

impl ImageCache {
    pub fn new(cfg: CacheConfig) -> Self {
        Self { cfg }
    }

    fn dir(&self) -> PathBuf {
        self.cfg.root_dir.join("images")
    }

    fn index_path(&self) -> PathBuf {
        self.dir().join("_index.json")
    }

    /// URL 的 SHA-1 hex（40 位）
    pub fn hash(url: &str) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(url.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn data_path(&self, hash: &str) -> PathBuf {
        let (bucket, rest) = hash.split_at(2);
        self.dir().join(bucket).join(format!("{rest}.bin"))
    }

    pub fn enabled(&self) -> bool {
        self.cfg.enabled
    }

    /// 命中返回 data URI（含 content_type 的 base64）；未启用 / 未命中返回 None
    pub fn get_data_uri(&self, url: &str) -> Result<Option<String>> {
        if !self.cfg.enabled {
            return Ok(None);
        }
        let idx = self.read_index()?;
        let Some(entry) = idx.get(url) else {
            return Ok(None);
        };
        let path = self.data_path(&Self::hash(url));
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path)?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
        Ok(Some(format!("data:{};base64,{}", entry.content_type, b64)))
    }

    /// 写入图片字节并更新索引（content_type 用于后续构造 data URI）
    pub fn put(&self, url: &str, content_type: &str, bytes: &[u8]) -> Result<()> {
        if !self.cfg.enabled {
            return Ok(());
        }
        let hash = Self::hash(url);
        let path = self.data_path(&hash);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        let mut idx = self.read_index()?;
        idx.insert(
            url.to_string(),
            ImageIndexEntry {
                size_bytes: bytes.len() as u64,
                fetched_at: super::now_ts(),
                content_type: content_type.to_string(),
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
        Ok(self
            .read_index()?
            .into_iter()
            .map(|(url, e)| {
                (
                    url,
                    IndexEntry {
                        size_bytes: e.size_bytes,
                        fetched_at: e.fetched_at,
                    },
                )
            })
            .collect())
    }

    pub(crate) fn remove(&self, url: &str) -> Result<()> {
        let path = self.data_path(&Self::hash(url));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let mut idx = self.read_index()?;
        if idx.remove(url).is_some() {
            self.write_index(&idx)?;
        }
        Ok(())
    }

    fn read_index(&self) -> Result<std::collections::BTreeMap<String, ImageIndexEntry>> {
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

    fn write_index(&self, idx: &std::collections::BTreeMap<String, ImageIndexEntry>) -> Result<()> {
        let path = self.index_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(idx)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImageIndexEntry {
    size_bytes: u64,
    fetched_at: i64,
    content_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;
    use base64::Engine;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_root() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("xhh_imgcache_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn cache() -> ImageCache {
        ImageCache::new(CacheConfig::for_root(tmp_root()))
    }

    #[test]
    fn hash_is_stable() {
        let h = ImageCache::hash("http://x/a.png");
        assert_eq!(h.len(), 40);
        assert_eq!(h, ImageCache::hash("http://x/a.png"));
        assert_ne!(h, ImageCache::hash("http://x/b.png"));
    }

    #[test]
    fn get_miss_returns_none() {
        let c = cache();
        assert!(c.get_data_uri("http://x/a.png").unwrap().is_none());
    }

    #[test]
    fn put_then_get_returns_data_uri() {
        let c = cache();
        let bytes = b"PNGBYTES";
        c.put("http://x/a.png", "image/png", bytes).unwrap();
        let uri = c.get_data_uri("http://x/a.png").unwrap().unwrap();
        let prefix = "data:image/png;base64,";
        assert!(uri.starts_with(prefix), "uri={uri}");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&uri[prefix.len()..])
            .unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn disabled_skips_cache() {
        let mut cfg = CacheConfig::for_root(tmp_root());
        cfg.enabled = false;
        let c = ImageCache::new(cfg);
        c.put("http://x/a.png", "image/png", b"x").unwrap();
        assert!(c.get_data_uri("http://x/a.png").unwrap().is_none());
    }

    #[test]
    fn stats_tracks_count_and_bytes() {
        let c = cache();
        c.put("http://x/a.png", "image/png", b"aaaa").unwrap();
        c.put("http://x/b.png", "image/png", b"bbbb").unwrap();
        let s = c.stats().unwrap();
        assert_eq!(s.count, 2);
        assert_eq!(s.bytes, 8);
    }

    #[test]
    fn same_url_overwrites() {
        let c = cache();
        c.put("http://x/a.png", "image/png", b"old").unwrap();
        c.put("http://x/a.png", "image/jpeg", b"new").unwrap();
        let uri = c.get_data_uri("http://x/a.png").unwrap().unwrap();
        assert!(uri.starts_with("data:image/jpeg;base64,"), "uri={uri}");
        let s = c.stats().unwrap();
        assert_eq!(s.count, 1);
        assert_eq!(s.bytes, 3);
    }

    #[test]
    fn clear_wipes_entries() {
        let c = cache();
        c.put("http://x/a.png", "image/png", b"data").unwrap();
        c.clear().unwrap();
        assert!(c.get_data_uri("http://x/a.png").unwrap().is_none());
        assert_eq!(c.stats().unwrap().count, 0);
    }
}
