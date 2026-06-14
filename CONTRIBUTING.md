# 贡献指南

本项目采用简化的 Gitflow 工作流。所有变更通过 Pull Request 合并，CI 根据目标分支自动构建对应的发行版。

## 分支模型

| 分支 | 作用 | 合并方式 | 触发构建 |
|---|---|---|---|
| `main` | 生产分支，始终可发布 | 仅接受来自 `develop` 的 PR | 正式版（按 version 自动打 tag） |
| `develop` | 开发主干，集成最新功能 | 接受 `feature/*`、`fix/*` 的 PR | nightly 预发布版（持续更新） |
| `feature/<name>` | 新功能开发 | 从 `develop` 切出，PR 回 `develop` | 不触发 |
| `fix/<name>` | Bug 修复 | 从 `develop` 切出，PR 回 `develop` | 不触发 |
| `hotfix/<name>` | 生产紧急修复 | 从 `main` 切出，合回 `main` 与 `develop` | 合并 `main` 时触发正式版 |

## 开发流程

1. 基于 `develop` 创建分支：

   ```bash
   git checkout develop
   git pull
   git checkout -b feature/your-feature
   ```

2. 开发并提交，保持提交历史清晰。
3. 推送并创建 PR：`develop ← feature/your-feature`。
4. 通过 Review 后合并，合并即触发 nightly 构建。

## 发版流程

1. 在 `develop` 上确认待发布功能已就绪。
2. 修改 [crates/xhh-app/tauri.conf.json](crates/xhh-app/tauri.conf.json) 的 `version`（如 `0.1.0` → `0.1.1`）。
3. 创建 PR：`develop → main`。
4. 合并后 CI 自动按新版本号打 tag（如 `v0.1.1`）并发布正式版。
5. 若 version 含 `-`（如 `0.2.0-beta.1`），自动标记为预发布。

> 同一 version 重复合并到 `main` 会被跳过（tag 已存在）。发新版必须 bump version。

## 紧急修复（Hotfix）

生产环境发现问题时：

1. 从 `main` 切出：`git checkout -b hotfix/xxx main`
2. 修复后 PR 合回 `main`（同时 bump patch version，触发正式版）
3. 再 PR 或 cherry-pick 同步回 `develop`，保持两条分支一致

## 分支命名

- `feature/<简短描述>` — 新功能
- `fix/<简短描述>` — Bug 修复
- `chore/<简短描述>` — 构建 / 依赖 / 重构
- `docs/<简短描述>` — 文档
- `hotfix/<简短描述>` — 生产紧急修复

## 提交前检查

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
```

## 分支保护建议

为保证流程不被绕过，建议在 GitHub 仓库 Settings → Branches 为 `main` 和 `develop` 开启：

- Require a pull request before merging
- Require approvals
- 禁止直接 push（仅限 PR 合并）
