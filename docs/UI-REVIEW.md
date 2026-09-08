# UI-REVIEW — 舵 Rudder 自举实测（Phase 4）

> 执行环境：rudder CLI v0.1.0（target/debug/rudder，2026-09-08 构建）· gpt-image-2 · 画布 1536x1024 · 项目「舵 Rudder · Studio」。
> 本次自举严格按 skill/rudder-design/SKILL.md 四步执行，全程仅凭 Skill 文档完成。以下两节是 Dogfooding 的打磨清单。

## 工具缺陷（含复现命令）

1. **board 候选缺 `seed`，总板不可复现**（P0）
   cli.md 承诺 `board generate` 返回 `{candidates: [{id, file, prompt, seed, size, quality}]}`，实际 CLI JSON 与导出 `manifest.json` 的 board 候选均无 `seed` 字段（pages 条目有）。导致 `--seed` 无法用于总板候选复现/升规格重掷，锚点机制少了确定性。
   复现：`rudder board generate --n 1 --quality low --dry-run --json | jq '.data.plan.params|keys'`；真实跑一次后 `python3 -c "import json;print(list(json.load(open('design-assets/manifest.json'))['board']['candidates'][0].keys()))"`（无 seed）对比 `['pages'][0]`（有 seed）。

2. **没有 `page update` / `component update` 命令，改简报必须手编 project.json**（P0）
   SKILL.md §3 写 "Regenerate with an amended brief to iterate"，cli.md 写 "edit `styleBrief`/page `brief` fields freely"——但磁盘 `project.json` 的实际键名是 snake_case `style_brief`，且 CLI 无任何 update 子命令；代理只能按 cli.md 的键名去改一个不存在的字段，或带着"勿在生成中手编"的警告裸编 JSON。本次为修 gallery 步条标签被迫用 python 手改。
   复现：`rudder page add --help 2>&1 | grep -c update`（=0）；`python3 -c "import json;print([k for k in json.load(open('project.json')) if 'brief' in k])"`（输出 `style_brief`，非 `styleBrief`）。

3. **`board generate` 与 `page/component generate` 返回形状不对称，且未写进 cli.md**（P1）
   board 是 `data.candidates[]`；page/component 是 `data.results[].candidates[]`（单 slug 也包一层 results）。写通用解析脚本必踩坑——本次执行实际抛了 `KeyError: 'candidates'`（所幸只是解析层，未重复花费）。
   复现：`diff <(rudder board generate --n 1 --dry-run --json | jq -r '.data|keys[]') <(rudder page generate <slug> --dry-run --json | jq -r '.data|keys[]')`。

4. **候选数组逐项重复整段 prompt，--json 输出爆炸**（P1）
   `--n 3` 时每个 candidate 携带同一 prompt 全文（约 1KB×n），外加 plan.params.prompt 再来一份。对代理上下文和日志都是浪费；prompt 应只在 plan 或 promptLog 出现一次，候选引用 id 即可。
   复现：`rudder board generate --n 3 --quality low --yes --json | python3 -c "import sys;print(sys.stdin.read().count('Purpose: a UI design system board'))"`（输出 4 = 3 候选 + 1 plan）。

5. **生成类命令默认 `quality high`，直接 `--yes` 就是高价调用**（P1）
   cli.md 默认值：board `--n 4`、quality `high`。SKILL 硬规则要求"探索用 low"，但 CLI 默认档是 high——代理漏带 `--quality low` 时一次 `--yes` 就 4 张高清。建议默认 low，或 `high` 必须显式确认。
   复现：`rudder component generate button-set --dry-run --json | jq '.data.results[0].plan.params.quality'`（输出 high）。

6. **非 --json 模式 stdout 恒空，成败只能看 stderr**（P2）
   `init` 成功摘要走 stderr、stdout 无输出；人工/脚本第一眼无法从 stdout 判定结果（本次首条命令即遇）。cli.md 有说明，但对"summary 命令"而言 stdout 空仍是反直觉的输出格式。
   复现：`rudder list 1>/dev/null; echo $?`（stderr 有字、stdout 空、exit 0）。

7. **export 把落选探索稿一并拷入资产包**（P2）
   `design-assets/board/candidates/` 含全部 4 张候选（3 张 low 探索稿）。下游代理拿到的是"一个风格宇宙"，但落选稿混在终版旁，容易被误当可用素材；建议 export 默认只带 current/anchor，候选归档到 `candidates/` 子目录外或加 `--with-candidates` 开关。
   复现：`ls design-assets/board/candidates | wc -l`（=4，其中 3 张未被选）。

## 界面改进项（对照生成图 × docs/THEME.md × docs/PRD 三大界面）

1. **P0 · 主色口径冲突**：锚点图主色为深海军蓝 `#0B3D91`，THEME.md `--primary` 为 `#1D4ED8`（light）/`#3B82F6`（dark）。工具自身界面实现必须二选一：改 THEME 采图片色，或改简报重出锚点。当前状态下"按图实现"与"按 THEME 实现"会产出两套蓝。
2. **P0 · 圆角口径冲突**：板面 Corner Radius 标注 按钮 4px / 输入 6px / 卡片 8px / 弹窗 12px，THEME.md 为 按钮/输入 8px、卡片 12px、徽标 999px。代码只能落一套，需 Phase 5 裁决后同步改 THEME 或改锚点简报（把四档半径写进 styleBrief）。
3. **P1 · 四步步条是产品命脉，图片不可直接照抄**：三次页面生成中两次步条标签失真（自造"项目创建/设计生成/素材管理/导出发布"、detail 版渲染成总板分节 tab、gallery 终版第 1 步花字）。实现 PRD 四步流程时以 THEME §5 + `components/stepper/current.png` 的三态规格为准；同时建议 CLI 在页面提示词自动追加"标签逐字不变量"（verbatim label list），减少重掷。
4. **P1 · 右栏表单两页不一致**：gallery 右栏（名称/简报 36/200/尺寸 1536x1024/质量"高清 (2x)"）与 detail 右栏（项目名/画布尺寸/品牌简报 0/300/质量"标准质量"）字段与文案对不上。产品实现需统一"详情表单 schema"（字段、计数上限、质量枚举），并在 i18n JSON 定稿，避免实现时各抄一张图。
5. **P1 · 空态插画规范缺位**：library 空态罗盘线稿与 THEME §6 精神一致，但 THEME 只写了"舵轮/罗盘线稿"，未定尺寸、线宽、单色规则。补：居中 160~200px、1.5px 线宽、海军蓝单色、下距 24px，插画元素限舵轮/罗盘/锚三选一。
6. **P2 · 字阶收敛**：板面 body=16px，THEME 默认正文 13px；工具 UI 为三栏高密度布局，应按 THEME 12/13/14/18/24 落地，图片字阶仅取其"层级关系"（display 48 → caption 12 的比例与字重节奏）。
7. **P2 · 缩略图微文字防花**：所有生成图在 <12px 的 CJK 标签处都易花字（组件卡内按钮标签、徽标）。工具自身 UI 的缩略图角标/徽标字号 ≥11px、避免全大写+加粗+宽字距三件套同用。
8. **P2 · 暗色模式无图可依**：本次图片集仅定义 light。THEME 承诺 light/dark 双模式，Phase 5 实现暗色前需补生成一张暗色总板（quality low 足够）作为 Image 1 参考，否则暗色 token 属"发明"。
9. **P2 · 悬停浮层操作对齐确认**：gallery 悬停卡右上浮现 3 个 ghost 图标钮（重生成/删除/放大）与 THEME §5 卡片姿态一致，实现时图标统一线性 1.5px、hover 描边 `--primary`、动效 150ms ease-out，浮层阴影按 THEME 允许的 `0 8px 24px rgba(2,8,23,.08)`。

## 花费与过程存档

- 张数：low 11（board 3 + 页面 6 + 组件 2）+ high 2（board 0004、gallery 0003）= 13 张，均在预算 $1.5 内（按 gpt-image 同档价目估算 ≈ $0.68）。
- 全程命令与 dry-run 计划：见 `design-assets/PROMPTS.md` 与 `design-assets/manifest.json`。
