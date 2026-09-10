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
- rudder-core 依赖：`reqwest`(rustls)、`tokio`、`serde/serde_json`、`base64`、`clap`（cli）、`anyhow/thiserror`、`dirs`、`uuid`、`chrono`、`keyring`（OS 钥匙串）
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

**共享项目目录（CLI ↔ 桌面端单一事实源）**：桌面端项目列表 = 扫描 `~/Rudder/projects/` **并入** `~/Rudder/registry.json` 登记的外部项目根。`rudder init` 缺省创建于扫描根内（`~/Rudder/projects/<uuid>`），显式 `--dir` 落在扫描根外时自动登记；`rudder project register/unregister` 可补登记/注销存量项目。登记项为规范化绝对路径、按规范形态去重、读取时自愈剔除已删除目录（prune 持久化清理），写盘原子（tempfile+rename）。`config.json` 的 `last_project` 同样存规范化绝对路径。

## 4. gpt-image-2 客户端（rudder-core::image）
- 端点：`{base}/v1/images/generations` 与 `{base}/v1/images/edits`（multipart：image[] 多参考图）。
- 鉴权：`Authorization: Bearer <api key>`。**凭证解析优先级：环境变量 `OPENAI_API_KEY` → OS 钥匙串（`rudder-core::config::credential`，service `rudder` / account `openai-api-key`）→ 未配置**；env 覆盖钥匙串，保证 CI/代理行为不变（`RUDDER_KEYCHAIN=0` 可整体关闭钥匙串，用于 CI 与隔离测试）。base 取 `$OPENAI_BASE_URL` → `config.json` 的 `base_url` → 默认 `https://api.openai.com`。**密钥唯一持久化位置是钥匙串；禁止明文落盘/日志/stdout；界面只显示尾 4 位。**
- 连通性自检：`GET {base}/v1/models`（免费，不生图），校验 200 且模型列表含 `gpt-image-2`（`config::credential::test_connection`，CLI `rudder config test` / 桌面端「测试连接」共用）。
- 响应：`b64_json` 优先；失败重试（429/5xx 指数退避，最多 3 次）；错误要带 HTTP 状态与响应体摘要。
- 生成参数映射：`model=gpt-image-2`、`size`、`quality`（默认 low，探索档；high 需显式）、`n`、`thinking`（默认 medium，字段以服务端实际接受为准，做可选透传）、`seed`（省略时自动生成并随 plan/lineage/候选/manifest 记录，保证可复现）。
- **DryRun 模式**：`ImageClient::dry_run` 只返回将要发送的 URL、参数 JSON、multipart 结构，不发请求。CLI 默认 dry-run，`--yes` 才真实调用；桌面端真实调用。
- 超时：生成 300s；测试环境可注入 mock server。

## 5. 提示词引擎（rudder-core::prompt）
成套一致性的落地核心，按调研报告实现三段式：
1. **总板提示词模板**：结构化「用途=UI design system board → 色板(含hex标签) → 字体层级 → 组件样本 → 图标风格 → 间距规则」，用户简报注入。
2. **页面/组件编辑提示词模板**：以「Image 1 是本产品设计系统总板」开头 + 布局简报 + **不变量清单**（严格沿用 Image 1 的配色、字体、圆角、组件样式，不要调整）。
3. 每次生成把最终 prompt 与参数写入 `promptLog`。

**模板协议（rudder-core::templates，PRD §0）**：`templates/` 资产包内嵌进二进制
（include_str），每套模板 = 参数化骨架（skeleton，槽位 `{project.name}`/`{page.brief}` 等）
+ 逐槽填槽指南（fillGuide：要什么/好例子/常见错误），供外部代理读取后**用自己的 LLM 填槽**。
两条拼装路径：引擎路径（显式 `--template` > `project.templateId` > 内置默认；骨架填充后自动
追加按产物类型分表的 **constraints 注入表**——15 条防坑清单结构化为 rule×applies，含
project.negativeHints 扩展与 `Labels: a|b|c` 逐字标签约束）；代理路径（`--prompt-file`，
文件内容即最终提示词，零改写零注入，锚点仍作 Image 1，promptLog/manifest 记
`source: "agent-file"` + 可选 `templateId` 血缘）。

## 6. CLI 命令集（rudder-cli）
```
rudder init <name> [--size web|mobile|desktop|WxH] [--dir <path>] [--brief "..."]
                            # 缺省落点 ~/Rudder/projects/<uuid>（桌面目录可见）；--dir 在扫描根外时自动登记
rudder project update [--name <n>] [--brand-brief <s>] [--style-brief <s>] [--template <id>] [--clear-template] [--negative-hint <s>]... [--clear-negative-hints]
rudder project register [--dir <path>]     # 把扫描根外项目登记进桌面目录（缺省最近项目；幂等）
rudder project unregister [--dir <path>]   # 从桌面目录注销（缺省最近项目）
rudder project list-roots                  # 输出扫描根 + 已登记项目根（--json 供代理消费）
rudder templates list [--json]               # 模板清单 + 槽位词表 + attribution
rudder templates show <id> [--json]          # 骨架 + fillGuide（代理读后自行填槽）
rudder board generate [--n 4] [--quality low] [--seed <int>] [--yes] [--template <id>] [--prompt-file <path>]
rudder board pick <candidate-id>          # 设为锚点
rudder page add <slug> --brief "..."
rudder page update <slug> --brief "..."   # 改布局简报（免手编 project.json）
rudder page generate <slug|--all> [--n 1-4] [--yes] [--template <id>] [--prompt-file <path>]
rudder page pick <slug> <candidate-id>    # 候选→主稿（旧主稿入 history/）
rudder component add <name> --type <t> --brief "..."
rudder component update <name> [--type <t>] [--brief "..."]
rudder component generate <name|--all> [--n 1-4] [--yes] [--template <id>] [--prompt-file <path>]
rudder component pick <name> <candidate-id>
rudder list [pages|components]            # status 概览
rudder export [--out <dir>] [--with-candidates]  # 默认只带 anchor/主稿；候选需显式带上
rudder e2e [--yes]                        # 冒烟：建样例项目→总板→1页→1组件→导出
rudder config get|set <key> <value>       # quality/thinking/n/base_url（非敏感）存 ~/Rudder/config.json（不含密钥）
rudder config set api-key                 # 密钥从 stdin 读入（绝不进 argv/history），写入 OS 钥匙串
rudder config clear api-key               # 清除钥匙串中的密钥（幂等）
rudder config test                        # 输出 base、key 来源 env/keychain/none、可用模型数（免费 GET /v1/models，--json 同构）
```
🆕 调研吸收（docs/RESEARCH.md）：页面/组件生成同样支持 `--n` 多候选（同 batch 共享风格，superdesign 多稿哲学）；候选命名 `candidates/NNNN.png`，选中即主稿。export 额外产出 `DESIGN.template.md`（项目元数据+全部图片相对路径+待填 token 表骨架），作为 AI 代理撰写 DESIGN.md 的契约底稿。已生成未 pick 的目标在 export 时以 stderr `warning:` 与 `--json data.warnings` 提示（不阻断）。
全局：`--project <path>`（默认 cwd 或最近项目）、`--dry-run`、`--json`（机器可读输出，Skill 用）。
输出纪律：**人类模式一行摘要走 stdout，日志/警告/错误走 stderr**；`--json` 时 stdout 输出 `{ok, data|error{code,message,hint}}`；生成类响应统一 `{dryRun, kind, target, plan, candidates:[{id,file,seed,size,quality}]}`（prompt 只在 plan.params.prompt 出现一次；`--all` 时包一层 `results[]`）；退出码 0 成功 / 1 参数错 / 2 API 错（429/配额归此类） / 3 项目状态错（如未设锚点就生成页面）。

## 7. 桌面应用（React）
- 三栏极简布局：左「项目列表+新建」/ 中「画廊（board 候选、页面、组件的分区网格）」/ 右「详情与操作（简报表单、生成按钮、历史版本）」。
- 四步流程即导航：顶部步骤条 `项目 → 总板 → 页面 → 组件`，未完成前置步骤时后续置灰。
- Tauri commands：`create_project` / `list_projects` / `get_project` / `generate_board` / `pick_anchor` / `add_page` / `generate_page` / `add_component` / `generate_component` / `export_project` / `delete_artifact`（全部薄封装 core，错误统一 `{code,message}`）；凭证设置：`get_credential_status` / `save_api_key` / `clear_api_key` / `save_base_url` / `test_connection`（钥匙串读写；状态只回来源标签与尾 4 位，绝不全量回传密钥；无凭证错误码 `NO_CREDENTIALS`，前端 toast 引导打开设置）。
- 图片展示：asset protocol 指向项目目录；生成中显示骨架屏（生成约 30-120s）。
- 多语言：`src/i18n/{zh-CN,en}.json`，t() 全覆盖；语言切换即时生效。
- **舵主题（Rudder theme）**：见 docs/THEME.md，CSS 变量实现，含 light/dark。

## 8. Skill 包（skill/rudder-design/SKILL.md）
教 AI 代理：① 前置检查（rudder 在 PATH、`rudder config test` 确认凭证来源 env/keychain）② 用 `--json`/`--dry-run` 安全探索 ③ 四步流程的命令序列 ④ 生成后如何读图并撰写 `DESIGN.md`（token 表）⑤ 常见错误与恢复（未设锚点、rate limit、尺寸不合法）。附 `examples.md`：从零到导出的完整命令脚本示例。

## 9. 测试与验收
- rudder-core 单测：尺寸校验、prompt 模板渲染、存储原子写、dry-run 计划、mock HTTP（wiremock 或手写 axum mock）。
- CLI 集成测试：`assert_cmd` 跑 init/list/export（不花钱的路径）+ e2e 的 dry-run。
- e2e 真实 API：`e2e/smoke.sh` 调 `rudder e2e --yes`（真实花钱，低质量档），人（即我）抽查图片。
- 前端：`pnpm build` 零错；关键组件渲染冒烟（vitest 可选）。

## 10. 打包
- `pnpm tauri build`（@tauri-apps/cli 为 devDependency；cargo install tauri-cli 本机编译失败已弃用）出 macOS .app/.dmg（aarch64）。
- CLI 单独 `cargo build --release`，随包附 `install.md`。

## 11. 服务端（server/ · rudder-server）
Go 后端，让客户端**0 配置**：不再自备 baseUrl/apiKey/model，注册登录即用，按次计费。

- 技术栈：Go 1.22+ 标准库路由 + `pgx`（Postgres）+ `golang-jwt` + `bcrypt`；无 Web 框架。
- 部署形态：nginx（veren.top，TLS）`location /api/v1/` → `127.0.0.1:8799` rudder-server → 上游 gpt-image-2 服务；Postgres 复用服务器 docker 实例（库 `rudder`）；systemd 单元 `rudder-server`（env 文件 `/etc/rudder-server.env`）。
- API（前缀 `/api/v1`）：`auth/register|login|me|change-password`；`images/generations|edits`（OpenAI 兼容，`model` 服务端注入，Bearer=用户 JWT）；`models`（免费探测）；`usage`（余额+流水）；admin：`users` 列表/patch/充值/重置密码、`settings`（积分单价/注册开关/上游地址与密钥——只回尾 4 位）、`stats`、`generations`、`console`（内置 Web 管理台）。
- 计费：每图 `credits_per_image` 积分（默认 10，admin 可配）；预扣（事务+行锁）→ 上游失败全额退款；admin 角色免计费；注册赠送可配。
- 凭证纪律：上游密钥只存 Postgres `settings` 表；API/控制台仅显示尾 4 位；绝不写日志。
- 客户端映射：桌面端/CLI `base` 默认 `https://veren.top/api`，Bearer 用**会话令牌**（env `RUDDER_SESSION_TOKEN` → 钥匙串 account `session-token`）；旧 BYO 链（`OPENAI_API_KEY`/钥匙串 `openai-api-key`）保留为 fallback。
