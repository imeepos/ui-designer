# DESIGN.md — 舵 Rudder · Studio（待填模板）

Source: manifest.json (generated 2026-09-08, rudder v0.1.0)

> Fill every table ONLY with values readable from the exported images. If the images don't define a value, ask the user — never invent.

## Tokens

### Palette

| token | hex | usage |
|---|---|---|
| primary | #… | buttons, active nav, links |
| bg | #… | app background |
| … | | (one row per swatch on the board) |

### Typography

family: …  
| role | size | weight | notes |
|---|---|---|---|
| display | … | … | |
| heading | … | … | |
| body | … | … | |
| caption | … | … | |

### Geometry

radius: card …px · button …px · input …px · spacing base …px · borders …px

## Components

### button-set (buttons)

variants(…) × states(default/hover/disabled/…) — cite `components/button-set/current.png`

### stepper (navigation)

variants(…) × states(default/hover/disabled/…) — cite `components/stepper/current.png`

## Layout

### project-library

Three-column web app shell, flat, paper-white background. Top bar: helm logo + '舵 Rudder · Studio' left; right side a 4-step stepper: 新建项目 (done, navy check), 设计系统总板 (current, brass dot + bold), 功能页面 (muted), 组件设计 (muted). Left sidebar 240px: header '项目' with primary navy button '新建项目'; 5 project list items (name + muted date), first item active with navy left edge: 舵 Rudder · Studio / 清单 App / 博客主题 / 数据面板 / 空项目. Main area: centered empty state — thin-line compass illustration (navy line art), heading '从一块总板开始', caption '生成设计系统总板，作为全组风格的锚点', primary button '生成设计系统总板' + ghost '查看引导'.

cite `pages/project-library/current.png` — regions, widths, alignment.

### gallery

Same app shell as library page (top bar + 4-step stepper, step 功能页面 current. The stepper MUST use EXACTLY these four labels in this order: 1 新建项目 (done), 2 设计系统总板 (done), 3 功能页面 (current, brass dot, bold), 4 组件设计 (future, muted) — never rename or invent other step names). Left sidebar 240px project list. Main center: gallery grid with three labeled sections stacked vertically: '总板候选' (3 board thumbnail cards in a row), '页面' (2x2 grid of page thumbnails), '组件' (2 component thumbnails); each card = white thumbnail, 1px border, name + badge below; the hovered card shows small ghost icon buttons floating at its top-right corner: 重新生成 / 删除 / 放大. Right panel 320px detail form: title '详情', inputs: 名称, 简报 textarea, 尺寸 select '1536x1024', 质量 select; bottom buttons: primary '重新生成' + secondary '设为当前'.

cite `pages/gallery/current.png` — regions, widths, alignment.

### detail

Same app shell (top bar + 4-step stepper, step 设计系统总板 current). Left sidebar 240px project list. Main center: large preview card (white, 1px border) showing a design-system board image with caption 'board · anchor'. Right panel 320px operation panel titled '操作面板': section '设计简报' form — 项目名 input, 画布尺寸 select, 品牌简报 textarea (3 rows placeholder), 质量 select; full-width primary button '生成设计系统总板'; below: loading skeleton of gray bars with muted caption '生成中，约需 30~120 秒…' and ghost '取消' button; bottom section '历史版本': horizontal strip of 4 small version thumbnails labeled v4 v3 v2 v1, v4 with brass border.

cite `pages/detail/current.png` — regions, widths, alignment.

## Known deviations

- (anything the pages do that the board doesn't define)
