# DESIGN.md — 舵 Rudder · Studio

Source: design-assets/manifest.json (generated 2026-09-08, rudder v0.1.0, model gpt-image-2, canvas 1536x1024)
依据 Skill 纪律：**所有取值均读自导出图片**（色值取总板 hex 标签，不取像素采样）。

## Tokens

### Palette

来源：`board/anchor.png` §1 COLOR PALETTE（7 枚带 hex 标签的色板）

| token | hex | 用途（板上标注） |
|---|---|---|
| primary | #0B3D91 | 深海军蓝 Navy Blue（主色）— 主按钮、选中态、链接、完成步 |
| accent | #D4A017 | 黄铜金 Amber（强调）— 当前步圆点、焦点、锚点标记 |
| text | #2F3A4A | 深海石板 Slate Gray（文字）— 正文/标题 |
| text-secondary | #6B7280 | 次级灰 Secondary Gray（次要）— 说明文字、置灰态 |
| border | #CBD5E1 | 边界灰 Border Gray（边框）— 分隔线、卡片描边 |
| surface-muted | #F1F5F9 | 表面灰 Surface Gray（表面）— 次级面板、骨架屏 |
| bg | #FAFAF8 | 纸白 Paper White（背景）— 画布底 |
| card | #FFFFFF | 卡片面（板上卡片填充，无标签，目视纯白采样） |

### Typography

family：Inter（英文）+ PingFang SC（中文）；辅助字见 `board/anchor.png` §2 TYPEFACE 标本行
来源：`board/anchor.png` §2 TYPOGRAPHY（逐行标注尺寸/字重）

| role | size | weight | notes |
|---|---|---|---|
| display | 48px | Bold | 页面主题 |
| heading-1 | 32px | Semibold | 区块题 |
| heading-2 | 24px | Semibold | 卡片/小标题 |
| subheading | 18px | Medium | 强调正文 |
| body | 16px | Regular | 正文 |
| caption | 12px | Regular | 辅助信息 |

### Geometry

来源：`board/anchor.png` §5 SPACING & GRID RULES

- radius：button 4px · input 6px · card 8px · modal 12px（板面 Corner Radius 四档标注）
- spacing base：8px；刻度 4 / 8 / 16 / 24 / 32px；栅格 12 列、间距 24px、边距 24px
- borders：1px 直线；图标线性风格，线宽 1.5px、圆角 2px（`board/anchor.png` §4）
- 三栏栏宽（页面图实测）：左 240px / 右 320px / 中自适应

## Components

### button-set（buttons）

来源：`components/button-set/current.png`

- variants：Primary（实心 #0B3D91、白字）/ Secondary（白底、1px 海军蓝描边、蓝字）/ Ghost（无边框、灰字）
- states：Default / Hover / Active / Disabled（灰底低对比）/ Focus（描边高亮）
- sizes：sm 28px · md 32px · lg 40px（高度）；圆角统一 8px
- 标签规范：主要按钮 / 次要按钮 / 幽灵按钮

### stepper（navigation）

来源：`components/stepper/current.png`

- 完成态：海军蓝实心圆 + 白色对勾 + 加粗标签，连接线海军蓝
- 当前态：黄铜 #D4A017 实心圆点 + 加粗标签，入线黄铜
- 未来态：灰描边空心圆 + 数字 + 置灰标签
- 紧凑变体：顶栏用（圆点在上、标签在下，用于三页顶部）
- 规格：圆 20px · 间距 8px · 连接线 1px
- 四步固定标签：1 新建项目 · 2 设计系统总板 · 3 功能页面 · 4 组件设计

### Input / Select / Card / Toggle / Checkbox

来源：`board/anchor.png` §3 COMPONENT SAMPLES —— 输入框（圆角内含放大镜后缀图标）、下拉选择器（右置 V 形）、卡片（白底 1px 边框 + 图标行 + 日期行 + 右箭头）、开关（滑块式，开=海军蓝）、复选框（选中=实心蓝底白勾）。

## Layout

### project-library

`pages/project-library/current.png` — 顶栏（舵轮 logo + 品牌名 | 右侧四步步条，步 2 当前）；左栏 240px（「项目」+ 新建项目 主按钮 + 5 个项目项，选中项左缘海军蓝竖条 + 浅底）；中部空态居中（罗盘线稿插画 → 标题「从一块总板开始」→ 说明行 → 主按钮「生成设计系统总板」+ ghost「查看引导」）。

### gallery

`pages/gallery/current.png` — 同壳；中部三段画廊：总板候选(3) / 页面(4) / 组件(2)，卡片=缩略图+名称+徽标，悬停卡右上浮出 3 个 ghost 图标钮（重生成/删除/放大），悬停描边高亮；右栏 320px「详情」表单：名称 input、简报 textarea（36/200 计数）、尺寸 select「1536x1024」、质量 select、底部 重新生成(主) + 设为当前(次)。

### detail

`pages/detail/current.png` — 同壳；中部大预览卡（白底 1px 边框承载总板图，题注 board·anchor）；右栏 320px「操作面板」：设计简报表单（项目名/画布尺寸/品牌简报 0/300/质量）→ 通栏主按钮「生成设计系统总板」→ 进度骨架（灰条 + 「生成中，约需 30~120 秒…」+ ghost「取消」）→ 「历史版本」横排 v4(黄铜描边)/v3/v2/v1。

## Known deviations

- 组件样本图回显色板时 hex 标签漂移（#04A017、"O83D91"）——以总板标签为准，组件图标签不可作为色值来源。
- `pages/detail/current.png` 画布尺寸下拉显示 1920x1080(16:9)，与产品默认 1536x1024 不符。
- 步条标签为模型易错点：gallery 终版第 1 步标签花字；detail 终版顶部渲染成总板分节 tab 而非四步步条。
- 板面圆角映射（按钮 4 / 输入 6 / 卡片 8 / 弹窗 12）与 docs/THEME.md（按钮/输入 8、卡片 12）不一致——两处口径需在 Phase 5 裁决。
- 板面 body=16px，THEME.md 默认正文 13px；图片集未定义暗色模式 token（需向负责人确认，不自行发明）。
