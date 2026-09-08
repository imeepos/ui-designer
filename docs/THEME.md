# 舵 Rudder · 主题规范（舵主题）v2

> 本规范约束**舵工具自身**的界面视觉，也是「舵主题」一词的定义。实现方式：TailwindCSS 4 + shadcn/ui CSS 变量（`:root` / `.dark`），light/dark 双模式。
> v2（Phase 5 裁决）：采纳自举锚点图色系与四档圆角（docs/UI-REVIEW.md 负责人裁决节）。

## 1. 设计气质
舵 = 航海掌舵。气质关键词：**沉稳、专业、克制的工具感**——深海军蓝为主舵向，黄铜色为转向舵柄的点缀，大面积中性色承载内容。无渐变滥用、无重阴影、无花哨动效（仅 150ms ease-out 微动效）。

## 2. 色板（CSS 变量，OKLCH 观感描述）
色系来源＝自举锚点图（design-assets/board/anchor.png）：深海军蓝主舵向、琥珀金黄铜点缀、纸白底。

| Token | Light | Dark | 用途 |
|---|---|---|---|
| `--background` | 纸白 #FAFAF8（微暖） | 深夜海军 #0E1626 | 画布底 |
| `--foreground` | 墨石 #2F3A44 | 雾白 #E8ECF4 | 正文 |
| `--primary` | 深海军蓝 #0B3D91 | 提亮舵蓝 #5C88DD | 主按钮/激活态/选中框 |
| `--primary-foreground` | #FFFFFF | #0B1220 | 主按钮文字 |
| `--accent`（舵柄黄铜） | 琥珀金 #D4A017 | 琥珀金 #D4A017（保持可读） | 步骤条当前步、锚点标记、焦点环 |
| `--accent-foreground` | 深铜棕 #14213A | #14213A | 黄铜底上的文字（深色字保证对比） |
| `--muted` | #EEF1F5 | #172238 | 次级面板/骨架屏 |
| `--muted-foreground` | 中灰 #6B7280 | 雾灰蓝 #93A2BE | 说明文字 |
| `--border` | 浅石灰 #E2E8F0 | #24304D | 分隔线/卡片描边 |
| `--input` | 石灰 #CBD5E1 | #24304D | 输入框描边 |
| `--card` | #FFFFFF | #131E33 | 卡片面 |
| `--destructive` | #DC2626 | #F87171 | 删除/错误 |

**dark 派生规则**（v2）：海军蓝整体提亮（#0B3D91 → #5C88DD）保证深底可读与白字按钮达标；纸白转深夜蓝（#FAFAF8 → #0E1626），内容面相应加深一档；琥珀金 #D4A017 保持原值——在深底上对比度 ≈ 7.5:1，无需调整。
**对比度自审（WCAG AA）**：light 主色对白底 10.0:1、正文对纸白 11.1:1、说明文字 #6B7280 对纸白 4.5:1、黄铜底深字 6.7:1；dark 主色底白字/深字均 ≥ 4.5:1，说明文字 ≥ 4.6:1。黄铜 #D4A017 在浅底上仅作填充/描边/图形，不作正文色（对浅底 ≈ 2.2:1）。

## 3. 字体与字阶
- 字族：`Inter, "PingFang SC", "Noto Sans SC", system-ui, sans-serif`；等宽（prompt/JSON 展示）：`"JetBrains Mono", ui-monospace`。
- 字阶：12 / 13（默认正文）/ 14（强调）/ 18（区块题）/ 24（页题）；行高 1.5；正文 `tracking-normal`，标签可 `tracking-wide` 全大写小字号。

## 4. 几何与间距
- 圆角四档（v2 裁决，锚点图口径）：**按钮 `4px` / 输入 `6px` / 卡片 `8px` / 弹窗 `12px`**；`--radius: 0.5rem`（8px）为基准派生——`--radius-sm` 4px、`--radius-md` 6px、`--radius-lg` 8px、`--radius-xl` 12px。徽标 `999px` 全圆。
- 间距：8px 基数；面板内边距 16px；区块间 24px；三栏栏宽 左 240px / 右 320px / 中自适应。
- 边框优先于阴影：`1px solid var(--border)`；阴影仅浮层（`0 8px 24px rgba(2,8,23,.08)`）。

## 5. 组件姿态（shadcn 基础上的定制）
- Button：primary 实心海军蓝；secondary 用 muted 反色；ghost 用于画廊 hover 操作。均无圆角外的花饰。
- 步骤条（四步流程）：完成步=primary 对勾，当前步=accent 黄铜圆点+加粗，未来步=muted 置灰。
- 卡片缩略图：1px border + hover 时 primary 描边 + 右上角浮现 ghost 操作（重生成/删除/放大）。
- 生成中：骨架屏 + 一行 muted 文案（"生成中，约需 30~120 秒…"），可取消。
- 空状态插画规范（v2 落地）：居中细线插画 **160~200px**（实现取 176px）、**1.5px 线宽**、**海军蓝单色**（light 用 `--primary`，dark 同）、插画下留白 24px；题材限**舵轮 / 罗盘 / 锚**三选一（纯 CSS/内联 SVG，禁照片与多色）+ 一个 primary CTA。

## 6. 多语言与文案基调
- zh-CN 简洁专业（"生成设计系统总板"），en 同义（"Generate design system board"）。
- 数字/尺寸用等宽字体；金额与耗时文案不做夸张修辞。
