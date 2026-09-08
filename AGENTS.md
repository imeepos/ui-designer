# AGENTS.md — 舵 Rudder 仓库协作指令

## 项目
AI 成套 UI 设计工作室「舵 Rudder」。桌面端 Tauri 2 + React + TS + Tailwind 4 + shadcn/ui + i18n(zh-CN/en)；核心 Rust workspace（crates/rudder-core、crates/rudder-cli）；gpt-image-2 驱动。

## 必读（动手前）
1. `docs/PRD.md` — 做什么、不做什么
2. `docs/ARCHITECTURE.md` — 目录、数据模型、CLI 命令集、错误码
3. `docs/THEME.md` — 舵主题视觉规范（实现 UI 时必须比对）
4. `docs/PLAN.md` — 当前 Phase 与验收规则

## 硬性纪律
- API 凭证只从环境变量 `OPENAI_API_KEY` / `OPENAI_BASE_URL` 读；**任何情况下不把密钥写入文件、日志、提交记录**。
- 真实生图调用必须显式 `--yes`（CLI）或桌面端按钮触发；代码与测试默认 dry-run。
- 生图探索用 `quality: low`；`high` 仅用于终版。
- 文案一律 i18n（`src/i18n/*.json`），JSX 中不得出现硬编码中英文案字符串。
- 提交信息用中文祈使句（"新增页面历史存储"），小步提交。
- 每完成一个 Phase，回报：验收命令输出摘要 + 变更文件清单 + 遗留问题。不要自行跨 Phase。
- 遇到 gpt-image-2 参数不确定（如 thinking 字段）：实现为可选透传 + dry-run 可见，不要臆造必填。

## 工程约定
- Rust：edition 2021，`anyhow` 用于 CLI 入口、`thiserror` 用于 core 错误类型；存储写入必须原子（tempfile+rename）。
- 前端：函数组件 + hooks；shadcn/ui 组件按需生成到 `src/components/ui`；主题只用 CSS 变量，不写死 hex（THEME.md 定义的变量除外）。
- 测试与构建命令以 `docs/PLAN.md` 各 Phase 验收为准，全绿才算完成。
