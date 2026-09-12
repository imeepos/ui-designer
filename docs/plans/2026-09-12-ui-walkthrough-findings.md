# 2026-09-12 核心页面新鲜视觉走查与打磨差距清单（P1）

> 执行：frontend 只读评审会话（Mock 模式，零 API 花费）· 主干 d3b8c27 · 2026-09-12
> 驱动：`e2e/visual-walkthrough.mjs`（stock 16 步，PASS）+ 新增临时脚本 `e2e/tmp-polish-walkthrough.mjs` / `e2e/tmp-polish-probes.mjs`（申报见 §7）
> 截图：`e2e/screenshots/polish-*.png` 共 40 张（37 张主走查 + 3 张补充探针）；环境：无头 Chromium 1440×900，Vite dev(5173) 为主、preview(4173) 复核 stock 脚本
> 唯一视觉事实源：docs/THEME.md v3「墨航」。本清单按主会话任务书要求的五元组（位置/现状/证据/严重度/建议修法）组织，可直接拆实现任务书。

## 0. 表面映射（任务书命名 → 当前 IA）

C 系列 IA 改版后，任务书所称「四步工作流」已由「向导三段 + 工作台」承载，本走查按下列映射覆盖，全部有截图证据：

| 任务书表面 | 当前实现 | 证据截图 |
|---|---|---|
| 项目库 | `home-view.tsx`（搜索/卡片/分页/空态） | polish-01/22/22b/29/32 |
| 四步·项目信息 | 向导第 1 段（`create-wizard.tsx` §1） | polish-02/03 |
| 四步·总板候选 | 向导第 2 段生成+选锚；工作台「切换锚点」弹框 | polish-04/05/06/07/31 |
| 四步·画廊 | 工作台舞台（无限画布）+「切换设计稿」弹框+灯箱 | polish-08/13/14/15/16/30/33 |
| 四步·详情 | 详情表单（添加页面/组件）+ 重生成抽屉 + 血缘面板 | polish-09/09b/10/10b/12/17/18 |
| 设置账号区 | `settings-dialog.tsx` 账户段四态 | polish-23/23b/24/25/26/27/28/34 |
| 导出对话框 | `export-dialog.tsx` 表单/完成两相 | polish-19/20 |

---

## 1. 发现清单

严重度校准：**Critical 0 条**（未发现布局破碎、对比度不达标、流程不可用）；Important 3 条；Minor 9 条。另有一条复验结论见 §5，盲区见 §6。

### Important

**I1 · 步条三态：THEME §5 规格在实现中无对应组件，「当前步」缺缃色态（七维自检第 4 维不符）**
- 位置：THEME.md §5（L49）；`src/components/create-wizard.tsx:384-408`（SectionTitle 仅 done/未来两态）；`src/components/workspace/left-menu.tsx:186-202`（当前项=primary/10 底）；实际缃色 accent 消费点仅剩锚点徽标+焦点环（grep `bg-accent` 全库 2 处：card-actions.tsx:76、lineage-panel.tsx:123）
- 现状：四步步条已不存在；向导段标题完成步=苍青对勾✓、未来步=muted✓，但「当前步」无任何高亮（与未来步同款轮廓）；左菜单当前项用 primary/10 而非 THEME 规定的缃色圆点+加粗。THEME 是唯一视觉事实源，二者必须收敛
- 证据：polish-04（当前段「② 生成总览」与未来段无视觉差）、polish-06（完成段苍青对勾）、对照 polish-08（左菜单 primary 高亮）
- 建议：裁决方向二选一——修订 THEME §5 步条条款为新 IA（向导段标题+左菜单门控）并给「当前段/当前项」补缃色指示；或按 THEME 原文给向导段标题补缃色当前态实现

**I2 · 向导内总板生成中：无进度、不可取消，不符 THEME §5「生成中可取消」**
- 位置：`src/components/create-wizard.tsx:308-313`（仅一行 spinner 文案）；对照 `src/components/workspace/regen-drawer.tsx:185-196`（抽屉内 JobPanel 有进度+取消）
- 现状：向导内点「生成设计系统总板」后只显示「正在生成总板候选…」一行；无进度条、无取消钮。真实档 30 秒~3 分钟，用户在向导里只能干等或关弹窗（关弹窗后作业仍在后台、无任何入口查看进度）
- 证据：polish-04（spinner 行、无 JobPanel）；对照 polish-12（抽屉内 JobPanel 完整姿态：进度+耗时+取消）
- 建议：向导内复用 JobPanel（或至少加取消），保持与抽屉同姿态

**I3 · 卡片悬停 ghost 三件套漂移：THEME §5 规定的「重生成/删除/放大」无实现落点，组件已成死代码**
- 位置：`src/components/card-actions.tsx:10-67`（CardAction/EnlargeIcon/TrashIcon/RegenerateIcon 全库无调用，仅 AnchorBadge 被引用）；`src/components/home-view.tsx:82`（卡片 hover 仅描边+阴影+上浮）；`src/components/workspace/switch-dialog.tsx:120-129`（候选卡悬停仅放大镜一项）
- 现状：home 项目卡与候选卡悬停均无「重生成/删除」ghost 钮；删除/重生成分散在左菜单行悬停与舞台右上常驻钮。视觉规范与实现再次脱节（同 I1 性质），且留下死代码
- 证据：polish-22b（home 卡悬停无操作钮）、polish-14（候选卡悬停仅放大镜）、polish-08b（菜单行悬停删除钮——现状替代落点）
- 建议：随 I1 一并裁决——或在 THEME §5 改写卡片悬停姿态为现状（描边+放大），或补齐三件套；同时删除 card-actions.tsx 死代码

### Minor

**M1 · `ui/input.tsx` 注释与实现不符（注释陈旧，实现正确）**
- 位置：`src/components/ui/input.tsx:4`（注释「inputs use 8px radius」）vs L10 `rounded-md`（=6px，符合 THEME §4 输入 6px）
- 现状：实现正确、注释说反；易误导后续维护者「修正」圆角
- 证据：代码比对（截图 polish-09 输入框圆角目测 6px 一致）
- 建议：改注释为 6px

**M2 · 字阶：24px 页题档零消费，设置余额用 30px 超出字阶表**
- 位置：`src/App.tsx:72`（页头品牌题 text-sm=14px serif）；`src/components/settings-dialog.tsx:319`（余额 `text-3xl`=30px）；THEME §3 字阶表（12/13/14/18/24）
- 现状：12/13/14/18 在用；24 档无任何消费点；余额数字 30px 不在表内（数据型大数字，视觉上成立）
- 证据：polish-01（页题 14px）、polish-27（余额大数字）
- 建议：给字阶表补「数据型大数字 30px」条目，或把余额降到 text-2xl=24px 顺势消费页题档

**M3 · 锚点选中标记双轨制，向导内 10px 字号低于徽标下限**
- 位置：`src/components/create-wizard.tsx:338-343`（选中候选下方「✓ 锚点」text-[10px] primary 文本）vs `src/components/card-actions.tsx:70-84`（AnchorBadge text-[11px] 缃色底）
- 现状：同一语义（已选为锚点）在向导候选用苍青 10px 文本、在页头/左菜单/首页卡用缃色徽标，双轨且前者低于 UI-REVIEW 确立的「徽标≥11px」下限
- 证据：polish-06（向导内 ✓ 锚点文本）对照 polish-08/22（缃色 AnchorBadge）
- 建议：向导选中态统一复用 AnchorBadge（或把该文本提到 11px 并说明与徽标的语义分工）

**M4 · 切换弹框候选 id caption 10px（边界观察）**
- 位置：`src/components/workspace/switch-dialog.tsx:130-132`
- 现状：候选 id（c-2u-1）mono 10px；内容为 ASCII 非 CJK，不属「徽标」，风险低，但与 11px 下限纪律贴边
- 证据：polish-13/14
- 建议：提到 11px 与全局微字下限对齐

**M5 · 生成中文案与 THEME §5 不一致，且语义重复**
- 位置：`src/i18n/zh-CN.json` `job.etaNote`＝「真实生成约 30 秒~3 分钟，最长约 3 分钟。」；THEME §5＝「生成中，约需 30~120 秒…」
- 现状：「约 30 秒~3 分钟，最长约 3 分钟」前后重复；与 THEME 文案口径不同
- 证据：polish-12（JobPanel etaNote 实拍）
- 建议：改为「生成中，约需 30 秒~3 分钟」，并同步修订 THEME §5 文案样例

**M6 · 日期未显式传 locale，中文界面出现 en 式日期**
- 位置：`src/components/home-view.tsx:119`、`src/components/settings-dialog.tsx:337`（`toLocaleDateString()` 无参）
- 现状：宿主 locale 为 en 时 zh 界面显示「9/11/2026」「12/31/2024」；与 i18n 纪律（文案全走 i18n）精神不符
- 证据：polish-22（创建于 9/11/2026）、polish-27（注册于 12/31/2024）
- 建议：按当前 i18next 语言传 locale 参数（`toLocaleDateString(lang)`）

**M7 · slug 失焦静默清空非法输入，再报「必填」**
- 位置：`src/components/detail-forms.tsx:205`（onBlur `slugifyHint`，中文输入「大屏」被清洗为空串）
- 现状：用户输入被无提示改写后报「请输入页面 slug。」——错误原因（含非法字符）被掩盖，输入丢失
- 证据：polish-09b（输入框已空 + required 红字；探针记录 blur 后 value=""）
- 建议：slugifyHint 只在能产生合法转换时改写，否则保留原文并报 slugFormat

**M8 · 设置行内表单错误不带 hint，Mock 下不提示原因**
- 位置：`src/components/settings-dialog.tsx:52-60`（describeError 只取 message）、L386-390（formError 渲染）
- 现状：浏览器 Mock 点登录，行内显示「该能力尚未接入核心库。」；hint「当前为 Mock 预览，Tauri 桥接将在后续阶段启用。」被丢弃（toast 通道有 hint，行内通道没有）
- 证据：polish-25（行内错误单行）
- 建议：describeError 返回 {message,hint} 两行使 hint 同源展示

**M9 · preview 构建缺 favicon，每次启动 console 一条 404**
- 位置：仓库根无 favicon.ico（`dist/favicon.ico` 404）
- 现状：stock 脚本轮次 console 唯一一条报错即此；噪音级
- 证据：走查 console 记录（§4 第 2 条）
- 建议：补一枚 favicon（舵轮线稿即可）或在 index.html 内联 data-uri

（微观察，不单列：添加页面/组件提交按钮 `w-full` 拉通，与向导/设置表单 `w-fit` 姿态不一——polish-09/10b 对照 polish-02/23，随顺手统一即可；导出对话框「选择目录…」在 Mock 下为 no-op 按钮，已有 title 提示，属诚实降级，见 §6。）

---

## 2. 七维规范自检表（逐项声明）

| 维度 | 结论 | 依据 |
|---|---|---|
| 色板 | **一致** | `src/index.css:7-52` 与 THEME §2 v3 逐 token 相符（宣纸 #F7F5EE/墨绢 #161C1E/苍青 #2C6E78→dark #6FB4BD/缃色 #D9A514 双模式原值/朱砂 #C3402B→#E08873/淡墨 #E3E4DC/#2C3639/纸白 #FFFFFF/#1D2527/墨灰 #5C6E80/#9AB0B5）；实拍色感一致（polish-01/08/29） |
| 圆角 | **一致**（1 条注释瑕疵=M1） | 按钮 rounded-sm=4px（ui/button.tsx:8）、输入 rounded-md=6px（input/textarea）、卡片 rounded-lg=8px（home-view:82、job-panel、toast）、弹窗 rounded-xl=12px（modal.tsx:61）、徽标 rounded-full；四档派生 `--radius-sm/md/lg/xl` 与 §4 相符 |
| 字阶 | **列差异** | 基础 13px/行高 1.5 ✓（index.css:97）；12（text-xs）/14（text-sm）/18（text-lg，空态题）在用；**24px 档零消费**；余额 30px 超表（=M2） |
| 步条三态 | **列差异（结构性）** | 四步步条无实现；向导段标题两态（完成✓/未来=muted）、**当前步无缃色态**；左菜单当前项 primary/10 非 THEME 缃色圆点（=I1） |
| 空态插画 | **一致** | `empty-state.tsx:30-46`：176px（size-[176px]）、1.5px 线宽（icons.tsx 全部 strokeWidth 1.5）、primary 苍青单色、下距 24px（mb-6）、标题 font-serif；题材白名单四母题（helm/compass/anchor/boat），首页用帆舟 BoatMark ✓；实拍 polish-01（帆舟）/11（罗盘）/21（罗盘） |
| 徽标字号 | **一致**（2 处贴边=M3/M4） | AnchorBadge text-[11px]≥11 ✓ 缃色底深字 ✓ rounded-full ✓（card-actions.tsx:76）；血缘 agent-file 徽标缃色 ✓（lineage-panel.tsx:123）；向导「✓ 锚点」10px 与候选 caption 10px 为贴边偏差 |
| 悬停浮层 | **列差异** | 卡片 hover primary 描边+浮起+阴影 ✓、浮层阴影统一 `0 8px 24px rgba(2,8,23,.08)` ✓、动效 150ms ease-out ✓；但「ghost 三件套（重生成/删除/放大）」仅存放大镜一处，三件套组件死代码（=I3）；线性 1.5px 图标 ✓（lucide 默认 1.5 或显式 strokeWidth 1.5） |

---

## 3. 三态核对表（loading / 空态 / 错误态，逐面）

| 面 | loading | 空态 | 错误态 |
|---|---|---|---|
| 项目库 | ⚠️ **缺失**——projects 读取无骨架/占位（home-view 无 loading 分支；Mock 秒开不可见，真实桥下首次进入会白板片刻） | ✓ BoatMark 空态+CTA（polish-01）；搜索无结果 muted 文案（代码证据 home-view.tsx:69，本轮未截图） | ⚠️ 盲区——列表读取失败无 UI 路径（Mock 不失败，见 §6） |
| 画廊/工作台（舞台+切换） | ✓ ArtImage 解码骨架（art-image.tsx animate-pulse）+ JobPanel 进度/取消（polish-12） | ✓ 舞台罗盘空态三分支文案（polish-11 无候选/21 无锚点/已生成未选=stage-empty-switch 按钮） | ✓ ANCHOR_REQUIRED 门控 toast（polish-21，code+message+hint 三行，destructive 描边）；⚠️ 生成失败 toast 不可达（Mock 不失败，盲区）；画布图片加载失败文案存在（stage.tsx loadFailed） |
| 详情表单（向导/添加页/组件/重生成抽屉） | ✓ 向导 creating spinner（wizard-create Loader2，代码证据）+ 抽屉 savingBrief + JobPanel | n/a（表单无空态语义） | ✓✓ 字段级行内错误全绿：尺寸规则（polish-03「边长须为 16 的倍数，当前 4 x 4 不满足」）、slug 必填（polish-09b）、组件简报必填（polish-10b）、登录 NOT_IMPLEMENTED（polish-25）；均为 destructive 12px 行内红字 |
| 设置账号区 | ✓ checking 态「正在读取登录状态…」（polish-23b，延迟桩实拍） | ✓ 未登录=分段登录/注册表单（polish-23） | ✓ 四种全有：必填行内（polish-24）、NOT_IMPLEMENTED 行内（polish-25）、连接测试 destructive 面板（polish-26）、会话过期引导条 role=alert（polish-28）；已登录卡（polish-27）为第四态 |

---

## 4. console 输出记录（走查全程）

预期零新增报错，实际：

1. **dev 服务器全程（polish 主走查 37 张 + 探针 3 张，约 6 分钟）**：`console issues captured: 0` —— 0 error / 0 warning / 0 pageerror。
2. **preview 服务器（stock 16 步轮）**：唯一 1 条——`Failed to load resource: 404 @ http://localhost:4173/favicon.ico`（=M9，非 JS 错误）。
3. 另：stock 脚本轮次功能断言全部通过（缩放 60%→124%、平移位移、fit 复位、ANCHOR_REQUIRED 门控、卡片罗盘占位/位图缩略、mono 尺寸标签），console 无报错。

---

## 5. 已知遗留复验：UI-REVIEW P2「VALIDATION_ERROR 文案过泛」

**结论：不在（已按建议方向修复；泛化文案经 UI 不可达）。**

- 代码层：`src/state/toast.tsx:84-99` 显式把 core 的具体原因注入 `{{detail}}`（注释即引用 UI-REVIEW P2）；`src/i18n/zh-CN.json` `errors.VALIDATION_ERROR.message`＝「输入不合法：{{detail}}」；`src/lib/api/mock-api.ts`（L215/219/277/348/354/377/457/463/487）所有 VALIDATION_ERROR 均携带字段级 detail。
- 运行层（本轮实测）：
  - 组件简报为空 → 被客户端校验拦截，行内「请输入组件简报。」（polish-10b），**无** toast，更无泛化文案；
  - slug 非法/为空 → 行内「请输入页面 slug。」/格式文案（polish-09b）；
  - 画布尺寸非法 → 行内具体规则+当前值「边长须为 16 的倍数，当前 4 x 4 不满足。」（polish-03）。
- 残余风险（降级为盲区观察）：detail 原文是英文（如 "Project name is required"），若未来某路径绕过客户端校验直抵 adapter，中文界面会混排英文 detail；当前所有表单路径都有客户端前置校验，UI 不可达。

---

## 6. 盲区声明（本轮未能覆盖，只记录不花钱验证）

1. **真实 Tauri 桥路径**：auth_status 真实会话、keychain 读写、真实生成 30s~3min 的进度 ramp/`rudder://job` 事件、NO_CREDENTIALS toast+「打开设置」action 链、BALANCE_REFRESH 余额刷新——Mock 下不可达（本轮 signedIn/expired 两态经运行时桩驱动，见 §7 申报）。
2. **生成失败态**：Mock 永远成功，JOB 失败 toast / 画布加载失败文案（lightbox.loadFailed）无实拍。
3. **切换弹框「无候选」空态**（switch.empty 文案）：stage-switch 在候选数为 0 时 disabled，正常流不可达。
4. **真实导出写盘与目录选择**：「选择目录…」在 Mock 为 no-op（title 提示「桌面版将打开系统目录选择」）。
5. **暗色×英文组合**未重拍（stock 脚本既有 13-dark-en.png 覆盖）；暗色下设置/导出对话框未逐张走。
6. **WKWebView（Tauri 壳）渲染差异**：art-image.tsx 注释所述两个资产协议缺陷仅在真实壳复现。
7. 英文/暗色下的日期与余额等 M6 项表现未逐一张量核对。

---

## 7. 范围外观察与驱动申报

- **临时脚本申报（白名单内）**：`e2e/tmp-polish-walkthrough.mjs`（主驱动）、`e2e/tmp-polish-probes.mjs`(补充探针：错误 toast 计时/行内错误/悬停/checking 态/seed DOM 实测)。二者均只写 `e2e/screenshots/polish-*.png`，不改任何既有文件、无 git 写操作。
- **运行时桩申报**：signedIn/expired/checking 三态通过 Vite dev 模块拦截给 `src/lib/api/auth.ts` 的浏览器降级分支注入 `window.__rudderAuthStatus` 桩实现（仓库文件零改动）；p22/23b/27/28 四张截图受此驱动，已在各条目标注。
- **产物覆盖申报**：为验证 stock 基建，重跑了 `e2e/visual-walkthrough.mjs`（PASS，exit 0），原 01~16 截图被今日新跑覆盖（均不入库；旧图已备份 /tmp/rudder-shots-backup）。
- **范围外观察**：① 向导关闭后作业无进度入口（并入 I2 修）；② 首页卡悬停无操作钮属 IA 收敛结果（并入 I3 裁决）；③ 向导候选网格 grid-cols-4 在窄弹窗下不收缩（1440 宽下无碍，移动窗口未测，记录备查）。
- **实测补充事实**：错误 toast 自动消失实测 9366ms（≈9s 设计值，非不消失）；血缘面板 seed 值 DOM 实测完整显示（Mock seed 本身 3 位，无截断——此前目测「截断」疑义已排除）。

## 8. 下一步建议（供主会话拆任务）

1. **THEME §5 对齐裁决**（I1+I3+M5 合并一张任务书）：步条/卡片悬停条款改为新 IA 口径或补实现，顺带删 card-actions 死代码、统一生成中文案。
2. **向导生成态补 JobPanel**（I2，单点小改）。
3. **微字与一致性小包**（M1/M2/M3/M4/M6/M7/M8/M9 一张任务书顺手清）。
