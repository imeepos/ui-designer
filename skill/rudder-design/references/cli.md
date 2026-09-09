# Rudder CLI Reference

Agent-facing contract of every command. Human mode prints a one-line summary
to **stdout** (success at a glance; dry-run plans, warnings and errors go to
stderr); pass `--json` to get the machine-readable envelope on stdout.

## Install

- macOS/Linux (project-local, preferred): `pnpm add -g @tauri-apps/cli` is the
  Tauri app CLI, NOT rudder. The rudder CLI ships with the Rudder desktop app
  or via `cargo install --path crates/rudder-cli` from the repo root. Verify:
  `rudder --version`.
- Credentials: env `OPENAI_API_KEY` (wins) or the OS keychain — see
  `rudder config` below. `OPENAI_BASE_URL` env is optional.

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
page/component `--n 1`, `--quality low` (exploration tier — `high` must be
opted into explicitly with `--quality high`), `--thinking medium`. `--seed`
is auto-generated and recorded when omitted (plan, project.json lineage,
candidate rows, manifest all carry it), so any batch can be replayed with
`--seed <recorded>`. Candidate ids are the `NNNN` file stems;
`rudder board pick 0001.png` (an `ls` listing) and `rudder board pick 0001`
address the same candidate.

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
component samples, icon style for the WHOLE set. Data (same envelope for
every generate command):
`{dryRun, kind: "board", target: "", plan, candidates: [{id, file, seed, size, quality}]}`.
The full prompt lives ONCE in `plan.params.prompt`; candidate rows reference
it by `id`/`file`.

### `rudder board pick <candidate-id>`
Copy candidate to `board/anchor.png` and lock it in `project.json`.

### `rudder project update [--name <n>] [--brand-brief <s>] [--style-brief <s>]`
Amend project metadata between generations (at least one flag). This is the
supported way to iterate on briefs — no hand-editing of `project.json`.

### `rudder page add <slug> --brief "..."` / `rudder page list`
Register a page (slug: `a-z0-9-`). Brief = layout structure (regions, counts,
labels), not adjectives.

### `rudder page update <slug> --brief "..."`
Replace a page's layout brief, then `page generate` to iterate.

### `rudder page generate <slug|--all> [--n 1-4] [--quality ...] [--yes] [--ref <img>...]`
Anchor-backed generation into `pages/<slug>/candidates/`. `--ref` adds extra
layout reference images (they are passed after the anchor). Promote with
`rudder page pick <slug> <candidate-id>` → `pages/<slug>/current.png`
(previous current moves to `history/`).
Envelope: a single target answers with the exact board shape (top-level
`candidates`); only `--all` wraps per-target entries as
`{dryRun, kind: "page", results: [{kind, target, plan, candidates}]}`.

### `rudder component add <name> --type <t> --brief "..."`
### `rudder component update <name> [--type <t>] [--brief "..."]`
### `rudder component generate <name|--all> [--n 1-4] ...` / `rudder component pick <name> <candidate-id>`
Same lifecycle as pages, under `components/<name>/`. `--type` is an open
label rendered into the prompt; common values: buttons | forms | cards |
navigation | icons | tables | modals.

### `rudder list [pages|components]`
Status overview: anchor present? pages/components with candidate counts.

### `rudder export [--out <dir>] [--with-candidates]`
Bundle (default): `board/anchor.png`, `pages/<slug>/current.png`,
`components/<name>/current.png`, `manifest.json` (full
prompt/model/seed/size lineage), `PROMPTS.md`, `DESIGN.template.md`.
Board exploration candidates are NOT included unless `--with-candidates` is
passed. Targets that were generated but never picked are excluded from the
bundle; each one produces a `warning:` line on stderr and an entry in the
`--json` `data.warnings` array — pick them (`board pick` / `page pick` /
`component pick`) before exporting a final set.

### `rudder config get <key>` / `rudder config set <key> <value>`
Defaults: `quality`, `thinking`, `n`, `base_url`. Stored in
`~/Rudder/config.json` (directory overridable with `RUDDER_HOME`).
Secrets are NEVER stored in config files.

### `rudder config set api-key` (stdin) · `rudder config clear api-key` · `rudder config test`
- `echo <key> | rudder config set api-key` stores the key in the OS keychain
  (service `rudder`, account `openai-api-key`). The value is read from stdin
  ONLY — never pass it as an argument (shell history). Output shows at most
  the tail 4 characters.
- `rudder config clear api-key` removes the stored key (idempotent).
- `rudder config test` prints the resolved base URL and key source
  (`env` | `keychain` | `none`); when a key resolves it also probes
  `GET {base}/v1/models` (free) and prints the available model count.
  Exit 2 with an error envelope when unreachable / unauthorized /
  `gpt-image-2` not served. Env keys win over the keychain; set
  `RUDDER_KEYCHAIN=0` to ignore the keychain entirely (CI/hermetic runs).

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

`project.json` is the source of truth; amend briefs with the `update`
subcommands (`rudder project update --style-brief …`, `rudder page update …`,
`rudder component update …`) between generations. Write is atomic
(tempfile+rename); don't hand-edit while a generation is running.
