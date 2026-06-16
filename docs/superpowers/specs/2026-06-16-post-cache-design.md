# 帖子缓存功能设计

## 目标

基于帖子 ID 缓存文本内容和图片资源，实现：本地 AI 分析加速、帖子详情浏览加速、离线查看。

## 关键决策

| 决策项 | 选择 |
|---|---|
| 缓存位置 | 下沉到 `xhh-core`，新增独立 `cache` 模块 |
| 文本缓存策略 | 按 `link_id` 缓存首屏（`result.link` 主楼），不缓存楼层分页 |
| 图片缓存形式 | 落盘二进制文件，按 URL SHA-1 哈希命名 |
| 容量控制 | 磁盘配额 + LRU 淘汰 + 设置页手动清理 |
| 默认行为 | 默认开启，配额 200MB，可在 config 关闭 |

## 模块结构

```
crates/xhh-core/src/cache/
├── mod.rs      // 模块导出 + CacheManager 门面
├── post.rs     // PostCache：帖子文本缓存
├── image.rs    // ImageCache：图片字节缓存
└── error.rs    // 缓存层错误（可选，复用 core::Error）
```

缓存配置合并进 `config::Config`：

```rust
pub struct CacheConfig {
    pub enabled: bool,            // 默认 true
    pub max_bytes: u64,           // 默认 200 MB
    pub root_dir: PathBuf,        // {config_dir}/xhh/cache
}
```

落盘位置沿用项目惯用法：`{config_dir}/xhh/cache/`，与 `ai_cache.json`、`agent_sessions/` 同级。

## 文本缓存（PostCache）

- key：`link_id`
- value：`PostCacheEntry { link_id, link: Value, fetched_at: i64 }`
- 落盘：`cache/posts/{link_id}.json`
- 索引：`cache/posts/_index.json` — `BTreeMap<link_id, { size_bytes, last_access }>`，常驻内存，写操作同步落盘

读取流程 `get_or_fetch(link_id, query, fetcher)`：
1. `!enabled` → 直接调 fetcher，不缓存
2. 首屏判定（`is_first == Some(1)` 或 `page` 空/为 1）→ 查索引命中则读文件、更新 last_access、返回
3. 未命中 → 调 fetcher，提取 `result.link` 写盘 + 更新索引
4. 写后检查配额，超限触发 LRU 淘汰

非首屏请求直接透传 API，不缓存。

`get_or_fetch` 用闭包注入 fetcher，不依赖 `XhhClient`，可独立单测。

## 图片缓存（ImageCache）

- key：URL 的 SHA-1 hex
- value：原始字节
- 落盘：`cache/images/{sha1前2位}/{sha1剩余}.bin`（分桶避免单目录膨胀）
- 索引：`cache/images/_index.json` — `BTreeMap<hash, { url, size_bytes, last_access }>`

读取流程 `get_or_fetch(url)`：
1. `!enabled` → 直接 `reqwest::get`
2. 命中索引 → 读 `.bin`，更新 last_access
3. 未命中 → `reqwest::get` 拉取 → 写 `.bin` + 索引 → 检查配额淘汰

提供便捷方法 `get_or_fetch_data_uri(url)` 返回 base64 data URI（喂 Ollama 用）。

下载用独立裸 `reqwest::Client`（图片 CDN 不需 hkey 签名）。

## 集成点

| 调用方 | 改动 |
|---|---|
| Tauri `post_detail` 命令 | 改调 `CacheManager::post_detail` |
| Tauri `ai_analyze_stream` | `download_to_data_uri` → `image_cache.get_or_fetch_data_uri` |
| HTTP `/api/post/detail/:id` | 改调缓存版 fetcher |
| Agent `PostDetailTool` | 改调缓存版 fetcher |
| 新增 Tauri 命令 | `cache_get_config` / `cache_save_config` / `cache_stats` / `cache_clear` |

## 前端

Settings.svelte 新增「内容缓存」分区：开关、配额输入、占用统计、清理按钮。

## 测试

- PostCache / ImageCache 单测：命中、未命中写入、LRU 淘汰、clear、enabled=false 透传
- 用临时目录隔离，不污染真实 config 目录
