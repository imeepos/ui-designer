---
name: rudder-design
description: >
  Generate consistent, production-grade UI design image sets (design system
  board, app pages, component sheets) with the Rudder CLI driven by the
  gpt-image-2 model. Use when the user asks to design an app/website look,
  create a design system, mock up screens/pages, explore visual directions,
  or produce UI design assets for a coding agent to implement from. Triggers:
  "design the UI", "design system", "mockup screens", "成套界面设计",
  "设计总板", "给这个产品设计一套界面".
---

# Rudder Design Studio

Drive the `rudder` CLI to produce a **style-consistent UI design set**: one
design-system board (the anchor) → every page/component derived from it via
image-edit references, so palette/typography/corner-radius never drift.

## Prerequisites

1. CLI available: `rudder --version` (if missing, see references/cli.md §Install).
2. Credentials in env — never print or store them: `OPENAI_API_KEY`,
   `OPENAI_BASE_URL` (any OpenAI-compatible gpt-image-2 endpoint).
3. Real image calls cost money. Explore with `--dry-run` (free, prints the
   request plan) and `quality low`; use `quality high` only for finals.

## Workflow (run in order; do not skip the anchor)

### 1. Create the project
```bash
rudder init "<Project Name>" --size web --brief "<brand: industry, mood, palette direction, audience>" --dir ./design
```
`--size`: `web` (1536x1024) | `mobile` (1024x1536) | `desktop` (2560x1440) | `WxH`
(constraints: sides multiple of 16, ratio ≤ 3:1, 0.65–8.3 MP). Confirm with
`rudder list --json` (exit 0, `ok:true`).

### 2. Design-system board (the anchor)
```bash
rudder board generate --n 3 --quality low --yes
```
VIEW every PNG under `board/candidates/` (read the image files). Judge:
palette usability, type hierarchy, component samples, CJK text quality.
Pick the best: `rudder board pick <candidate-id>`.
If all are weak, diagnose the brief (too vague / conflicting adjectives),
refine it in `project.json` `styleBrief`, regenerate. Do not proceed without
an anchor — page/component generation exits code 3.

### 3. Pages
```bash
rudder page add dashboard --brief "<info architecture: regions, counts, nav items>"
rudder page generate dashboard --n 2 --quality low --yes
```
Write briefs as **layout structure**, not vibes: name every region and its
content ("top nav 5 items: …, left sidebar with 4 KPI cards: …, main chart
area, right panel task list"). The model renders labeled UI reliably; vague
briefs produce pretty posters that cannot be implemented.
Review `pages/<slug>/candidates/`, then promote the winner:
`rudder page pick <slug> <candidate-id>`. Regenerate with an amended brief to
iterate; history is kept automatically.

### 4. Components
```bash
rudder component add button-set --type buttons --brief "primary/secondary/ghost, 3 states each"
rudder component generate button-set --quality low --yes
```
`--type`: buttons | forms | cards | navigation | icons | tables | modals.
Same review loop: view → `rudder component pick <name> <candidate-id>`.

### 5. Finals, export, DESIGN.md contract
1. Regenerate the chosen anchor + 2-4 key pages with `--quality high`.
2. `rudder export --out ./design-export` → images + `manifest.json` +
   `PROMPTS.md` + `DESIGN.template.md`.
3. VIEW every exported image, then fill `DESIGN.template.md` into a
   `DESIGN.md` (tokens: palette hexes sampled from the board, type scale,
   radius, spacing, component specs). Rules: **every value must come from the
   images; if the images don't define it, ask — never invent.**
   See references/design-md.md for the template walkthrough.

## Hard rules

- Real spends need `--yes`. Default to `--dry-run` while exploring flags.
- `--json` for machine reading: stdout is `{ok, data|error}`; logs go to stderr.
- Never put secrets in commands, logs, or committed files.
- One project = one style universe. Never mix candidates from different
  boards in one export.
- Cost discipline: `low` for exploration, at most 2-3 `high` calls per set.

## Error recovery

| Exit | Meaning | Fix |
|---|---|---|
| 1 | bad arguments | read `hint` in the JSON error, correct flags |
| 2 | API error (429/quota/5xx) | wait 30s and retry once; then reduce `--n` or quality |
| 3 | project state (no anchor, no page) | run the missing earlier step |
| 1/2/3 + `ok:false` | see `error.code` | `code=SIZE_INVALID` → fix dimensions per §1 constraints |

Full command reference: references/cli.md. DESIGN.md guide: references/design-md.md.
