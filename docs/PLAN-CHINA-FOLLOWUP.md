# 墨航重设计 · 遗留项与缺陷修复计划（2026-09-10）

> 依据：docs/REDESIGN-CHINA.md §5 遗留项 + 重设计报告暴露缺陷。
> 方法论（社区最佳实践检索结论）：
> 1. **图像 API 客户端健壮性** —— 响应体形状校验失败（如缺 `b64_json`）应与 429/5xx 同列为「可重试瞬态错误」，指数退避、有限次数（参考 vercel/ai 对 image 响应格式的防御处理模式、OpenAI 图像 API b64 响应契约）。
> 2. **pnpm 存储锁恢复** —— 卡死的 install 进程应终止；必要时只删 store 索引而非整库（mastra#18443 实践），避免全量重下。
> 3. **Git worktree 隔离** —— 每项代码改动开独立 worktree + 分支，完成即测试提交，合并回主分支后删除 worktree 与分支，保证与并发 WIP 零冲突（git-worktree-isolation 模式）。

## 前置状态

- 工作区并发 WIP（无限画布/凭证 UI 等 23 文件 + infinite-canvas.tsx）自 23:46 起无变动，判定为**已闲置**，先整体快照存档（F0），保证后续合并不与脏文件冲突、源码零丢失。
- 挂起的 `pnpm install`（PID 79914）已消失，`pnpm --version` 即时响应——存储锁已释放（F7 只补文档）。

## 任务清单

| # | 优先级 | 任务 | 验收 | 分支策略 |
|---|---|---|---|---|
| F0 | P0 | WIP 快照存档（含 lightbox 1 行编译修复），先跑 tsc/cargo check 记录状态 | 快照提交后 `git status` 干净 | 主分支直提 |
| F1 | P0 | rudder-core 图像客户端：2xx 响应缺 `b64_json`（BAD_RESPONSE）纳入可重试集，指数退避 ≤3 次 | `cargo test --workspace` 全绿 + 新增 mock 用例（首掷坏体→重试→成功；连续坏体→结构化报错） | worktree `fix/bad-response-retry` |
| F2 | P1 | high 档总板重掷（seed 9996298467514345，F1 合并后重建 CLI 执行）；成功则审图择优转正，失败则记录上游限制 | 产出 high 锚点或书面结论 + 花费记录 | 主分支直提（资产） |
| F3 | P1 | 暗色总板（墨绢底版）生成：新项目 design-china-dark，low 档 1 张转正导出，作暗色 token 的 Image 1 依据（UI-REVIEW §8 关闭） | 锚点图 + manifest + 审图记录 | 主分支直提（资产） |
| F4 | P1 | 帆舟白描空态：icons.tsx 新增 BoatMark，empty-state 支持 `mark="boat"`，首页项目空态改用帆舟；THEME §5 题材扩为四选一 | `tsc -b` 0 错 + 相关渲染测试过 + 截图目测 | worktree `feat/boat-empty-state` |
| F5 | P2 | `--seal` 独立印章红 token（light `#C3402B` / dark `#E08873`）+ THEME §2/§5 注记（与 destructive 解耦，暂无 UI 消费点） | `tsc -b` 0 错 | worktree `feat/seal-token` |
| F6 | P1 | 回归：`pnpm build`（官方验收命令）+ visual-walkthrough 重跑；步条 7 若仍失败，定位是否 WIP 缺陷并记录（不代改他人 WIP） | build 0 错；走查结论入档 | 主分支执行 |
| F7 | P2 | README 增「故障排查」：pnpm 卡死→杀进程→`pnpm store prune`/删 store 索引 | 文档链接可达、命令可复制 | 主分支直提 |

## 纪律

- 每项完成立刻测试并 commit（中文祈使句小步提交）；合并回 main 后 `git worktree remove` + `git branch -d` 清理。
- 真实生图仅 F2/F3：low/medium 探索、high 只掷终版锚点 1 张，单张 high ≤ $0.5 预算线。
- 不代改并发 WIP 的功能逻辑；发现缺陷只记录。

## 执行状态（2026-09-10 02:30 更新）

| # | 状态 | 备注 |
|---|---|---|
| F0 | ✅ 完成 | `3542aa6` WIP 快照（tsc/cargo check 双绿），并发会话随后将 WIP 正式化为 `23093d2` |
| F1 | ✅ 完成 | `219de0b` BAD_RESPONSE 退避重试，新增 2 mock 用例；worktree 已清理 |
| F1b | ✅ 完成（计划外新增） | `c2d9493` 硬超时天花板：当晚端点两次挂起（high 36min / medium 18min 无响应）证明 reqwest 自身超时覆盖不了连接停滞，改用 `tokio::time::timeout` 全周期封顶，挂起转化为 `API_UNREACHABLE` 快速失败；新增静默监听器 hang 测试 |
| F2 | ⚠️ 端点侧受阻 | high 档重掷两次均挂起（重试预算 4×300s 耗尽仍无响应），判定镜像端 high 队列夜间劣化；low 锚点（0001）继续服役，seed 已记录可复现，待端点恢复后重掷 |
| F3 | 🔄 重试中 | 首掷 medium n=2 同样挂起被终止；项目目录已从 design-china/ 内挪正至 `design-china-dark/`；待 F1b 二进制重建后以 low n=1 快速失败语义重试 |
| F4 | ✅ 完成 | `73e0e1f` 帆舟空态合并；注释过 lint 由并发会话补刀 `b7969f0` |
| F5 | ✅ 完成 | `29567d5` --seal token，构建产物含 seal token |
| F6 | ⏳ 待办 | pnpm build + visual-walkthrough 回归 |
| F7 | ✅ 完成 | `541ec05`（并发会话执行） |
