# 舵 Rudder · awesome-gpt-image-2 CLI 工具链借鉴方案

> 研究对象：[freestylefly/awesome-gpt-image-2](https://github.com/freestylefly/awesome-gpt-image-2) 的 CLI 与配套工具链源码。
> 性质：**纯方案，不改代码**。供后续 Phase 规划参考。
> 研究日期：2026-09-09。源码样本：`bin/install.mjs`、`scripts/generate-style-skill.mjs`、`src/apimartClient.js`、skill `package.json`。

## 一、对方工具链拆解（它到底有什么"CLI"）

对方的仓库本质是内容库，"工具链"由 4 个松耦合组件构成：

| 组件 | 形态 | 干什么 | 关键实现细节 |
|---|---|---|---|
| ① `gpt-image-2-style-library` npm 包 | Node CLI（`bin/install.mjs`，仅 92 行） | 把 skill 包复制进各 AI 编码代理的技能目录 | `install [all\|codex\|claude-code\|agents]` 三目标；目标根目录可被 `CODEX_HOME`/`CLAUDE_HOME`/`AGENTS_HOME` 环境变量覆盖；安装前 `rmSync` 旧目录（覆盖式幂等）；未知动词回退 install；**纯文件复制，零网络、零状态** |
| ② `generate-style-skill.mjs` 构建脚本 | Node 脚本 | **单一数据源生成**：`data/style-library.json` → 渲染出 `references/style-library.md`（Agent 可读索引） | 校验严格：模板 id 唯一、categories/styles/scenes 的 value 唯一、每个模板的 anchor 必须真实存在于 `docs/templates.md`、cover 图必须存在且不得重复；双语 label 走 `zh→en→zh` fallback 链；甚至对生成文本做反模式 lint（禁「不是…而是…」句式）；官网与 Agent skill **共享同一份 style library** |
| ③ `apimartClient.js` 生图客户端 | 浏览器端 JS（配套 `.test.js`） | 试生成 | **异步任务三段式**：`submit → taskId → poll`（2s 间隔、上限 120 次/10 分钟、429 时尊重 `Retry-After`）；错误全部码化（`APIMART_RATE_LIMITED`/`TASK_TIMEOUT`/`INVALID_RESPONSE`/`POLL_ABORTED`）并挂 `retryAfterMs` 结构化字段；个人 key 直连与平台代理双通道；**密钥掩码只露尾 4 位**（`••••••••xxxx`）；pending task 持久化（页面刷新后可恢复轮询）；生成结果按 LRU 留最近 12 条 + `expiresAt` 过期清理；`fetch/wait/now` 全部可注入（可测试性） |
| ④ 配套网站 | Vercel+Supabase | 案例画廊+在线试生成+积分/支付 | 案例卡「复制完整 prompt → 在线试生成 → 跳回 GitHub 源案例」闭环；密钥只存服务端环境变量 |

一句话总结：**它的 CLI 是「分发器」，真正的工程精华在「单一数据源 + 校验生成器」和「异步任务客户端」。**

## 二、逐项借鉴评估

| # | 对方做法 | 对舵的价值 | 结论 |
|---|---|---|---|
| 1 | style-library.json 单一数据源 → 同时供 CLI/Skill/网站三方消费 | 舵的模板/风格知识目前散在 docs 与 skill 文案里，没有机器可读单源 | ⭐ **借鉴（P1，最高价值）** |
| 2 | 生成器内置强校验（id 唯一/anchor 存在/cover 存在/文本反模式 lint） | 模板库会持续扩充，需要编译期/CI 兜底 | ⭐ **借鉴（随 P1）** |
| 3 | skill 安装器（复制到 `~/.codex`、`~/.claude`、`~/.agents`） | 舵已有 `skill/rudder-design`，但无分发通道，用户手工拷贝 | ⭐ **借鉴（P2）** |
| 4 | 异步任务 submit→poll + Retry-After + 错误码化 | 舵现在同步阻塞 30-120s；若日后接 APIMart 类聚合网关或网关转异步，可直接演进 | ◐ **预留（P3/P4，v0.1 只做错误码细化）** |
| 5 | 密钥掩码只露尾 4 位 | 与舵现有纪律完全一致 | ✅ 已具备，无需改 |
| 6 | pending task 刷新恢复、结果 LRU+过期清理 | 为浏览器会话设计；舵图片直接落盘项目目录，无此问题 | ✗ 不借鉴（记 roadmap：仅当异步任务化时再考虑 `--resume`） |
| 7 | npm 包分发 skill | 舵是 Rust 项目，再加一个 npm 发布面得不偿失；CLI 子命令即可达成同样效果 | ✗ 不借鉴（用 P2 替代） |
| 8 | 网站积分/支付/会员 | 超出舵 v0.1「本地优先、无账号」定位（PRD 明确不做协作/账务） | ✗ 不借鉴 |
| 9 | stdout/stderr 纪律、`--json` 同构输出、退出码分级 | 对方客户端的码化错误 + 结构化字段与舵 CLI 输出纪律同思路 | ✅ 已具备，仅细化（见 P3） |

## 三、落地方案（按优先级，均未实施）

### P1 · 模板单一数据源 + 生成器（核心借鉴）

**目标**：让「模板知识」成为机器可读资产，CLI、Skill、（未来）桌面端三方共享一份，杜绝三处文案漂移。

**设计**：
1. 新增 `templates/library.json`（或 `crates/rudder-core/src/templates/library.json` 随包内嵌），schema 参照对方：
   ```jsonc
   {
     "version": 1,
     "templates": [              // 总板/页面/组件/品牌四类参数化模板
       {
         "id": "board-brand-vi",           // 唯一
         "kind": "board|page|component|brand",
         "title": { "zh": "品牌VI总板", "en": "Brand VI Board" },
         "anchor": "board-brand-vi",       // 必须在 docs/模板文档 中存在对应小节（学对方的 anchor 校验）
         "styles": ["minimal", "editorial"],
         "promptTemplate": { "zh": "…{brand_name}…{aspect_ratio}…", "en": "…" },
         "variables": [                    // 槽位声明（学对方的参数槽，换成舵的变量体系）
           { "name": "brand_name", "from": "project.name" },
           { "name": "aspect_ratio", "from": "project.size" },
           { "name": "style_brief", "from": "project.styleBrief" }
         ],
         "constraints": [                  // 学对方的 pitfalls，编译成默认注入约束
           "文字清晰可读，禁止乱码与占位文本",
           "比例信息置于提示词最前"
         ],
         "exampleCases": ["agi2:133", "agi2:132"]   // 文档引用，不内置原文
       }
     ]
   }
   ```
2. 校验器（`rudder-core` 单测或 `build.rs`/`xtask`）：id 唯一、kind 合法、`variables.from` 指向已存在的项目字段、promptTemplate 与 variables 槽位一一对应、constraints 非空。**校验失败即构建失败**（学对方 anchor/cover 校验的严格度）。
3. 消费路径：
   - CLI：`board/page/component generate` 拼prompt 时从库取模板（现有 prompt 模板渲染逻辑升级为读库）；
   - Skill：脚本从同一 JSON 生成 `skill/rudder-design/references/templates.md`（学对方 generate-style-skill 的渲染器，双语 fallback 与反模式 lint 一并学）；
   - `rudder templates [--kind board] [--json]`：列出内置模板（只读，免费路径，进 `rudder list` 家族语义）。
4. 归属：库内首批模板改写自对方 `docs/templates.md`（MIT），文件头注明来源与许可；案例仅以 `agi2:<编号>` 引用编号，不复制原文（规避第三方内容风险，见 `docs/GPT-IMAGE-2-DESIGN-KNOWLEDGE.md` 第一节）。

**验收草案**：`cargo test -p rudder-core templates`（schema+渲染单测全绿）；`rudder templates --json | jq '.data.templates|length'` ≥ 4（每 kind 至少 1 条）。

### P2 · Skill 安装器 + GUI 统一下发（分发器借鉴 × 舵的分发形态）

**目标**：用户装好 GUI 客户端的那一刻，CLI 与 skill 就已就位；GUI 升级时，CLI 与 skill 随包同步升级。三者同源同版、统一打包下发，用户零手工步骤。

**决策：统一下发是默认路径**（本节为既定方向），CLI 仍保留 cargo install / brew 等裸装通道作为非 GUI 用户的备选路径，二者不互斥。

**分发模型（三层）**：

1. **打包层——单源进包**
   - `rudder-cli` 作为 Tauri sidecar（`bundle.externalBin`）随 GUI 打包，按 target triple 出二进制；
   - skill 资产（SKILL.md + references + examples）以 `include_dir!` 内嵌进 GUI 二进制（或放 bundle resources，二选一，倾向内嵌）；
   - GUI、CLI、skill 三个工件来自同一 commit 构建，版本号严格一致——这是「更新即同步」的前提。
2. **首启层——装 GUI 即全装**
   - GUI 启动流程增加「工具链同步」步骤：比对内嵌 VERSION 与已装 VERSION，不一致即静默执行安装（等价 `rudder skill install`）；
   - skill 安装 = 纯文件复制到 `~/.codex/skills`、`~/.claude/skills`、`~/.agents/skills`（学对方 install.mjs，目标根目录可被 `CODEX_HOME` 等覆盖），零网络零密钥；
   - CLI PATH 暴露默认开启：
     - macOS：symlink 直指 `Rudder.app/Contents/MacOS/` 内的 sidecar 二进制——**app 更新后链接天然指向新版**；
     - Windows / Linux：copy 到 `%LOCALAPPDATA%\Rudder\bin` / `~/.local/share/rudder/bin` 并写用户级 PATH（AppImage 挂载点易变，不 symlink）；
     - 首启引导页告知此动作，设置页提供开关与「移除 PATH」（写 PATH 是系统级副作用，可回退但不静默瞒着用户）。
3. **更新层——更 GUI 即全更**
   - Tauri updater 换包后，bundle 内 CLI/skill 已是新版本；
   - 下一次启动「工具链同步」按 VERSION 比对自动重装 skill、重写 copy 型 CLI；symlink 型无需任何动作；
   - 全程无需联网下载组件——统一下发的意义就是把更新面收敛到「更新 GUI 一件事」。

**CLI 侧配套子命令**（服务裸装与诊断场景，GUI 主路径之外）：
```
rudder skill install [all|codex|claude-code|agents]   # 手动安装/重装（与对方语义一致）
rudder skill status                                    # 各目标已装版本 vs 内嵌版本；PATH 暴露状态与悬空检测
```
- 对方没有的能力，舵补上：**VERSION 戳**（防「装的是旧版却不知道」）、`--json` 输出 `{ok,data:{installed:[{target,path,version}],pathStatus}}`、悬空 symlink 检测（app 被移动/改名场景）；
- 幂等可重入；错误走舵的退出码纪律；
- GUI 设置页的「AI 代理集成」区展示 `skill status` 同构数据 + 一键重装（调同一 core 逻辑，不另写实现——CLI 与 GUI 共享 `rudder-core`，安装器做成 core 函数而非 CLI 私有）。

**纪律确认**：打包不引入网络下载；安装动作不触碰钥匙串与任何密钥；更新签名校验由 Tauri updater 承担。

**验收草案**：
- 打包：`tauri build` 产物内含 sidecar CLI 与内嵌 skill（`strings`/资源检查 VERSION 存在且三工件版本一致）；
- 首启：新装 GUI 后断言 `~/.claude/skills/rudder-design/VERSION` 存在且等于 GUI 版本；
- 更新：改版本号重打包覆盖安装，启动后断言 skill 目录版本已跟随；`CODEX_HOME=<tmp>` 隔离环境的 assert_cmd 用例照旧适用于 CLI 子命令路径。

### P3 · 错误码细化：RATE_LIMITED + retry_after_ms（小步，可进 v0.1）

**目标**：把对方客户端最实用的一处错误处理学过来——429 不是普通 API 错，是带等待时长的可重试错误。

**设计**：
- 退出码仍是 2（API 错大类），但 `--json` 的 `error` 增加结构：`{code:"RATE_LIMITED", retryAfterMs: 2000, hint:"稍后重试或降低并发"}`；
- HTTP 响应头 `Retry-After` 存在则透传为 `retryAfterMs`（学对方 `retryAfterMilliseconds()`）；
- Skill 的「常见错误与恢复」章节同步补充 RATE_LIMITED 的处置；
- 桌面端 toast 可据 `retryAfterMs` 显示倒计时（前端改动，随桌面 Phase 走）。

### P4 · 异步任务模式预留（roadmap，不在 v0.1）

若日后网关侧从同步 `images/generations` 演进为异步任务（APIMart 模式：submit→taskId→poll），`rudder-core` 的生成客户端按对方 `pollApimartTask` 的参数面预留 trait 形状：`interval 2s / maxAttempts 120 / maxElapsed 10min / abort 信号 / onProgress 回调`；CLI 侧对应 `--resume <task-id>` 恢复语义。**现在只记录，不实现**（避免为不存在的 API 过度设计，符合 AGENTS.md「不臆造必填」纪律）。

## 四、与现有文档/计划的衔接

- P1、P2 属新能力，建议分别立 Phase，走 `docs/PLAN.md` 的验收流程；P3 可并入最近的生成相关 Phase 顺带做；
- P2 落地时需回写 `docs/ARCHITECTURE.md`：
  - §6 CLI 命令集加 `skill install|status`；
  - §7 桌面应用加「工具链同步」首启步骤（sidecar 打包、VERSION 比对、PATH 暴露开关）与对应 Tauri command（如 `toolchain_sync` / `skill_status`）；
  - §8 Skill 包章书写明安装通道为 GUI 统一下发 + `rudder skill install` 双路径；
- P1 实施时需同步更新 `docs/ARCHITECTURE.md`（§6 CLI 命令集加 `templates`）与 `docs/PRD.md`（若「内置模板库」此前未列入范围）；
- `docs/GPT-IMAGE-2-DESIGN-KNOWLEDGE.md`（已归档的知识提炼）是 P1 模板内容的素材库：第四章组件三元组、第六章总板分区法、第七章防坑清单 → `constraints` 字段。

## 五、明确不做（本方案边界）

- 不发 npm 包、不做网站、不做积分/支付/会员；
- 不内置对方画廊任何案例 prompt 原文（第三方版权不保证商用）；
- 不做 pending-task 恢复与结果 LRU（本地落盘无此需求，异步化时再议）；
- 不改任何代码——本文档仅提案，落地需另立 Phase 并过验收。
