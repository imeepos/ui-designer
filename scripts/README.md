# scripts/

工程辅助脚本目录（Phase 1 预留）。

| 脚本 | 用途 | 调用 |
|---|---|---|
| `check-hardcoded-text.mjs` | 扫描 `src/**` 中 i18n 之外的硬编码中文文案（AGENTS.md 纪律） | `pnpm lint:text` |

后续 Phase 计划占用：`tauri-icon.mjs`（图标生成辅助）、`e2e` 前置检查等。
