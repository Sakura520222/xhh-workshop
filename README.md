# 黑盒工坊

小黑盒社区第三方客户端，通过逆向 Web 端 API 实现扫码登录、发帖、评论、点赞、收藏、LLM Agent 等功能。

> **免责声明**：本项目为第三方非官方客户端，仅供学习与研究用途，与小黑盒（heybox / xiaoheihe）官方无关。使用本项目带来的任何账号风险（封禁、限制等）由使用者自行承担。请勿用于商业用途或大规模自动化。

## 功能

- **扫码登录** — 终端 QR 码 / 桌面应用扫码
- **帖子** — 浏览 feeds、查看详情、发帖/编辑/删帖
- **评论** — 评论 CRUD，支持楼中楼
- **互动** — 点赞（帖子+评论）、收藏
- **搜索** — 帖子/用户/游戏/话题/社区
- **COS 上传** — 图片上传到小黑盒图床
- **LLM Agent** — 多 Provider（OpenAI 兼容 / Claude / Ollama），17 个内置工具，自动发帖
- **HTTP 服务** — 35 个 REST API 端点 + Swagger UI
- **桌面客户端** — Tauri 2 + Svelte 5，毛玻璃 UI

## 技术栈

- **核心 / CLI / HTTP**：Rust（tokio · reqwest · axum · clap · utoipa）
- **桌面应用**：Tauri 2 + Svelte 5 + TypeScript + Vite
- **LLM Agent**：多 Provider 抽象（OpenAI 兼容 / Claude / Ollama）

## 项目结构

```
crates/
├── xhh-core/     # 核心库：hkey 签名、扫码登录、API 封装
├── xhh-agent/    # LLM Agent 模块
├── xhh-http/     # HTTP REST 服务（axum + utoipa）
├── xhh-cli/      # 命令行工具
└── xhh-app/      # Tauri 桌面应用
    └── frontend/ # Svelte 5 前端
```

## 快速开始

### 环境要求

- Rust 1.81+
- Node.js 18+（桌面应用前端）
- MSVC 构建工具（Windows）

### 构建

```bash
# 构建 CLI
cargo build -p xhh-cli --release

# 构建桌面应用
cargo build -p xhh-app --release

# 构建 HTTP 服务
cargo build -p xhh-http --release
```

### 测试

```bash
cargo test --workspace --lib
```

### Lint

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## 开发流程

本项目采用 Gitflow 工作流：从 `develop` 切分支开发，PR 合回 `develop` 触发 nightly 构建；`develop` 合并到 `main` 触发正式版发布。详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 使用

### CLI

```bash
# 扫码登录
xhh login

# 查看账号信息
xhh info

# 拉取 feeds
xhh feeds

# 发帖
xhh post --title "标题" --content "正文"

# 启动 HTTP 服务
xhh serve --bind 127.0.0.1:9876

# Agent 交互模式
xhh agent
```

### 桌面应用

```bash
cd crates/xhh-app
cargo tauri dev
```

## 构建

### CLI / HTTP 服务

```bash
# 编译优化后的二进制（位于 target/release/）
cargo build -p xhh-cli --release
cargo build -p xhh-http --release
```

Release profile 已配置 LTO + strip，输出为独立可执行文件，无需运行时依赖。

### 桌面应用（NSIS 安装包）

```bash
cd crates/xhh-app
cargo tauri build
```

输出位于 `target/release/bundle/nsis/`，生成 `.exe` 安装程序。

## 配置

| 文件 | 路径 | 用途 |
|---|---|---|
| 登录凭据 | `%APPDATA%\xhh\config.json` | pkey / heybox_id |
| Agent 配置 | `%APPDATA%\xhh\agent.json` | Provider / 配额等 |
| 配额计数器 | `%APPDATA%\xhh\agent_counters.json` | 每日配额 |

## License

[GPL-3.0](LICENSE)
