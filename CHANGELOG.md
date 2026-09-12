# 更新日志 · 舵 Rudder

本文件记录「舵 Rudder」各版本的可见变更。格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## 0.2.0 — 2026-09-12

### 客户端全面对接 cms：账户、生图与计费（重大变更）

- 桌面端账户区改接 cms 服务：注册/登录（Cookie 会话存钥匙串 `cms-session`，失效引导重登）、自助 API Key 创建/吊销、点数余额展示与生成后自动刷新；删除 baseUrl/apiKey/model 手动配置与自建会话令牌链（`session-token`/`RUDDER_SESSION_TOKEN`）。
- 生图链路改为前端 openai@7.13.0 SDK 直连 cms 网关（默认 `https://veren.top/api`）：generations 与 edits（多参考图、锚点在前）同链路；响应 b64_json/URL 双形态分流，URL 形由 Rust 侧下载落盘（预签名地址日志打码）。
- 按次计费由 cms points 域承载：余额不足返回 402 给专属文案与引导，成功扣点、失败退还，流水可追溯。
- 生图凭证解析优先级：环境变量 `OPENAI_API_KEY` > 钥匙串 cms API Key > 旧钥匙串 BYO 兜底。
- 删除自建 Go 服务端 `server/`（v0.1.0 后曾短暂随客户端提供账户与计费，本版本起生产入口与客户端全面走 cms，旧部署停用保回滚）。
- CLI：新增 cms 账户客户端（注册/登录/自助建 key/轮换/登出/余额）；新增 `record_generated_image` / `get_cms_api_key` / `update_board_brief` 桌面命令与 `get_generation_config` 只读配置镜像，支撑 SDK 直连的落盘与配置链路。
- 质量防线：prompt 引擎 TS 移植版与 Rust 版 golden 对拍测试（防 CLI/桌面双源漂移）；修复测试环境变量跨模块竞争（共享 ENV_LOCK + 配置注入）。

### 界面打磨

- 向导生成等待态补齐进度条、耗时与取消钮（与重生成抽屉同姿态），取消即时回空态、无后台孤儿作业。
- 导航当前态改缃色指示：向导段标题三态（完成=苍青对勾 / 当前=缃色圆点+加粗 / 未来=muted 轮廓）；左菜单当前项=缃色圆点+加粗（替换原苍青底色块）。
- THEME §5 改写为「向导三段+工作台」新 IA 口径（原四步步条条款退役）；字阶表新增 30px 数据型大数字档。
- 细节一致性：候选编号与锚点标记提至 11px 微字下限；日期按界面语言显示 locale；slug 非法输入不再被静默清空（保留原文并提示格式原因）；设置行内错误带原因 hint 双行展示（与 toast 同源）；内嵌舵轮线稿 favicon 消除控制台 404；清理卡片悬停操作钮死代码（现行落点：候选卡放大镜、左菜单行悬停删除、重生成抽屉）。

### CLI 项目在桌面端可见：共享项目目录登记表（缺陷修复）

- 缺陷：`rudder init` 缺省在当前目录创建项目，桌面端项目列表只扫描 `~/Rudder/projects/`，CLI 建的项目在 GUI 完全不可见；`last_project` 还会存入相对路径，跨工作目录失配。
- `rudder init` 缺省落点改为 `~/Rudder/projects/<uuid>`（ARCHITECTURE §3 共享目录），GUI 直接可见；显式 `--dir` 行为不变，落在扫描根外时自动登记进 `~/Rudder/registry.json`。
- 新共享登记表（rudder-core::registry）：条目规范化绝对路径、按规范形态去重、原子写；读取自愈剔除已删除目录，prune 持久化清理；进程级互斥防并发丢更新。
- CLI 新命令：`rudder project register [--dir]`（存量项目补登记，幂等；扫描根内项目说明跳过）、`rudder project unregister [--dir]`、`rudder project list-roots`（扫描根 + 已登记根，`--json` 供代理消费）。
- 桌面端项目列表合并登记表：规范路径去重、失效项自动剔除、仍按创建时间倒序；`projects_root`/`new_project_dir` 委托 core 共享实现，消除双份逻辑。
- `last_project` 改存规范化绝对路径，跨工作目录解析不再失配。
- `rudder e2e` 自测项目不再登记进用户目录（冒烟零污染）。

### 模板协议：代理可消费的提示词资产（PRD §0 产品边界落地）

- `templates/` 资产包：5 套参数化模板（`board-design-system` / `page-ui-standard` / `page-landing-sections` / `component-sheet-grid` / `brand-identity-lite`），每套 = 参数化骨架 + 逐槽填槽指南（fillGuide：要什么/好例子/常见错误）+ 中英文名与来源标注；`manifest.json` 汇总槽位词表、填槽协议与 attribution（awesome-gpt-image-2，MIT，仅吸收结构模式、案例原文零内嵌）。
- CLI 新命令：`rudder templates list [--json]` / `rudder templates show <id> [--json]`。
- CLI generate 系列新旗标：`--template <id>`（引擎骨架选择，解析顺序 显式 > `project.templateId` > 内置默认）与 `--prompt-file <path>`（代理填好的最终提示词直灌，引擎零改写零注入，锚点仍作 Image 1；`--template` 仅记血缘）。校验失败（文件缺失/空/非 UTF-8、模板不存在）退出码 1 带 hint。
- 提示词引擎升级（rudder-core::prompt/templates）：constraints 注入表（15 条防坑清单结构化为 rule×适用产物，RENDER_RULES 并入，按类型自动追加，dry-run 可见）；页面/组件简报支持 `Labels: a|b|c` 逐字标签槽；`project.json` 新增可选 `templateId` / `negativeHints`（serde 默认空，向后兼容），`rudder project update` 增对应旗标。
- 血缘记录：GenRecord / promptLog / manifest 全部记录 `source`（`engine|agent-file`）与 `templateId`。
- Skill（skill/rudder-design）：SKILL.md 新增「模板工作流」章节（读骨架→自己填槽→`--prompt-file` 出图，附填好的完整示例）；references/cli.md 同步全部新契约。
- 桌面端零改动（GUI 定位人类查看器）；`src-tauri` 仅机械适配核心错误枚举新变体（穷尽 match）与测试结构体字面量新字段，无新 command。

## 0.1.0 — 2026-09-08

首个公开里程碑：AI 成套 UI 设计工作室走通「新建项目 → 设计系统总板 → 功能页面 → 组件设计」四步流程，CLI / 桌面端 / AI 代理 Skill 三入口共享同一 Rust 核心库。

### Phase 1 · 仓库脚手架

- Monorepo 骨架：React 前端 + Rust workspace（`rudder-core` / `rudder-cli`）+ Tauri 2 壳。
- 舵主题 CSS 变量（light/dark 双模式）与三栏极简骨架、四步步条。
- i18n 骨架（zh-CN / en），界面文案零硬编码（`scripts/check-hardcoded-text.mjs` 把关）。
- `ping` command 前后端打通。

### Phase 2 · 核心库 + CLI

- `rudder-core`：项目存储（tempfile+rename 原子写）、三段式提示词引擎、gpt-image-2 客户端（dry-run 计划、multipart 多参考图、429/5xx 指数退避重试）、资产包导出器（manifest.json 溯源 + PROMPTS.md + DESIGN.template.md）。
- `rudder` CLI 全命令：`init` / `project update` / `board generate|pick` / `page add|update|generate|pick` / `component add|update|generate|pick` / `list` / `export` / `e2e` / `config`。
- 输出契约：`--json` 信封 `{ok, data|error{code,message,hint}}`，退出码 0/1/2/3；`--yes` 显式确认才真实调用，默认 dry-run。
- 生成批次自动记录 `seed`（plan / project.json / 候选行 / manifest），任何批次可凭 `--seed` 复现。
- 生成类默认质量为探索档 `low`，`high` 需显式指定。

### Phase 3 · 桌面应用

- 三栏布局完整四步流程：项目新建（尺寸预设 + 自定义校验）、总板候选与锚点、页面/组件生命周期（候选对比 → 转正 → 历史）、导出对话框。
- 生成中骨架屏 + 任务进度事件（约 30s~3min，可取消）；错误统一 `{code,message}` 并走 i18n 文案。
- Tauri commands 薄封装 `rudder-core::ops`，asset protocol 展示项目图片。

### Phase 4 · Skill + 自举

- `skill/rudder-design`：SKILL.md 四步工作流 + 命令参考 + DESIGN.md 契约 + 错误恢复表，供 AI 编码代理自主驱动。
- 自举实测：用本工具 CLI+Skill 为「舵」自身生成设计资产（总板锚点 + 三屏 + 组件图，存 `design-assets/`），并产出 `docs/UI-REVIEW.md` 打磨清单。

### Phase 5 · 打磨 + 发布

- 采纳自举锚点图色系与四档圆角（THEME v2）：primary `#0B3D91` / accent `#D4A017` / 纸白底 / 墨石正文；按钮 4px / 输入 6px / 卡片 8px / 弹窗 12px；dark 模式按「海军蓝提亮、纸白转深、琥珀金保持」派生并通过对比度自审。
- UI-REVIEW 工具缺陷 1-7、10 全部关闭：seed 全程记录、`update` 子命令、生成响应与 board 对称（候选去重 prompt）、默认质量 low、人类模式 stdout 一行摘要、export 默认只带锚点与主稿（`--with-candidates` 显式带探索稿）、未 pick 目标导出警告（stderr + `data.warnings`）。
- 界面整改：详情表单 schema 统一（简报 200 字上限 + 计数、候选数 1-4、质量枚举草稿/标准/高清）并 i18n 定稿；空态插画规范落地（176px、1.5px 线宽、海军蓝单色、舵轮/罗盘/锚三选一）；缩略图徽标字号 ≥11px；悬停浮层按 THEME §5 姿态统一（1.5px 线性图标、hover 主色描边、150ms ease-out、浮层阴影）。
- 发布物：macOS aarch64 `.app` / `.dmg`（`pnpm tauri build`），MIT LICENSE，本更新日志与 README 安装指南。
