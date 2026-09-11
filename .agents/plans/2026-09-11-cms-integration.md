# 2026-09-11 移除自建 server · 对接 cms 整合计划（v2）

## 变更日志
- v1（2026-09-11 12:02）：初版，客户端生图保持 Rust 通道。
- v2（2026-09-11 12:49）：**用户裁决路线 A——桌面端生图/编辑改用前端 openai@7.13.0 SDK 直连 cms**；触发器：用户指令「客户端应该使用 openai@7.13.0（最新）生成、编辑图片」+ 追答「cms 端开 CORS，先制定计划，等 CMS 支持 CORS 后再对接」。代价（用户已接受）：CLI 保留 Rust 通道 → 双客户端长期维护；API key 需注入 webview 内存。

## 前置门 G1（C2-FE 的硬闸）
cms T25 CORS 合并主干并部署生效，探针全部通过才算开闸：
1. 对 `https://veren.top/api/v1/` 的 OPTIONS preflight 返回 `Access-Control-Allow-Origin` 覆盖 Tauri webview origin；
2. `Access-Control-Allow-Headers` 含 `authorization`（SDK 用 Bearer 头，非 cookie credentials——与 T25「跨域 credentials 不支持」边界兼容）；
3. `POST /v1/images/generations`（带 key）从浏览器上下文实际成功一次。
探针由主会话在 T25 部署后亲自执行并记证据。

## 波次（滚动）
- 锁定区：C3（运行中，session-b667f42c）→ 合并后派 C2-RS。
- C2-RS（code-quality，不依赖 CORS）：Rust 侧支撑命令——把 ops::generate「上游调用」与「结果落盘」拆分，新增 Tauri 命令 `record_generated_image`（b64/bytes+metadata→复用既有项目存储/lineage 落盘）；CLI 保留 ImageClient+dry-run 不动。Consumes：C3 合并（commands.rs/lib.rs 同文件避冲突）。Produces：`record_generated_image` 命令名与参数形（喂给 C2-FE）。
- C2-FE（frontend，**硬闸 G1**）：pnpm 精确 pin `openai@7.13.0`；SDK client 工厂（baseURL=`https://veren.top/api/v1`，apiKey 内存注入永不持久化/渲染/日志，maxRetries=3、timeout=300s 对齐既有语义）；canvas 生图与编辑改走 SDK（generations JSON + edits multipart 多参考图锚点在前）；结果 b64→`record_generated_image`；生成后刷新余额（auth_status）；SDK 错误形→用户提示映射。任务书内含 spike：验证 SDK 7.13.0 `images.edit` 是否支持 image[] 多文件，不支持则回报主会话裁决（手写 multipart 或升版本）。与 C2-RS 并行（文件集不相交，契约=命令名+参数形）。
- C4（删 server/ + 文档）：C2-FE 合并后。
- T24 部署（cms→138+nginx 切换+rudder-server 下线）：等 cms T24 扣点 + T25 CORS 合并后派发，G1 探针随部署一起做。

## cms 侧状态快照（2026-09-11 12:49）
- T21 generations：已并主干。T23 edits：已并主干（8d66fa4 等）。T24 扣点：feat/t24-image-billing 在途。T25 CORS：已立项（d988dbe），实现未落地。t19-alipay：他人并行任务。

## 波次（v1 记录，归档备查）
- 锁定区（本波）：cms T22/T23 已由用户派发外部会话执行；uisd C1 已派发。
- 待细化区：OAuth 登录对接、旧 rudder Postgres 下线（默认不迁移用户/积分，冷启动，保留回滚窗口）、管理面沿用 cms。

## 裁决记录（用户已拍板）
1. 方案：cms 侧图片代理已有 T21（feat/t21-images，待合并），本期补齐缺口后客户端全面对接；ui-designer 删除 `server/`。
2. 计费：本期接线扣点（cms points 域），成功扣、失败退、不负数。
3. 部署：138 服务器，veren.top 沿用，客户端默认 `https://veren.top/api` 不变（nginx 改映射到 cms）。

## 侦查结论（契约事实）
- cms T21 提供：`POST /v1/images/generations`（OpenAI 形、非流式、Bearer cms API key + llm:invoke、`image.openai.*` 上游配置热切、usage 记账、/v1/models 并集）。**明确范围外：edits/variations**。
- cms 缺口 1：无 `/v1/images/edits`（multipart 多参考图）；ui-designer `ops.rs:762 run_edits` 真实在用 → 必补（T22）。
- cms 缺口 2：网关不扣点 → 需接线（T23）。
- cms 已有：注册/登录（信封 + Cookie 会话）、默认角色带 apikey:manage、`POST /v1/apikeys` 一次性明文、`GET /v1/points/me` 余额+流水、admin configs 热切。
- ui-designer 消费面：`server_auth.rs`、`image.rs`、`config.rs(server_url)`、src-tauri `auth_*` 命令、`src/lib/api/auth.ts` + settings-dialog 账号区、相关测试、ARCHITECTURE §11、server/ 目录本体。

## 依赖图
```
[cms]  合并 t21-images → T22 edits → T23 扣点 → T24 部署138+nginx切换
[uisd] C1+C2 rust-core(cms客户端改造) → C3 tauri+frontend → C4b 删server目录收尾
C4a 文档改写可与 C3 并行；T24 与 C1..C3 可并行（以契约为准）
```

## 波次
- 锁定区（本波）：cms T22/T23 已由用户派发外部会话执行；uisd C1 已派发。
- 待细化区：OAuth 登录对接、旧 rudder Postgres 下线（默认不迁移用户/积分，冷启动，保留回滚窗口）、管理面沿用 cms。

## 进度账本
- 2026-09-11 11:57 用户确认 cms T22/T23 已派发（外部会话，完成后人工通知）。
- 2026-09-11 12:02 派发 C1（rust-core cms 账户客户端）→ 会话 session-18c6a44f-1a87-49d1-8de8-8d73f1d2f678，分支 feat/cms-auth-client，状态：运行中。
- 2026-09-11 12:33 验活：C1 存活，分支上 3 笔提交（钥匙串账户/账户客户端/凭证链切换），继续等待回报。
- 2026-09-11 12:33 cms 侧侦察：T21(generations) 已合并主干并清枝；edits 实际任务号为 **T23**（feat/t23-images-edits 在途，2 笔提交）；计费扣点尚未开分支（须等 T23 合并后串行）；t18-recharge/t19-alipay 为他人并行任务，不干扰本计划。编号错位记录：本账本后续以 cms 实际号（T23=edits）为准。
- 2026-09-11 12:38 **C1 验收通过并合并主干**：主会话独立复核（非子会话口径）——cargo test -p rudder-core 真实输出 175 passed/0 failed；build --workspace 退出码 0；clippy --workspace 退出码 0；diff 仅 rudder-core 5 文件；无明文打印，钥匙串账户名/脱敏正确；cms 仓库仅其自身笔记文件脏（非 C1 所为）。ff-only 合并 910df62→a2b476c，分支已删。main 领先 origin/main 4 笔提交，**推送待用户点头**。Ruling: 接受 C1 增量辅助项（login_in/account_status_in/is_session_expired/CmsPointsEntry）——密闭测试需要 SecretStore 注入，签名主体零偏移——错了的代价：无。
- 2026-09-11 12:39 派发 C3（frontend：tauri 命令+账号区+旧链下线）→ 会话 session-b667f42c-9846-4d13-ac12-bcbd5feb848d，分支 feat/cms-account-ui，状态：运行中。
- 2026-09-11 12:50 计划升 v2：用户裁决生图链路走前端 openai@7.13.0 直连（cms 开 CORS 前置）。Ruling: A 路线 + G1 CORS 硬闸——用户明示指令；已知代价（双客户端/key 入 webview 内存）已由用户接受——错了的代价：若 CORS 联调受挫，回退 B（Rust 通道）的成本是重建 C2-FE，已记待细化区。C3 不受影响继续跑；C2-RS 等 C3 合并即派；C2-FE 等 G1。

## C3 任务书草稿（C1 验收通过后派发；frontend 型）
- 目标：src-tauri `commands.rs` 的 `auth_login/auth_register/auth_me` 改接 `cms_auth`（按 C1 Produces 签名），DTO 同步改造；前端 `src/lib/api/auth.ts` 类型与调用更新（email+password+name 表单、余额=points balance）；`settings-dialog.tsx` 账号区三态（未登录/登录中/已登录含余额、Cookie 失效引导重登）；i18n zh/en 文案同步（credits→点数/余额语义）；删除 `server_auth.rs`、lib.rs 导出、旧 `session-token` 账户清理与 `RUDDER_SESSION_TOKEN` 残留；`auth.test.ts`、`App.test.tsx`、src-tauri 测试更新。
- 验收：`pnpm test`（vitest）全绿；`pnpm build` 通过；`cargo build --workspace` + `cargo test --workspace` 全绿；桌面端跑起来截图证据链：登录表单校验错误态 → 登录成功显示余额 → 登出/失效态；console 无错误无明文泄漏；grep 验证 `server_auth|RUDDER_SESSION_TOKEN|credits` 前端零残留（CHANGELOG 除外）。
- 边界：不动 image.rs 请求构造、不动 e2e 基建、不删 server/ 目录（C4 做）。

## 任务提示词存档
- cms 任务提示词：见下方正文（已交付用户派发外部会话执行）。

---

## cms 任务提示词（可直接复制）

# cms 配合开发：图片网关 edits 端点 + 生图按次扣点（建议任务号 T22/T23）

## 背景
舵 Rudder 桌面端/CLI 将移除自建服务端，全面对接 cms。图片网关 generations 已就绪（T21，可能仍在待合并分支 feat/t21-images）；当前还差两块能力：参考图编辑端点（edits）、生图按次扣积分。

## 目标
1. 新增 OpenAI 协议图片编辑端点 `POST /v1/images/edits`：multipart 多参考图（`image[]` 可多张，锚点图在前），行为与 generations 完全对齐——同一鉴权判定与拒绝形状、同一模型白名单/映射、stream=true 同样拒绝 400、同一错误形状、同样记入 usage。
2. 生图计费接线积分域：generations 与 edits 都按生成张数扣积分（单价管理面可配、默认 10、热切生效）；生成前预检/预扣，上游失败全额退还；任何路径不得把余额扣成负数；每笔扣/退在积分流水中可追溯（reason 必填），并与 usage 记录可对应。

## 起止边界
- 起点：基于最新主干开发。若 T21 未合并先协调合并或基于其之上开工，禁止与在途分支（t21-images、t18-recharge）产生覆盖性冲突；开工前按仓库 AGENTS.md 核对任务号与迁移号占用，需求登记进 TODO.md。
- 止点：两项功能验收全绿并合并回主分支，确认合并与源码安全后完成清理。
- 明确不做：图片流式；variations 端点；多上游负载均衡；size/quality 枚举校验；旧 rudder 用户与积分数据迁移；特定角色免计费特例；注册开关；短信/OAuth 等一切域外改动。

## 验收标准（全部需真实执行证据，禁止以表面成功信号替代）
1. `make conformance` 新增 edits 场景组：SDK 直连成功形（多参考图、b64 可解码）、stream 拒绝 400 形、未知模型 404 形、上游 4xx/5xx 原样透传、usage 统计出现 edits 行。
2. multipart 体积上限覆盖桌面端多参考图场景（按旧系统口径 ≥30MB）；缺图/空图请求被拒且返回 OpenAI 错误形。
3. 扣点红绿双向证明：先写失败测试再实现；覆盖「成功扣 n×单价」「上游失败全额退还」「余额不足拒绝生成（不出图不扣点）」「并发扣减无负数漏洞（-race）」「退还与扣款流水一一对应」。
4. 管理面修改单价即时生效；`GET /v1/points/me` 流水能看到每笔扣/退及 reason。
5. edits 与 generations 鉴权行为一致（同一 API Key 权限判定，拒绝形状一致）。
6. 仓库门禁四件套全绿：lint + test + build + commit（遵循 cms/AGENTS.md）。

## 执行要求
- 使用独立 worktree + 短命分支，禁止直接在主分支改动；edits 与扣点分两个任务分支（T22、T23），各自分批小步修改、测试、commit，每笔提交可独立回滚。
- 先完成并合并 T22，再基于最新主干做 T23（两者会触碰相邻文件，串行避冲突）。
- 完成后按收尾四步合并回主分支并逐步机械核验；确认合并与推送成功、源码安全后才清理旧分支与 worktree；合并失败严禁删树，回 worktree 反向同步后重试。
- 无需逐步请示，自主推进；遇到阻塞如实报告原因与已尝试的措施，不得隐瞒或绕过。
- 完成后回报：验收命令输出摘要、变更文件清单、任务号与分支名、遗留问题。
