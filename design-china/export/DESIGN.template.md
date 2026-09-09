# DESIGN.md — 舵 · 墨航 Rudder Ink-Voyage（待填模板）

Source: manifest.json (generated 2026-09-09, rudder v0.1.0)

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

### stepper-buttons (navigation)

variants(…) × states(default/hover/disabled/…) — cite `components/stepper-buttons/current.png`

## Layout

### library

项目库三栏屏：左项目列表/中空态水墨帆舟插画+新建/右项目详情表单

cite `pages/library/current.png` — regions, widths, alignment.

### gallery

画廊工作台：四步步条+总板候选网格+右侧生成表单

cite `pages/gallery/current.png` — regions, widths, alignment.

### detail

详情屏：大图预览+血缘缩略图+锚点信息面板

cite `pages/detail/current.png` — regions, widths, alignment.

## Known deviations

- (anything the pages do that the board doesn't define)
