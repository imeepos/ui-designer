# 舵 Rudder

AI 成套 UI 设计工作室：用 gpt-image-2 把「设计系统总板 → 功能页面 → 组件」一次性生成风格一致的成套界面设计图，并提供 CLI + Skill 供 AI 编码代理驱动。

- 产品需求：`docs/PRD.md`
- 架构设计：`docs/ARCHITECTURE.md`
- 视觉规范：`docs/THEME.md`
- 开发计划：`docs/PLAN.md`

## 目录结构

```
ui-designer/
├── src/                     # React 前端（Tauri 桌面应用）
├── src-tauri/               # Tauri 2 壳（薄壳，业务在 core）
├── crates/
│   ├── rudder-core/         # 核心库：项目存储、提示词引擎、gpt-image-2 客户端、导出
│   └── rudder-cli/          # CLI（bin 名：rudder）
├── skill/rudder-design/     # AI 代理 Skill 包
├── scripts/                 # 工程辅助脚本
├── docs/                    # PRD / 架构 / 主题 / 计划
├── e2e/                     # 端到端验收脚本
└── package.json             # pnpm 根
```

## 环境要求

- Node.js ≥ 20.19，pnpm ≥ 10（11 开发验证）
- Rust 1.98+（edition 2021）
- Tauri CLI：使用 devDependency 内置的 `@tauri-apps/cli`，以 `pnpm tauri <cmd>` 调用，无需全局安装

## 快速开始

```bash
pnpm install          # 安装前端依赖
pnpm dev              # 浏览器预览（无 Tauri 壳，页脚显示降级状态）
pnpm tauri dev        # 桌面端开发模式
pnpm build            # 前端类型检查 + 产物构建
pnpm test             # 渲染冒烟测试（vitest）
pnpm lint:text        # 检查 JSX/TS 硬编码文案（文案只允许出现在 src/i18n/）
pnpm tauri --version  # Tauri CLI（npm 预编译版）
cargo check --workspace   # Rust 全 workspace 检查
cargo test --workspace    # Rust 单元测试
```

## 环境变量

复制 `.env.example` 为 `.env` 并填入真实值（`.env` 已被 gitignore，永不入库）：

- `OPENAI_API_KEY` — gpt-image-2 服务凭证（Phase 2 起由 core 从环境读取）
- `OPENAI_BASE_URL` — 可选，默认 `https://api.openai.com`

凭证只从环境变量读取，任何情况下不写入文件、日志或提交记录。

## 当前状态

Phase 1 · 仓库脚手架（见 `docs/PLAN.md`）：

- [x] Monorepo 骨架（前端 + Rust workspace + Tauri 壳）
- [x] 舵主题 CSS 变量（light/dark，docs/THEME.md §2 全部 token）
- [x] 三栏极简骨架 + 四步步条（项目 → 总板 → 页面 → 组件）
- [x] i18n 骨架（zh-CN / en，界面文案零硬编码）
- [x] `ping` command 前端接入（页脚显示 Rust 核心连通状态）
- [ ] Phase 2 · 核心库 + CLI（进行中由后续会话接手）
