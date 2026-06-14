# 黑盒工坊 (xhh-workshop) 项目概述

## 1. 项目简介
黑盒工坊是一个基于逆向工程的小黑盒（XiaoHeiHe）社区非官方第三方客户端，提供了从基础互动到高级AI自动化的一整套解决方案。

## 2. 技术栈

**核心技术：**
- **后端与核心逻辑**：Rust 语言，使用 `tokio`（异步运行时）、`reqwest`（HTTP 客户端）、`axum`（Web 框架）、`clap`（命令行解析）、`utoipa`（API 文档生成）。
- **桌面客户端**：基于 `Tauri 2` 框架，前端采用 `Svelte 5` + `TypeScript` + `Vite` 技术栈。
- **LLM Agent 集成**：支持多模型提供商抽象（OpenAI 兼容、Claude、Ollama）。

**界面与交互：**
- **Web 界面**：桌面应用前端使用 Svelte 5 构建，支持毛玻璃 UI 效果。
- **API 文档**：HTTP 服务内置 Swagger UI。

## 3. 项目结构

项目采用 Rust 工作区（Workspace）组织，核心代码位于 `crates/` 目录下，各模块职责清晰：

- **`xhh-core`**：项目的核心库。封装了与小黑盒服务器通信的所有底层逻辑，包括签名算法（hkey）、扫码登录、各类社区 API 的调用。
- **`xhh-agent`**：LLM Agent 功能模块。负责集成和管理多种 AI 模型提供商，提供 17 个内置工具（如获取信息、发帖等），实现自动化交互与内容生成。
- **`xhh-http`**：HTTP REST 服务。基于 `axum` 构建，将核心功能以 35 个 API 端点的形式暴露，便于其他系统或前端调用。
- **`xhh-cli`**：命令行工具。提供直接的终端交互方式，包括登录、信息查询、发帖、启动服务、运行 Agent 等操作。
- **`xhh-app`**：Tauri 桌面应用。包含一个完整的图形用户界面，其前端代码位于子目录 `frontend/` 中。

## 4. 开发约定

从项目结构和配置文件中可以推断出以下开发规范：

- **分支与发布策略**：遵循 `Gitflow` 工作流。功能开发基于 `develop` 分支，通过 Pull Request 合并。`develop` 的合并会触发 nightly 构建，当稳定版本发布时，将 `develop` 合并到 `main` 分支并打上正式版本标签。
- **持续集成**：通过 `.github/workflows/` 下的配置文件实现自动化构建、测试和发布流程。
- **代码质量**：
    - **格式化**：使用 `cargo fmt` 统一代码风格。
    - **静态分析**：使用 `cargo clippy` 进行代码检查，并将警告视为错误（`-D warnings`）以保证代码质量。
    - **测试**：要求通过 `cargo test --workspace --lib` 执行工作区内的单元测试。
- **模块化设计**：功能被严格划分到独立的 Crate 中（core/agent/http/cli/app），确保高内聚低耦合，便于独立构建和维护。
- **版本与依赖管理**：通过 `Cargo.toml` 和 `Cargo.lock` 管理依赖，使用 `rust-toolchain.toml` 固定 Rust 工具链版本，确保团队开发环境一致。
- **构建产物**：Release 构建配置了 LTO（链接时优化）和 strip（移除调试符号），旨在生成体积小、性能优的独立可执行文件。