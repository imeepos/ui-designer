# 舵 Rudder · 主题规范（舵主题）v3「墨航」

> 本规范约束**舵工具自身**的界面视觉，也是「舵主题」一词的定义。实现方式：TailwindCSS 4 + shadcn/ui CSS 变量（`:root` / `.dark`），light/dark 双模式。
> v3（中国风重设计）：采纳 `design-china/` 自举设计集（苍青墨航色系），中国风转译见 `docs/RESEARCH-CHINA-UI.md`。v2（海军蓝航海风）留档于 git 历史。

## 1. 设计气质：墨航 Ink-Voyage

同一片海，从「深海军蓝的军舰」驶入「宋代水墨的绢本手卷」。气质关键词：**素雅、书卷、克制的工具感**——苍青（青瓷系）为主舵向，缃色为焦点舵柄，朱砂印章是全卷唯一浓色点缀，宣纸底大面积留白承载内容。白描线稿表达航海母题（舵轮/罗盘/帆舟/锚）。无渐变、无重阴影、无花哨动效（仅 150ms ease-out 微动效）；**边框优先于阴影**（1px 发丝界格）。

## 2. 色板（CSS 变量）

色系来源＝墨航锚点图（`design-china/board/anchor.png`）；传统色名与转译依据见 RESEARCH-CHINA-UI.md §3。

| Token | Light | Dark | 传统色 | 用途 |
|---|---|---|---|---|
| `--background` | 宣纸 #F7F5EE | 墨绢 #161C1E | 宣纸 / 墨绢 | 画布底 |
| `--foreground` | 墨玄 #3A4247 | 雾白 #E9ECE8 | 墨玄 | 正文（纸底 ≈9.3:1） |
| `--primary` | 苍青 #2C6E78 | 提亮苍青 #6FB4BD | 苍青（天水碧加深） | 主按钮/激活态/选中框/白描线稿 |
| `--primary-foreground` | #FFFFFF | #0F1517 | — | 主按钮文字（≈5.8:1 / ≈7.8:1） |
| `--accent`（缃色） | 缃色 #D9A514 | 缃色 #D9A514（原值保持） | 缃色 | 向导当前段/左菜单当前项、锚点徽标、焦点环 |
| `--accent-foreground` | #2A2410 | #1C1607 | — | 金底深字（≈6.9:1 / ≈7.5:1） |
| `--destructive` | 朱砂 #C3402B | 朱砂·亮 #E08873 | 朱砂 | 删除/错误 |
| `--seal`（v3.1） | #C3402B | #E08873 | 朱砂 | 品牌「舵」字印章专用（与 destructive 解耦，暂无 UI 消费点） |
| `--muted` | #EFECE3 | #212A2D | 纸灰 | 次级面板/骨架屏 |
| `--muted-foreground` | 墨灰·深 #5C6E80 | #9AB0B5 | 墨灰 | 说明文字（≈4.8:1 / ≈7.6:1） |
| `--border` | 淡墨 #E3E4DC | #2C3639 | 淡墨 | 发丝分隔线/卡片描边 |
| `--input` | #D6D8CE | #2C3639 | — | 输入框描边 |
| `--card` | #FFFFFF | #1D2527 | 纸白 | 卡片面 |

**朱砂使用纪律**：删除/错误走 `--destructive`；品牌印章装饰走 `--seal`（v3.1 起独立 token，二者数值暂同源、语义解耦）；均禁作大面积色块或正文色。
**dark 派生规则**：宣纸→墨绢、纸白→加深一档；苍青提亮（#2C6E78→#6FB4BD）保深底可读与深字按钮达标；缃色原值保持（深底 ≈7.5:1）；朱砂提亮保文字对比。

## 3. 字体与字阶

- 无衬线（正文）：`Inter, "PingFang SC", "Noto Sans SC", system-ui, sans-serif`。
- **宋体展示字（v3 新增，`font-serif` 工具类）**：`"Noto Serif SC", "Songti SC", "STSong", "SimSun", serif`——用于**页题/品牌标题/空态标题**；工具高密度小标题仍走无衬线，避免满屏书卷气拖慢扫读。
- 等宽（prompt/JSON/seed）：`"JetBrains Mono", ui-monospace`。
- 字阶：12 / 13（默认正文）/ 14（强调）/ 18（区块题）/ 24（页题）/ 30（数据型大数字，余额等大数值展示）；行高 1.5；标签可 `tracking-wide`。

## 4. 几何与间距

- 圆角四档（承 v2，方中带圆如界格）：**按钮 4px / 输入 6px / 卡片 8px / 弹窗 12px**；`--radius: 0.5rem` 派生 `sm/md/lg/xl`；徽标 `999px` 全圆。
- 间距：8px 基数；面板内边距 16px；区块间 24px；三栏栏宽 左 240px / 右 320px / 中自适应。
- 边框优先于阴影：`1px solid var(--border)`；阴影仅浮层（`0 8px 24px rgba(2,8,23,.08)`）。

## 5. 组件姿态

- Button：primary 实心苍青；secondary 纸灰反色；ghost 用于画廊与浮层的次级操作钮（hover 走 muted，不走缃色）。
- 导航当前态（IA＝向导三段 + 工作台）：向导段标题三态：完成段=苍青（primary）对勾✓；当前段=缃色（accent）圆点+段标题加粗；未来段=muted 轮廓。左菜单当前项=缃色（accent）圆点+加粗（不铺底色，替代旧 primary/10 底）。当前态圆点一律用 token 变量（`bg-accent`），禁止内联 hex。
- 锚点徽标/血缘当前态：缃色底 + 深字（bg-accent）。
- 卡片：1px border；hover=primary 描边 + 浮起 + 阴影 `0 8px 24px rgba(2,8,23,.08)`，150ms ease-out。卡片悬停不再浮现 ghost 三件套（重生成/删除/放大），现行落点：候选卡悬停仅放大镜一项（进灯箱）；删除走左菜单行悬停操作钮；重生成走详情区重生成抽屉。
- 生成中：骨架屏 + 一行 muted 文案（"生成中，约需 30 秒~3 分钟"），可取消。
- 空态插画规范（v3 增补帆舟）：居中细线插画 176px、1.5px 线宽、**primary 苍青单色白描**、下留白 24px；题材限**舵轮 / 罗盘 / 锚 / 帆舟**四选一（帆舟=墨航概念图主母题，`BoatMark`，首页项目空态默认用之）+ 一个 primary CTA；标题用 `font-serif`。
- 品牌标识：页头保留舵轮线稿标（primary）；朱砂「舵」字方印仅出现在设计资产与文档插图；实现层若引入印章元素，一律用 `--seal` token（`bg-seal`/`text-seal`），不得挪用 destructive。

## 6. 多语言与文案基调

- zh-CN 简洁专业（"生成设计系统总板"），en 同义（"Generate design system board"）。
- 数字/尺寸用等宽字体；金额与耗时文案不做夸张修辞。

## 7. 设计资产与追溯

- v3 自举设计集：`design-china/`（总板锚点 + library/gallery/detail 三屏 + stepper-buttons 组件图 + 全部 prompt 与 manifest，`rudder export` 包在 `design-china/export/`）。
- 重设计方案与实现映射：`docs/REDESIGN-CHINA.md`。
