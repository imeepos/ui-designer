# Rudder CLI Reference

Agent-facing contract of every command. Human-readable one-line summaries go
to stderr; pass `--json` to get machine-readable data on stdout.

## Install

- macOS/Linux (project-local, preferred): `pnpm add -g @tauri-apps/cli` is the
  Tauri app CLI, NOT rudder. The rudder CLI ships with the Rudder desktop app
  or via `cargo install --path crates/rudder-cli` from the repo root. Verify:
  `rudder --version`.
- Requires `OPENAI_API_KEY` and optionally `OPENAI_BASE_URL` in env.

## Global flags (all commands)

| Flag | Effect |
|---|---|
| `--project <path>` | target project dir (default: cwd if it is a project, else last used) |
| `--json` | stdout becomes `{ok: true, data: …}` or `{ok: false, error: {code, message, hint}}` |
| `--dry-run` | print the request plan (endpoint, params, payload shape), no network, free |
| `--yes` | REQUIRED for any real image API spend; without it generate-commands are dry-runs |

Exit codes: `0` ok · `1` bad arguments (see `hint`) · `2` API error
(429/quota/5xx — back off 30s, retry once, then lower `--n`/quality) ·
`3` project state error (e.g. no anchor picked yet).

Defaults (unless overridden by flags or `rudder config`): board `--n 4`,
page/component `--n 1`, `--quality high`, `--thinking medium`. Candidate ids
are the `NNNN` file stems; `rudder board pick 0001.png` (an `ls` listing) and
`rudder board pick 0001` address the same candidate.

## Commands

### `rudder init <name> [--size web|mobile|desktop|WxH] [--dir <path>] [--brief "..."]`
Create a project. Size presets: `web`=1536x1024, `mobile`=1024x1536,
`desktop`=2560x1440. Custom `WxH` constraints: each side multiple of 16,
aspect ≤ 3:1, total pixels 0.65M–8.3M. `--brief` seeds `styleBrief`.
Data: `{projectId, dir, canvasSize}`.

### `rudder board generate [--n 1-4] [--quality low|medium|high] [--seed <int>] [--thinking low|medium|high] [--ref <img>...]`
Generate design-system board candidates (`board/candidates/NNNN.png`).
Without `--ref` this uses the generations endpoint; any `--ref` images switch
it to the edits endpoint with those images attached.
Anchor mechanics: one board defines palette, type scale, corner radius,
component samples, icon style for the WHOLE set. Data:
`{candidates: [{id, file, prompt, seed, size, quality}]}`.

### `rudder board pick <candidate-id>`
Copy candidate to `board/anchor.png` and lock it in `project.json`.

### `rudder page add <slug> --brief "..."` / `rudder page list`
Register a page (slug: `a-z0-9-`). Brief = layout structure (regions, counts,
labels), not adjectives.

### `rudder page generate <slug|--all> [--n 1-4] [--quality ...] [--yes] [--ref <img>...]`
Anchor-backed generation into `pages/<slug>/candidates/`. `--ref` adds extra
layout reference images (they are passed after the anchor). Promote with
`rudder page pick <slug> <candidate-id>` → `pages/<slug>/current.png`
(previous current moves to `history/`).

### `rudder component add <name> --type <t> --brief "..."`
### `rudder component generate <name|--all> [--n 1-4] ...` / `rudder component pick <name> <candidate-id>`
Same lifecycle as pages, under `components/<name>/`. `--type` is an open
label rendered into the prompt; common values: buttons | forms | cards |
navigation | icons | tables | modals.

### `rudder list [pages|components]`
Status overview: anchor present? pages/components with candidate counts.

### `rudder export [--out <dir>]`
Bundle: `board/`, `pages/`, `components/` (currents), `manifest.json`
(full prompt/model/seed/size lineage), `PROMPTS.md`, `DESIGN.template.md`.

### `rudder config get <key>` / `rudder config set <key> <value>`
Defaults: `quality`, `thinking`, `n`. Stored in `~/Rudder/config.json`
(directory overridable with `RUDDER_HOME`).
Secrets are NEVER stored — env only.

### `rudder e2e [--quality low] [--yes]`
Self-test: sample project → board → 1 page → 1 component → export. Use to
verify the toolchain before real work. Without `--yes` (or with `--dry-run`)
it prints the per-step request plans and creates only the sample project —
free, no network.

## Project layout on disk

```
<project>/
├── project.json
├── board/candidates/NNNN.png · board/anchor.png
├── pages/<slug>/{candidates/,current.png,history/}
├── components/<name>/{candidates/,current.png,history/}
└── refs/
```

`project.json` is the source of truth; edit `styleBrief`/page `brief` fields
freely between generations. Write is atomic (tempfile+rename); don't hand-edit
while a generation is running.
