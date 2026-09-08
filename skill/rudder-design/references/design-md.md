# Writing DESIGN.md from exported Rudder assets

`DESIGN.md` is the contract between the design set and the code that will be
written from it. Rudder exports `DESIGN.template.md` with project metadata and
image paths pre-filled; your job is to VIEW the images and fill the token
tables. **Every value must be readable from an image. If the images don't
define a value, ask the user — never invent.**

## Procedure

1. `rudder export --out ./design-export`
2. VIEW `board/anchor.png` first. Extract:
   - palette: sample every swatch (they carry hex labels on the board — use
     those exact values);
   - type scale: heading/body/caption ratios and weights;
   - corner radius per element class (card vs button vs input);
   - spacing feel (padding scale), icon style (stroke vs filled, weight).
3. VIEW each exported page. Extract layout regions and confirm the board
   tokens hold (note deviations under Known deviations).
4. VIEW component sheets. Write per-component specs (variants, states, sizes).
5. Fill the template sections (below). Keep it under 150 lines — it is
   loaded into coding agents' context.

## Template structure

```markdown
# DESIGN.md — <Project>
Source: design-export/manifest.json (generated <date>, rudder v<ver>)

## Tokens
### Palette
| token | hex | usage |
|---|---|---|
| primary | #… | buttons, active nav, links |
| bg | #… | app background |
| … | | (one row per swatch on the board) |

### Typography
family / scale table / weights (from board type hierarchy)

### Geometry
radius: card …px, button …px, input …px · spacing base …px · borders …px

## Components
### Button
variants(primary/secondary/ghost) × states(default/hover/disabled) — cite
`components/button-set/current.png`
### Card / Input / Nav …(same pattern)

## Layout
per page: regions, widths, alignment — cite `pages/<slug>/current.png`

## Known deviations
- (anything the pages do that the board doesn't define)
```

## Discipline

- Colors: hex only, no "slate-ish". If anti-aliasing makes sampling ambiguous,
  sample the board swatch labels, not the pixels.
- Radii/spacing: nearest 4px value.
- The file must let an engineer implement without seeing the images; cite
  image paths for every section anyway.
