## 变更说明

<!-- 简述本次变更内容与动机 -->

## 变更类型

- [ ] feature（新功能）
- [ ] fix（Bug 修复）
- [ ] chore（构建 / 依赖 / 重构）
- [ ] docs（文档）
- [ ] hotfix（生产紧急修复）

## 合并目标

- [ ] → `develop`（触发 nightly 构建）
- [ ] → `main`（触发正式版，确认已 bump `tauri.conf.json` 的 version）

## 检查清单

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --lib`
