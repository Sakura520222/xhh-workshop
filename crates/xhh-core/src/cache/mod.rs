//! 帖子文本 / 图片资源的本地缓存
//!
//! 按 `link_id` 缓存帖子首屏响应、按图片 URL 的 SHA-1 缓存图片字节，
//! 用于加速帖子详情浏览、本地 AI 分析（Ollama data URI）与离线查看。
//! 落盘位置：`{config_dir}/xhh/cache/{posts,images}/`。
//!
//! CacheManager 无状态：GET 直接按确定性路径查文件，索引仅在写入淘汰与
//! 统计时读取，故多个调用方（Tauri / HTTP / Agent）各自按需构造互不干扰。

pub mod image;
pub mod post;

pub use image::ImageCache;
pub use post::PostCache;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// 默认配额 200MB
pub const DEFAULT_MAX_BYTES: u64 = 200 * 1024 * 1024;

/// 缓存配置（合并进 [`crate::config::Config`]）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct CacheConfig {
    /// 是否启用，默认 true
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 磁盘配额（字节），post + image 共享，默认 200MB
    #[serde(default = "default_max_bytes")]
    pub max_bytes: u64,
    /// 缓存根目录，运行期由 config_dir 派生，不持久化
    #[serde(skip)]
    pub root_dir: PathBuf,
}

fn default_true() -> bool {
    true
}

fn default_max_bytes() -> u64 {
    DEFAULT_MAX_BYTES
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_bytes: DEFAULT_MAX_BYTES,
            root_dir: PathBuf::new(),
        }
    }
}

impl CacheConfig {
    /// 指定根目录，其余用默认值
    pub fn for_root(root_dir: PathBuf) -> Self {
        Self {
            enabled: true,
            max_bytes: DEFAULT_MAX_BYTES,
            root_dir,
        }
    }
}

/// 缓存整体统计
#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheStats {
    pub enabled: bool,
    pub max_bytes: u64,
    pub used_bytes: u64,
    pub posts: CacheAreaStats,
    pub images: CacheAreaStats,
}

/// 单区域统计
#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheAreaStats {
    pub count: usize,
    pub bytes: u64,
}

/// 缓存区域标识（全局淘汰用）
#[derive(Debug, Clone, Copy)]
enum Area {
    Post,
    Image,
}

/// 索引条目：单条缓存的体积与抓取时间
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct IndexEntry {
    pub(crate) size_bytes: u64,
    pub(crate) fetched_at: i64,
}

/// 缓存门面：协调 post / image 两区域与全局配额淘汰
pub struct CacheManager {
    cfg: CacheConfig,
    posts: PostCache,
    images: ImageCache,
}

impl CacheManager {
    /// 显式构造：创建 posts / images 目录
    pub fn new(cfg: CacheConfig) -> Result<Self> {
        std::fs::create_dir_all(cfg.root_dir.join("posts"))?;
        std::fs::create_dir_all(cfg.root_dir.join("images"))?;
        Ok(Self {
            posts: PostCache::new(cfg.clone()),
            images: ImageCache::new(cfg.clone()),
            cfg,
        })
    }

    /// 从默认 config 构造（读 Config.cache，root_dir 派生为 config_dir/cache）。
    /// 失败时降级为禁用实例，绝不抛错给调用方。
    pub fn from_default_config() -> Self {
        let root = crate::config::Config::config_dir().join("cache");
        let mut cfg = match crate::config::Config::load(None) {
            Ok(c) => c.cache.clone(),
            Err(_) => CacheConfig::default(),
        };
        cfg.root_dir = root;
        Self::new(cfg).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "缓存初始化失败，本次禁用缓存");
            Self::disabled()
        })
    }

    /// 禁用实例：enabled=false，所有读写均 noop
    pub fn disabled() -> Self {
        let cfg = CacheConfig {
            enabled: false,
            ..CacheConfig::default()
        };
        Self {
            posts: PostCache::new(cfg.clone()),
            images: ImageCache::new(cfg.clone()),
            cfg,
        }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.cfg
    }

    pub fn posts(&self) -> &PostCache {
        &self.posts
    }

    pub fn images(&self) -> &ImageCache {
        &self.images
    }

    pub fn get_post(&self, link_id: &str) -> Result<Option<serde_json::Value>> {
        self.posts.get(link_id)
    }

    pub fn put_post(&self, link_id: &str, response: serde_json::Value) -> Result<()> {
        self.posts.put(link_id, response)?;
        self.enforce_quota()
    }

    pub fn get_image_data_uri(&self, url: &str) -> Result<Option<String>> {
        self.images.get_data_uri(url)
    }

    pub fn put_image(&self, url: &str, content_type: &str, bytes: &[u8]) -> Result<()> {
        self.images.put(url, content_type, bytes)?;
        self.enforce_quota()
    }

    pub fn stats(&self) -> Result<CacheStats> {
        let posts = self.posts.stats()?;
        let images = self.images.stats()?;
        Ok(CacheStats {
            enabled: self.cfg.enabled,
            max_bytes: self.cfg.max_bytes,
            used_bytes: posts.bytes + images.bytes,
            posts,
            images,
        })
    }

    pub fn clear(&self) -> Result<()> {
        self.posts.clear()?;
        self.images.clear()?;
        Ok(())
    }

    /// 全局配额淘汰：合并 post + image 索引按 fetched_at 升序删最旧，直至总量回到配额内
    fn enforce_quota(&self) -> Result<()> {
        if !self.cfg.enabled || self.cfg.max_bytes == 0 {
            return Ok(());
        }
        let mut entries: Vec<(Area, String, IndexEntry)> = Vec::new();
        for (k, e) in self.posts.list_entries()? {
            entries.push((Area::Post, k, e));
        }
        for (k, e) in self.images.list_entries()? {
            entries.push((Area::Image, k, e));
        }
        let total: u64 = entries.iter().map(|(_, _, e)| e.size_bytes).sum();
        if total <= self.cfg.max_bytes {
            return Ok(());
        }
        entries.sort_by_key(|(_, _, e)| e.fetched_at);
        let mut to_free = total - self.cfg.max_bytes;
        for (area, key, e) in entries {
            if to_free == 0 {
                break;
            }
            match area {
                Area::Post => self.posts.remove(&key)?,
                Area::Image => self.images.remove(&key)?,
            }
            to_free = to_free.saturating_sub(e.size_bytes);
        }
        Ok(())
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_root() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("xhh_cm_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn manager(max_bytes: u64) -> CacheManager {
        let mut cfg = CacheConfig::for_root(tmp_root());
        cfg.max_bytes = max_bytes;
        CacheManager::new(cfg).unwrap()
    }

    fn big_json(n: usize) -> serde_json::Value {
        json!({ "data": "x".repeat(n) })
    }

    #[test]
    fn put_then_get_post_hits() {
        let cm = manager(DEFAULT_MAX_BYTES);
        let v = json!({"result": {"link": {"linkid": "1"}}});
        cm.put_post("123", v.clone()).unwrap();
        assert_eq!(cm.get_post("123").unwrap(), Some(v));
    }

    #[test]
    fn stats_aggregates_both_areas() {
        let cm = manager(DEFAULT_MAX_BYTES);
        cm.put_post("p1", json!({"x": "aaaa"})).unwrap();
        cm.put_image("http://x/a.png", "image/png", b"bbbb")
            .unwrap();
        let s = cm.stats().unwrap();
        assert_eq!(s.posts.count, 1);
        assert_eq!(s.images.count, 1);
        assert_eq!(s.used_bytes, s.posts.bytes + s.images.bytes);
        assert!(s.used_bytes > 0);
    }

    #[test]
    fn clear_wipes_both_areas() {
        let cm = manager(DEFAULT_MAX_BYTES);
        cm.put_post("p1", json!({"x": 1})).unwrap();
        cm.put_image("http://x/a.png", "image/png", b"y").unwrap();
        cm.clear().unwrap();
        let s = cm.stats().unwrap();
        assert_eq!(s.posts.count, 0);
        assert_eq!(s.images.count, 0);
        assert_eq!(s.used_bytes, 0);
    }

    #[test]
    fn quota_enforced_total_under_limit() {
        let cm = manager(200);
        for i in 0..10 {
            cm.put_post(&format!("p{i}"), big_json(100)).unwrap();
        }
        let s = cm.stats().unwrap();
        assert!(s.used_bytes <= 200, "used={} 超 200", s.used_bytes);
    }

    #[test]
    fn global_eviction_removes_oldest_first() {
        let cm = manager(150);
        cm.put_post("old", big_json(100)).unwrap();
        cm.put_post("new1", big_json(100)).unwrap();
        let s = cm.stats().unwrap();
        assert!(s.used_bytes <= 150);
        assert!(cm.get_post("old").unwrap().is_none(), "最旧的 old 应被淘汰");
        assert!(cm.get_post("new1").unwrap().is_some(), "最新的 new1 应保留");
    }

    #[test]
    fn eviction_crosses_area_boundaries() {
        // post 与 image 共享配额，按时间全局淘汰
        let cm = manager(120);
        cm.put_post("p", big_json(100)).unwrap();
        cm.put_image("http://x/i.png", "image/png", &[0u8; 100])
            .unwrap();
        let s = cm.stats().unwrap();
        assert!(s.used_bytes <= 120);
        assert!(
            cm.get_post("p").unwrap().is_none(),
            "最旧的 post 应被淘汰以保留更新的 image"
        );
    }

    #[test]
    fn disabled_manager_noops_without_disk() {
        let cm = CacheManager::disabled();
        cm.put_post("p1", json!({"x": 1})).unwrap();
        assert!(cm.get_post("p1").unwrap().is_none());
        cm.put_image("http://x/a.png", "image/png", b"z").unwrap();
        assert!(cm.get_image_data_uri("http://x/a.png").unwrap().is_none());
    }
}
