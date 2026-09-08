# PROMPTS.md — 舵 Rudder · Studio

Full prompt/parameter log, newest last. Project `舵 Rudder · Studio` (id `f20ffa15-441f-4ef5-8365-2ccb6a85056d`), canvas 1536x1024.

## 1. board — generations

- at: `2026-09-08T19:24:31.069Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`low` n=3 thinking=`medium`
- candidates: 0001, 0002, 0003

```
Purpose: a UI design system board (风格总板) for the product "舵 Rudder · Studio".
Style brief: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Canvas: 1536x1024. Lay out ONE labeled sheet with these sections:
1. COLOR PALETTE — 5-8 swatch chips as rounded rectangles, each with its exact hex code printed as a label under or inside the chip.
2. TYPOGRAPHY — type hierarchy specimen: display / heading / subheading / body / caption rows, each labeled with size and weight.
3. COMPONENT SAMPLES — a row of buttons (primary, secondary, ghost; default/hover/disabled states), one text input, one select, one card, one toggle, one checkbox, labeled.
4. ICON STYLE — a row of 6-8 sample icons showing stroke weight and corner treatment.
5. SPACING & GRID RULES — a spacing scale diagram (4/8/16/24/32px) and corner-radius samples per element class.
Sections separated by thin dividers, generous margins, engineering spec-sheet clarity.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 2. page-add `project-library` — -

- at: `2026-09-08T19:26:00.175Z`
- params: model=`-` size=`-` quality=`-` n=0

```
[page-add] project-library
```

## 3. page-add `gallery` — -

- at: `2026-09-08T19:26:00.190Z`
- params: model=`-` size=`-` quality=`-` n=0

```
[page-add] gallery
```

## 4. page-add `detail` — -

- at: `2026-09-08T19:26:00.205Z`
- params: model=`-` size=`-` quality=`-` n=0

```
[page-add] detail
```

## 5. page `project-library` — edits

- at: `2026-09-08T19:26:51.339Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`low` n=2 thinking=`medium`
- candidates: 0001, 0002

```
Image 1 是本产品设计系统总板。严格沿用它的设计语言，只组合新的页面内容。
Task: design the full "project-library" page as one high-fidelity UI mockup.
Layout brief: Three-column web app shell, flat, paper-white background. Top bar: helm logo + '舵 Rudder · Studio' left; right side a 4-step stepper: 新建项目 (done, navy check), 设计系统总板 (current, brass dot + bold), 功能页面 (muted), 组件设计 (muted). Left sidebar 240px: header '项目' with primary navy button '新建项目'; 5 project list items (name + muted date), first item active with navy left edge: 舵 Rudder · Studio / 清单 App / 博客主题 / 数据面板 / 空项目. Main area: centered empty state — thin-line compass illustration (navy line art), heading '从一块总板开始', caption '生成设计系统总板，作为全组风格的锚点', primary button '生成设计系统总板' + ghost '查看引导'.
Invariants (do NOT change): strictly reuse Image 1's exact
- color palette (same hex values for primary/background/text/accent),
- typography family, sizes and weights,
- corner radii and border treatment,
- component styling (buttons, inputs, cards), icon style and stroke weight,
- spacing rhythm and density.
Overall mood stays: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Only compose NEW layout/content; never redesign the system.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 6. page `gallery` — edits

- at: `2026-09-08T19:28:24.241Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`low` n=2 thinking=`medium`
- candidates: 0001, 0002

```
Image 1 是本产品设计系统总板。严格沿用它的设计语言，只组合新的页面内容。
Task: design the full "gallery" page as one high-fidelity UI mockup.
Layout brief: Same app shell as library page (top bar + 4-step stepper, step 功能页面 current). Left sidebar 240px project list. Main center: gallery grid with three labeled sections stacked vertically: '总板候选' (3 board thumbnail cards in a row), '页面' (2x2 grid of page thumbnails), '组件' (2 component thumbnails); each card = white thumbnail, 1px border, name + badge below; the hovered card shows small ghost icon buttons floating at its top-right corner: 重新生成 / 删除 / 放大. Right panel 320px detail form: title '详情', inputs: 名称, 简报 textarea, 尺寸 select '1536x1024', 质量 select; bottom buttons: primary '重新生成' + secondary '设为当前'.
Invariants (do NOT change): strictly reuse Image 1's exact
- color palette (same hex values for primary/background/text/accent),
- typography family, sizes and weights,
- corner radii and border treatment,
- component styling (buttons, inputs, cards), icon style and stroke weight,
- spacing rhythm and density.
Overall mood stays: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Only compose NEW layout/content; never redesign the system.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 7. page `detail` — edits

- at: `2026-09-08T19:29:22.723Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`low` n=2 thinking=`medium`
- candidates: 0001, 0002

```
Image 1 是本产品设计系统总板。严格沿用它的设计语言，只组合新的页面内容。
Task: design the full "detail" page as one high-fidelity UI mockup.
Layout brief: Same app shell (top bar + 4-step stepper, step 设计系统总板 current). Left sidebar 240px project list. Main center: large preview card (white, 1px border) showing a design-system board image with caption 'board · anchor'. Right panel 320px operation panel titled '操作面板': section '设计简报' form — 项目名 input, 画布尺寸 select, 品牌简报 textarea (3 rows placeholder), 质量 select; full-width primary button '生成设计系统总板'; below: loading skeleton of gray bars with muted caption '生成中，约需 30~120 秒…' and ghost '取消' button; bottom section '历史版本': horizontal strip of 4 small version thumbnails labeled v4 v3 v2 v1, v4 with brass border.
Invariants (do NOT change): strictly reuse Image 1's exact
- color palette (same hex values for primary/background/text/accent),
- typography family, sizes and weights,
- corner radii and border treatment,
- component styling (buttons, inputs, cards), icon style and stroke weight,
- spacing rhythm and density.
Overall mood stays: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Only compose NEW layout/content; never redesign the system.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 8. page `gallery` — edits

- at: `2026-09-08T19:32:06.213Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`high` n=1 thinking=`medium`
- candidates: 0003

```
Image 1 是本产品设计系统总板。严格沿用它的设计语言，只组合新的页面内容。
Task: design the full "gallery" page as one high-fidelity UI mockup.
Layout brief: Same app shell as library page (top bar + 4-step stepper, step 功能页面 current. The stepper MUST use EXACTLY these four labels in this order: 1 新建项目 (done), 2 设计系统总板 (done), 3 功能页面 (current, brass dot, bold), 4 组件设计 (future, muted) — never rename or invent other step names). Left sidebar 240px project list. Main center: gallery grid with three labeled sections stacked vertically: '总板候选' (3 board thumbnail cards in a row), '页面' (2x2 grid of page thumbnails), '组件' (2 component thumbnails); each card = white thumbnail, 1px border, name + badge below; the hovered card shows small ghost icon buttons floating at its top-right corner: 重新生成 / 删除 / 放大. Right panel 320px detail form: title '详情', inputs: 名称, 简报 textarea, 尺寸 select '1536x1024', 质量 select; bottom buttons: primary '重新生成' + secondary '设为当前'.
Invariants (do NOT change): strictly reuse Image 1's exact
- color palette (same hex values for primary/background/text/accent),
- typography family, sizes and weights,
- corner radii and border treatment,
- component styling (buttons, inputs, cards), icon style and stroke weight,
- spacing rhythm and density.
Overall mood stays: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Only compose NEW layout/content; never redesign the system.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 9. component-add `button-set` — -

- at: `2026-09-08T19:33:29.827Z`
- params: model=`-` size=`-` quality=`-` n=0

```
[component-add] button-set
```

## 10. component-add `stepper` — -

- at: `2026-09-08T19:33:29.840Z`
- params: model=`-` size=`-` quality=`-` n=0

```
[component-add] stepper
```

## 11. component `button-set` — edits

- at: `2026-09-08T19:34:29.268Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`low` n=1 thinking=`medium`
- candidates: 0001

```
Image 1 是本产品设计系统总板。严格沿用它的设计语言，为单个组件族出细节图。
Task: a component detail sheet for the "buttons" family: Component sheet on paper-white background, one labeled row per variant × state grid. Variants: Primary / Secondary / Ghost. States: Default / Hover / Disabled. Chinese labels inside buttons: 主要按钮 / 次要按钮 / 幽灵按钮. Primary = solid navy #0B3D91 white text; Secondary = white with 1px navy border navy text; Ghost = no border muted text. Hover rows slightly darker/lighter; disabled = muted gray fill, low-contrast text. Include size row (sm 28px / md 32px / lg 40px heights) and corner radius 8px. Thin-line icons allowed. Spec-sheet clarity with labels.
Show every variant and state in a tidy grid, each labeled (e.g. default / hover / active / disabled / focus).
Invariants (do NOT change): strictly reuse Image 1's exact
- color palette (same hex values for primary/background/text/accent),
- typography family, sizes and weights,
- corner radii and border treatment,
- component styling (buttons, inputs, cards), icon style and stroke weight,
- spacing rhythm and density.
Overall mood stays: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Only compose NEW layout/content; never redesign the system.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 12. component `stepper` — edits

- at: `2026-09-08T19:35:28.537Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`low` n=1 thinking=`medium`
- candidates: 0001

```
Image 1 是本产品设计系统总板。严格沿用它的设计语言，为单个组件族出细节图。
Task: a component detail sheet for the "navigation" family: Component sheet: a horizontal 4-step stepper specimen on paper-white background, three state rows. Steps labeled in Chinese: 1 新建项目, 2 设计系统总板, 3 功能页面, 4 组件设计, connected by thin lines. Row A (完成态): steps 1-2 done = navy circle with white check + navy label. Row B (当前态): step 3 current = brass/amber #D4A017 filled dot + bold navy label, others muted. Row C (未来态): all steps muted gray outline circles. Also show a compact variant for top bars (dot + label inline) and size specs (circle 20px, gap 8px, connector 1px).
Show every variant and state in a tidy grid, each labeled (e.g. default / hover / active / disabled / focus).
Invariants (do NOT change): strictly reuse Image 1's exact
- color palette (same hex values for primary/background/text/accent),
- typography family, sizes and weights,
- corner radii and border treatment,
- component styling (buttons, inputs, cards), icon style and stroke weight,
- spacing rhythm and density.
Overall mood stays: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Only compose NEW layout/content; never redesign the system.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

## 13. board — edits

- at: `2026-09-08T19:36:43.521Z`
- params: model=`gpt-image-2` size=`1536x1024` quality=`high` n=1 thinking=`medium`
- candidates: 0004

```
Purpose: a UI design system board (风格总板) for the product "舵 Rudder · Studio".
Style brief: Deep-sea nautical design studio for a professional developer tool called Rudder (Chinese: 舵). Navy blue primary color, brass/amber accent used sparingly, large neutral grays carrying content, warm paper-white background. Calm, restrained, professional instrument feel — like a ship's bridge console. Flat design, no gradients, no heavy shadows, 1px borders preferred. Typography Inter + PingFang SC. The design-system board must show: hex-labeled color swatches, a type scale specimen (Inter/PingFang), button / card / input-field component samples, thin-line icon style, and 8px spacing grid annotations.
Canvas: 1536x1024. Lay out ONE labeled sheet with these sections:
1. COLOR PALETTE — 5-8 swatch chips as rounded rectangles, each with its exact hex code printed as a label under or inside the chip.
2. TYPOGRAPHY — type hierarchy specimen: display / heading / subheading / body / caption rows, each labeled with size and weight.
3. COMPONENT SAMPLES — a row of buttons (primary, secondary, ghost; default/hover/disabled states), one text input, one select, one card, one toggle, one checkbox, labeled.
4. ICON STYLE — a row of 6-8 sample icons showing stroke weight and corner treatment.
5. SPACING & GRID RULES — a spacing scale diagram (4/8/16/24/32px) and corner-radius samples per element class.
Sections separated by thin dividers, generous margins, engineering spec-sheet clarity.
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, no photorealistic scenes, no 3D perspective, no device frames beyond the canvas itself.
```

