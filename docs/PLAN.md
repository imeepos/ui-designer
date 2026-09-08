# 舵 Rudder · 开发计划与验收规则 v1

> 项目负责人驱动，按 Phase 派发给独立会话执行；每个 Phase 有明确验收命令，验收通过才进入下一 Phase。派发时只给需求与验收规则，不带源码。

## Phase 1 · 仓库脚手架（builder-scaffold 会话）
**范围**：按 ARCHITECTURE.md §1 建立全部目录与工程文件；Tauri 2 + React + TS + Tailwind 4 + shadcn/ui 初始化；`crates/rudder-core`、`crates/rudder-cli` 空壳可编译；i18n 骨架（zh-CN/en 两份 JSON + t() 接入）；舵主题 CSS 变量落地（docs/THEME.md）；`src-tauri` 至少一个 `ping` command 前端可调；根 README。
**验收（全绿才算过）**：
- `pnpm install && pnpm build` 退出码 0
- `cargo check --workspace` 退出码 0
- `pnpm tauri --version` 可用（@tauri-apps/cli 为 devDependency，pnpm 暴露本地 bin；cargo install tauri-cli 在本机编译失败已弃用）
- 前端页面渲染出三栏骨架 + 四步步条 + 中英切换可用（人工/截图审）

## Phase 2 · 核心库 + CLI（builder-core 会话）
**范围**：按 ARCHITECTURE.md §3-6 实现 rudder-core（存储/提示词引擎/图像客户端含 dry-run 与重试/导出）与 rudder-cli 全命令；单测 + CLI 集成测试（不花钱路径）；`rudder e2e` 命令。
**验收**：
- `cargo test --workspace` 全过（含尺寸校验、prompt 渲染、原子写、dry-run、mock HTTP 生成流程）
- `cargo clippy --workspace --all-targets -- -D warnings` 0 告警
- `target/debug/rudder init 压测项目 --size web --dir /tmp/rdr-test && .../rudder list --json` 退出码 0 且 JSON 合法
- `rudder board generate`（不带 --yes）输出 dry-run 计划且零网络调用
- `e2e/smoke.sh`（真实 API，low 质量）产出总板 1 张 + 页面 1 张 + 导出包，负责人抽查图片文字正确性

## Phase 3 · 桌面应用四步流程（builder-app 会话）
**范围**：按 PRD §3 与 ARCHITECTURE §7 实现完整 UI：项目新建（名称+尺寸预设+自定义校验）、总板候选（n 张网格+选锚点）、页面管理（增/生成/历史/删除）、组件管理、导出、生成中状态与错误呈现；全部文案走 i18n；遵循 THEME.md。
**验收**：
- `pnpm build` 0 错
- `cargo check --workspace` 0 错
- `pnpm tauri dev` 启动后，真实走完四步流程（真实 API，负责人录屏/截图审查）
- 文案抽查：zh/en 切换后无遗漏硬编码字符串（grep 校验 JSX 内中文字面量为 0，t() 键全存在）
- 界面截图经负责人比对 THEME.md（色板/圆角/步条姿态）

## Phase 4 · Skill + 自举优化（builder-skill 会话）
**范围**：按 ARCHITECTURE §8 写 `skill/rudder-design/`（SKILL.md + examples.md + 参考文档）；然后**用本工具的 CLI+Skill 为「舵」自身生成设计资产**：总板→核心三屏（项目库/画廊/详情）→组件单图，存 `design-assets/`；对照产出撰写 `docs/UI-REVIEW.md`（当前实现 vs 生成图的差距清单）。
**验收**：
- Skill 文档完整覆盖命令序列、`--json` 约定、错误恢复（一个新会话仅凭 Skill 文档能跑通 dry-run 全流程——由负责人出题实测）
- `design-assets/` 含 1 总板 + 3 屏 + 2 组件图，且同风格（负责人目测一致性）
- `docs/UI-REVIEW.md` 列出可执行改动项（≥5 条）并标注优先级

## Phase 5 · 打磨 + 发布（builder-release 会话）
**范围**：按 UI-REVIEW.md 打磨界面；补齐错误路径与加载态；`cargo tauri build` 出包；写 CHANGELOG、安装文档；全量回归。
**验收**：
- `cargo tauri build` 产出 .dmg/.app（aarch64），`du -h` 上报体积
- e2e/smoke.sh 重跑通过；UI-REVIEW 条目全部关闭或注明遗留理由
- README 安装三行命令可复现（负责人在干净目录实测 CLI 安装段）

## 里程碑纪律
- 每 Phase 结束：会话必须回报「验收命令输出摘要 + 变更文件清单」，负责人复核后才归档该会话。
- 任何真实 API 调用只在 `--yes`/桌面端发生；测试默认 dry-run。
- 预算纪律：真实生图一律先 low 探索、high 只用于终版抽查；单张 high ≤ $0.5 预算线。
