# 舵 Rudder · 架构设计 v0.1

## 1. 总体形态
Monorepo，三个可交付物共享一个 Rust 核心库：

```
ui-designer/
├── src/                     # React 前端（Tauri 桌面应用）
├── src-tauri/               # Tauri 2 壳（薄壳，业务在 core）
├── crates/
│   ├── rudder-core/         # 核心库：项目存储、提示词引擎、gpt-image-2 客户端、导出
│   └── rudder-cli/          # CLI（bin 名：rudder）
├── skill/rudder-design/     # AI 代理 Skill 包（SKILL.md + 参考文档）
├── docs/                    # PRD / 架构 / 主题 / 计划 / 调研
├── e2e/                     # 端到端验收脚本
└── package.json             # pnpm 根
```

## 2. 技术栈（锁定）
- Rust 1.98+ / Tauri 2：桌面壳 + 文件对话框 + 打包
- rudder-core 依赖：`reqwest`(rustls)、`tokio`、`serde/serde_json`、`base64`、`clap`（cli）、`anyhow/thiserror`、`dirs`、`uuid`、`chrono`
- 前端：React 18 + TypeScript + Vite + TailwindCSS + shadcn/ui + react-i18next（zh-CN/en）
- 前后端桥：Tauri command 调 rudder-core；CLI 直接调 rudder-core

## 3. 数据模型与存储
项目根：`~/Rudder/projects/<project-id>/`

```
project.json          # 元数据（见下）
board/                # 设计系统总板候选与选中
  candidates/0001.png …
  anchor.png          # 选定的锚点图（copy）
pages/<slug>/         # 每个功能页面
  candidates/NNNN.png # 多稿候选（pick 后保留备查）
  current.png         # 选中的主稿（copy）
  history/<ts>.png    # 被替换的旧主稿
components/<name>/
  candidates/NNNN.png
  current.png
  history/<ts>.png
refs/                 # 用户提供的布局参考图
```

`project.json` 字段：`{ id, name, canvasSize{w,h,preset}, brandBrief, styleBrief, anchor{candidateId,prompt,seed?,createdAt}, pages[{slug,brief,prompt,seed?,updatedAt}], components[{name,type,brief,prompt?,updatedAt}], promptLog[] }`。全部 serde 序列化；写入用「写临时文件+rename」保证原子性。

## 4. gpt-image-2 客户端（rudder-core::image）
- 端点：`{base}/v1/images/generations` 与 `{base}/v1/images/edits`（multipart：image[] 多参考图）。
- 鉴权：`Authorization: Bearer $OPENAI_API_KEY`；base 取 `$OPENAI_BASE_URL`（默认 `https://api.openai.com`）。**凭证只从环境读，不存储。**
- 响应：`b64_json` 优先；失败重试（429/5xx 指数退避，最多 3 次）；错误要带 HTTP 状态与响应体摘要。
- 生成参数映射：`model=gpt-image-2`、`size`、`quality`（默认 high）、`n`、`thinking`（默认 medium，字段以服务端实际接受为准，做可选透传）、`seed`（可选）。
- **DryRun 模式**：`ImageClient::dry_run` 只返回将要发送的 URL、参数 JSON、multipart 结构，不发请求。CLI 默认 dry-run，`--yes` 才真实调用；桌面端真实调用。
- 超时：生成 300s；测试环境可注入 mock server。

## 5. 提示词引擎（rudder-core::prompt）
成套一致性的落地核心，按调研报告实现三段式：
1. **总板提示词模板**：结构化「用途=UI design system board → 色板(含hex标签) → 字体层级 → 组件样本 → 图标风格 → 间距规则」，用户简报注入。
2. **页面/组件编辑提示词模板**：以「Image 1 是本产品设计系统总板」开头 + 布局简报 + **不变量清单**（严格沿用 Image 1 的配色、字体、圆角、组件样式，不要调整）。
3. 每次生成把最终 prompt 与参数写入 `promptLog`。

## 6. CLI 命令集（rudder-cli）
```
rudder init <name> [--size web|mobile|desktop|WxH] [--dir <path>] [--brief "..."]
rudder board generate [--n 4] [--quality high] [--yes]
rudder board pick <candidate-id>          # 设为锚点
rudder page add <slug> --brief "..."
rudder page generate <slug|--all> [--n 1-4] [--yes]
rudder page pick <slug> <candidate-id>    # 候选→主稿（旧主稿入 history/）
rudder component add <name> --type <t> --brief "..."
rudder component generate <name|--all> [--n 1-4] [--yes]
rudder component pick <name> <candidate-id>
rudder list [pages|components]            # status 概览
rudder export [--out <dir>]               # 资产包：图片+manifest.json+PROMPTS.md+DESIGN.template.md
rudder e2e [--yes]                        # 冒烟：建样例项目→总板→1页→1组件→导出
rudder config get|set <key> <value>       # quality/thinking/n 等默认值，存 ~/Rudder/config.json（不含密钥）
```
🆕 调研吸收（docs/RESEARCH.md）：页面/组件生成同样支持 `--n` 多候选（同 batch 共享风格，superdesign 多稿哲学）；候选命名 `candidates/NNNN.png`，选中即主稿。export 额外产出 `DESIGN.template.md`（项目元数据+全部图片相对路径+待填 token 表骨架），作为 AI 代理撰写 DESIGN.md 的契约底稿。
全局：`--project <path>`（默认 cwd 或最近项目）、`--dry-run`、`--json`（机器可读输出，Skill 用）。
输出纪律：**数据走 stdout、日志走 stderr**；人类模式一行摘要；`--json` 时 stdout 输出 `{ok, data|error{code,message,hint}}`；退出码 0 成功 / 1 参数错 / 2 API 错（429/配额归此类） / 3 项目状态错（如未设锚点就生成页面）。

## 7. 桌面应用（React）
- 三栏极简布局：左「项目列表+新建」/ 中「画廊（board 候选、页面、组件的分区网格）」/ 右「详情与操作（简报表单、生成按钮、历史版本）」。
- 四步流程即导航：顶部步骤条 `项目 → 总板 → 页面 → 组件`，未完成前置步骤时后续置灰。
- Tauri commands：`create_project` / `list_projects` / `get_project` / `generate_board` / `pick_anchor` / `add_page` / `generate_page` / `add_component` / `generate_component` / `export_project` / `delete_artifact`（全部薄封装 core，错误统一 `{code,message}`）。
- 图片展示：asset protocol 指向项目目录；生成中显示骨架屏（生成约 30-120s）。
- 多语言：`src/i18n/{zh-CN,en}.json`，t() 全覆盖；语言切换即时生效。
- **舵主题（Rudder theme）**：见 docs/THEME.md，CSS 变量实现，含 light/dark。

## 8. Skill 包（skill/rudder-design/SKILL.md）
教 AI 代理：① 前置检查（rudder 在 PATH、env 密钥）② 用 `--json`/`--dry-run` 安全探索 ③ 四步流程的命令序列 ④ 生成后如何读图并撰写 `DESIGN.md`（token 表）⑤ 常见错误与恢复（未设锚点、rate limit、尺寸不合法）。附 `examples.md`：从零到导出的完整命令脚本示例。

## 9. 测试与验收
- rudder-core 单测：尺寸校验、prompt 模板渲染、存储原子写、dry-run 计划、mock HTTP（wiremock 或手写 axum mock）。
- CLI 集成测试：`assert_cmd` 跑 init/list/export（不花钱的路径）+ e2e 的 dry-run。
- e2e 真实 API：`e2e/smoke.sh` 调 `rudder e2e --yes`（真实花钱，低质量档），人（即我）抽查图片。
- 前端：`pnpm build` 零错；关键组件渲染冒烟（vitest 可选）。

## 10. 打包
- `pnpm tauri build`（@tauri-apps/cli 为 devDependency；cargo install tauri-cli 本机编译失败已弃用）出 macOS .app/.dmg（aarch64）。
- CLI 单独 `cargo build --release`，随包附 `install.md`。
