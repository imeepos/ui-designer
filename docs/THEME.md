# 舵 Rudder · 主题规范（舵主题）v1

> 本规范约束**舵工具自身**的界面视觉，也是「舵主题」一词的定义。实现方式：TailwindCSS 4 + shadcn/ui CSS 变量（`:root` / `.dark`），light/dark 双模式。

## 1. 设计气质
舵 = 航海掌舵。气质关键词：**沉稳、专业、克制的工具感**——深海军蓝为主舵向，黄铜色为转向舵柄的点缀，大面积中性色承载内容。无渐变滥用、无重阴影、无花哨动效（仅 150ms ease-out 微动效）。

## 2. 色板（CSS 变量，OKLCH 观感描述）
| Token | Light | Dark | 用途 |
|---|---|---|---|
| `--background` | 近白 #FAFAF8（微暖纸感） | 深海军 #0B1220 | 画布底 |
| `--foreground` | #0F172A | #E6EAF2 | 正文 |
| `--primary` | 海军蓝 #1D4ED8 | 亮舵蓝 #3B82F6 | 主按钮/激活态/选中框 |
| `--primary-foreground` | #FFFFFF | #0B1220 | 主按钮文字 |
| `--accent`（舵柄黄铜） | #B45309 | #D97706 | 步骤条当前步、锚点标记、焦点环 |
| `--muted` | #EEF1F5 | #16203A | 次级面板/骨架屏 |
| `--muted-foreground` | #64748B | #8A97B1 | 说明文字 |
| `--border` | #E2E8F0 | #1E2A44 | 分隔线/卡片描边 |
| `--card` | #FFFFFF | #101A30 | 卡片面 |
| `--destructive` | #DC2626 | #EF4444 | 删除/错误 |

## 3. 字体与字阶
- 字族：`Inter, "PingFang SC", "Noto Sans SC", system-ui, sans-serif`；等宽（prompt/JSON 展示）：`"JetBrains Mono", ui-monospace`。
- 字阶：12 / 13（默认正文）/ 14（强调）/ 18（区块题）/ 24（页题）；行高 1.5；正文 `tracking-normal`，标签可 `tracking-wide` 全大写小字号。

## 4. 几何与间距
- 圆角：卡片 `12px`（`--radius: 0.75rem`）、按钮/输入 `8px`、徽标 `999px`。
- 间距：8px 基数；面板内边距 16px；区块间 24px；三栏栏宽 左 240px / 右 320px / 中自适应。
- 边框优先于阴影：`1px solid var(--border)`；阴影仅浮层（`0 8px 24px rgba(2,8,23,.08)`）。

## 5. 组件姿态（shadcn 基础上的定制）
- Button：primary 实心海军蓝；secondary 用 muted 反色；ghost 用于画廊 hover 操作。均无圆角外的花饰。
- 步骤条（四步流程）：完成步=primary 对勾，当前步=accent 黄铜圆点+加粗，未来步=muted 置灰。
- 卡片缩略图：1px border + hover 时 primary 描边 + 右上角浮现 ghost 操作（重生成/删除/放大）。
- 生成中：骨架屏 + 一行 muted 文案（"生成中，约需 30~120 秒…"），可取消。
- 空状态：居中细线插画（纯 CSS/内联 SVG，航海元素：舵轮/罗盘线稿）+ 一个 primary CTA。

## 6. 多语言与文案基调
- zh-CN 简洁专业（"生成设计系统总板"），en 同义（"Generate design system board"）。
- 数字/尺寸用等宽字体；金额与耗时文案不做夸张修辞。
