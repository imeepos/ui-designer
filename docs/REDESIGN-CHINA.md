# 舵 Rudder · 中国风整体界面重设计方案（墨航 Ink-Voyage）

> 执行方式：**用舵自身的 CLI 完成重设计自举**（rudder CLI v0.1.0 · gpt-image-2 · 画布 1536×1024）。
> 设计学习笔记：`docs/RESEARCH-CHINA-UI.md` · 主题规范：`docs/THEME.md` v3 · 设计资产：`design-china/`。

## 1. 一句话方案

**墨航**：保留「航海掌舵」的产品灵魂（舵轮/罗盘/帆舟/锚全部保留为白描线稿母题），把画法从「深海军蓝的军舰」换成「宋代水墨的绢本手卷」——苍青主舵向、宣纸底、缃色焦点、朱砂印章点睛、宋体展示字。

## 2. CLI 自举过程（四步全走真实 API）

| 步骤 | 命令 | 结果 |
|---|---|---|
| 建项目 | `rudder init "舵 · 墨航 Rudder Ink-Voyage" --size web --dir design-china` | 1536×1024 画布 |
| 总板 | `board generate --prompt-file prompts/board.md --n 4 --quality low --yes` → 审图 → `--quality high --seed …` 重掷遇端点异常 → `board pick 0001`（low 稿即终版锚点） | 苍青墨航总板，hex 标签逐字正确 |
| 三屏 | `page generate library/gallery/detail --quality medium --prompt-file … --yes`（gallery 首掷遇端点异常，换 seed 重掷成功）→ `page pick` ×3 | 项目库/画廊/详情三屏，标签逐字命中 |
| 组件 | `component generate stepper-buttons --quality medium --prompt-file … --yes` → `component pick` | 步条三态 + 按钮四类三态 + 输入四态 + 徽标 |
| 导出 | `export --out export` | 5 图 + manifest.json + PROMPTS.md + DESIGN.template.md |

真实花费：low×4 + medium×4 ≈ **$0.12**（远低于 $1.5 预算线）。
**服务端发现**：该镜像端点偶发 2xx 响应体缺 `b64_json`（high 档 generations 与 medium 档 edits 各遇一次）；CLI 以 `BAD_RESPONSE` 结构化报错、未落盘半成品，换 seed 重掷即恢复。建议后续给 core 客户端把该错误纳入可重试清单。

## 3. 设计资产清单（design-china/export/）

- `board/anchor.png` — 墨航设计系统总板：8 色板（含 hex+传统色名标签）、宋体/无衬线/等宽三字系样本、按钮/输入/下拉/卡片/开关/复选/步条组件样本、白描图标（舵轮/罗盘/帆舟/锚/卷轴/毛笔）、间距与四档圆角规则。
- `pages/library/current.png` — 项目库三栏屏：白描帆舟空态 + 项目列表 + 项目详情表单。
- `pages/gallery/current.png` — 画廊工作台：四步步条（项目✓→总板●→页面→组件）+ 总板候选网格 + 锚点朱砂印标签 + hover 三 ghost 操作 + 右栏生成表单。
- `pages/detail/current.png` — 详情屏：宣纸衬底大图预览 + 血缘缩略条（当前高亮）+ 锚点信息 mono 参数面板。
- `components/stepper-buttons/current.png` — 组件规格图：步条三态、按钮四类×三态、输入四态（含缃色焦点环/朱砂错误态）、徽标三枚。

## 4. 实现（已完成）

| 层 | 变更 | 说明 |
|---|---|---|
| `src/index.css` | light/dark 全量 token 替换（THEME v3 色板） | 宣纸/墨绢底、苍青 primary、缃色 accent+ring、朱砂 destructive、淡墨 border；全部对比度 ≥AA（详见 THEME.md §2 注） |
| `src/index.css` | 新增 `--font-serif` 宋体展示字族 | Noto Serif SC / Songti SC / STSong / SimSun |
| `src/App.tsx` | 页头品牌标题改 `font-serif` | 书卷气品牌面 |
| `src/components/empty-state.tsx` | 空态标题改 `font-serif` | 空态即留白，题字用宋体 |
| `docs/THEME.md` | v2→v3 重写 | 墨航气质、色板表（传统色名+对比度）、宋体规则、朱砂纪律、白描空态规范 |
| 组件层 | 零改动 | 三栏/步条/卡片/徽标全部 token 驱动，换变量即整体换肤；四档圆角（方中带圆）承 v2 不变 |

验证：`pnpm build` 零错（见提交记录）；界面文案未动，i18n 全覆盖不变。

## 5. 遗留与后续（不跨 Phase 自行扩张）

1. **high 档重掷**：镜像端点修复后可对锚点补一张 high 终版（seed 已记录在 manifest，可复现构图）。
2. **暗色总板**：UI-REVIEW §8 遗留项，暗色 token 本轮按派生规则给出（负责人截图终审），可补生成暗色总板作 Image 1。
3. **帆舟空态**：概念图中的白描帆舟插画 loved，但实现空态保持舵轮/罗盘/锚三件套（THEME §5 纪律）；若采纳帆舟需先修订 THEME §5 题材清单。
4. **印章 logo**：朱砂「舵」方印暂不进实现（避免 destructive 语义混用），如需引入建议新增独立 `--seal` token。
