# scripts/

工程辅助脚本目录（Phase 1 预留）。

| 脚本 | 用途 | 调用 |
|---|---|---|
| `check-hardcoded-text.mjs` | 扫描 `src/**` 中 i18n 之外的硬编码中文文案（AGENTS.md 纪律） | `pnpm lint:text` |
| `check-i18n-keys.mjs` | 校验 `t()` 键位在 zh-CN/en 双语存在且 parity | `node scripts/check-i18n-keys.mjs` |
| `release/check-version.mjs` | 发布门禁：package.json / Cargo.toml workspace / tauri.conf.json 三处版本一致；`--expect v<semver>` 时对账 tag 名 | `node scripts/release/check-version.mjs` |
| `release/prepare-sidecar.mjs` | 构建 rudder-cli release 二进制并按 target triple 落位 `src-tauri/binaries/`，附 `--version` 与 GUI 清单对账（sidecar 发布约定见 `docs/CLI-BORROW-PLAN.md` P2） | `pnpm sidecar` |

后续 Phase 计划占用：`tauri-icon.mjs`（图标生成辅助）、`e2e` 前置检查等。

## 发布流水线（.github/workflows/release.yml）

`gate`（版本一致 + 前端 build/test/i18n + core/cli clippy/test）→ `build`（macOS arm64 / Linux AppImage+deb / Windows NSIS+MSI 三平台 Tauri 打包，安装包内含 sidecar CLI）→ `release`（仅 `v*` tag：发 GitHub Release，附安装包、按 triple 命名的裸 CLI 二进制与 SHA256SUMS.txt）。

本地打包等价命令：`pnpm sidecar && pnpm tauri build`（`tauri dev`/`build` 的 before 命令已自动包含 sidecar 准备，无需手动）。
