import boardTemplateJson from "../../../templates/board-design-system.json";
import componentTemplateJson from "../../../templates/component-sheet-grid.json";
import pageTemplateJson from "../../../templates/page-ui-standard.json";

/**
 * Frontend port of the prompt engine (crates/rudder-core/src/prompt.rs +
 * templates.rs) for the SDK-direct path: the webview composes the exact prompt
 * it sends, so `record_generated_image` can store it as the `engine` source
 * lineage. The port mirrors the Rust engine:
 * - templates stay the single source of truth (`/templates/*.json`, imported
 *   verbatim — skeletons are never duplicated in TS);
 * - `fill` drops lines whose slots ALL resolve empty (optional briefs);
 * - the constraint table, verbatim labels and invariants render identically.
 */

// ---------------------------------------------------------------------------
// Template JSON shape (templates.rs `Template`, only what filling needs)
// ---------------------------------------------------------------------------

export interface TemplateJson {
  id: string;
  appliesTo: string;
  skeleton: string;
}

const BOARD_TEMPLATE = boardTemplateJson as TemplateJson;
const PAGE_TEMPLATE = pageTemplateJson as TemplateJson;
const COMPONENT_TEMPLATE = componentTemplateJson as TemplateJson;

/** The literal anchor reference every page/component prompt starts with. */
// Mirrored engine constant (prompt.rs ANCHOR_REFERENCE); model-facing
// prompt text, not UI copy. lint:text allowlisted below.
export const ANCHOR_REFERENCE = "Image 1 是本产品设计系统总板";

export type PromptKind = "board" | "page" | "component";

// ---------------------------------------------------------------------------
// Constraint table (prompt.rs CONSTRAINTS, from KNOWLEDGE §7 pitfall list)
// ---------------------------------------------------------------------------

interface Constraint {
  id: string;
  applies: readonly string[];
  /** English rule text; `{w}`/`{h}` in `canvas-locked` are substituted. */
  text: string;
}

export const CONSTRAINTS: readonly Constraint[] = [
  {
    id: "text-hardcode",
    applies: ["board", "page", "component"],
    text: "every visible label is real UI text spelled exactly as briefed; CJK glyphs must be correct — no lorem ipsum, no gibberish glyphs, no placeholder boxes",
  },
  {
    id: "explicit-negatives",
    applies: ["board", "page", "component"],
    text: "forbidden: watermarks, invented brand logos, random decorative icons, gibberish text",
  },
  {
    id: "canvas-locked",
    applies: ["board", "page", "component"],
    text: "compose for exactly {w}x{h} and keep every element inside the canvas",
  },
  {
    id: "explicit-counts",
    applies: ["board", "page", "component"],
    text: "every repeated element (nav items, cards, buttons, icons, sections) appears exactly as many times as the brief states — never invent or drop items",
  },
  {
    id: "consistency-first",
    applies: ["page", "component"],
    text: "consistency beats novelty: reuse the anchor's palette, typography, corner radii, component styling and spacing before adding anything new",
  },
  {
    id: "board-cohesion",
    applies: ["board"],
    text: "all sections share one palette and one type system; the sheet reads as a single spec page, not a collage",
  },
  {
    id: "identity-lock",
    applies: ["component"],
    text: "all cells show the same component family: identical geometry and styling; only the labeled state or variant changes between cells",
  },
  {
    id: "flat-ui",
    applies: ["board", "page", "component"],
    text: "flat vector UI mockup, high fidelity, crisp edges, generous whitespace; no photorealistic scenes, no 3D perspective, no device frames beyond the canvas",
  },
  {
    id: "single-output",
    applies: ["board", "page", "component"],
    text: "render exactly ONE finished sheet — no moodboard, no multiple alternative concepts, no process or comparison shots",
  },
  {
    id: "brand-accent-only",
    applies: ["page", "component"],
    text: "brand color works as accents (lines, labels, icons, buttons), not full-bleed color fields",
  },
  {
    id: "style-feature-not-name",
    applies: ["board", "page", "component"],
    text: "interpret style references through their features (palette, stroke, mood); never reproduce a named artwork's composition",
  },
  {
    id: "small-size-legibility",
    applies: ["board", "component"],
    text: "labels stay legible at small sizes; prefer short labels over dense micro-copy",
  },
];

/** Project data the constraint block and templates read. */
export interface PromptProject {
  name: string;
  brandBrief: string;
  styleBrief: string;
  /** Project-level negative hints (`explicit-negatives` extension). */
  negativeHints?: string[];
  canvasW: number;
  canvasH: number;
}

/** Render the `Constraints:` block appended to every engine prompt. */
export function renderConstraints(kind: PromptKind, project: PromptProject): string {
  const lines = ["Constraints:"];
  for (const constraint of CONSTRAINTS) {
    if (!constraint.applies.includes(kind)) continue;
    let text = constraint.text;
    if (constraint.id === "canvas-locked") {
      text = text
        .replaceAll("{w}", String(project.canvasW))
        .replaceAll("{h}", String(project.canvasH));
    }
    const negatives = project.negativeHints ?? [];
    if (constraint.id === "explicit-negatives" && negatives.length > 0) {
      text += `; additionally forbidden per project: ${negatives.join("; ")}`;
    }
    lines.push(`- ${constraint.id}: ${text}`);
  }
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Verbatim labels (`Labels: a|b|c` lines inside briefs)
// ---------------------------------------------------------------------------

/**
 * Extract verbatim labels from `Labels: a|b|c` markers and return the brief
 * text with those markers stripped (prompt.rs `extract_verbatim_labels`).
 */
export function extractVerbatimLabels(brief: string): { labels: string[]; kept: string } {
  const marker = "Labels:";
  const labels: string[] = [];
  const kept: string[] = [];
  for (const line of brief.split("\n")) {
    const pos = line.indexOf(marker);
    if (pos === -1) {
      kept.push(line);
      continue;
    }
    const head = line.slice(0, pos).trimEnd();
    if (head.trim().length > 0) kept.push(head);
    for (const part of line.slice(pos + marker.length).split("|")) {
      const value = part.trim();
      if (value.length > 0 && !labels.includes(value)) labels.push(value);
    }
  }
  return { labels, kept: kept.join("\n").trim() };
}

function verbatimConstraint(labels: string[]): string {
  const quoted = labels.map((label) => `"${label}"`).join(" ");
  return (
    "Verbatim labels: the following must appear exactly as written, " +
    "character-for-character, one per element — " +
    `${quoted}; never translate, reword, add or drop them.`
  );
}

// ---------------------------------------------------------------------------
// Invariants (prompt.rs `invariants` — the set-consistency core)
// ---------------------------------------------------------------------------

function renderInvariants(styleBrief: string): string {
  const lines = [
    "Invariants (do NOT change): strictly reuse Image 1's exact",
    "- color palette (same hex values for primary/background/text/accent),",
    "- typography family, sizes and weights,",
    "- corner radii and border treatment,",
    "- component styling (buttons, inputs, cards), icon style and stroke weight,",
    "- spacing rhythm and density.",
  ];
  const mood = styleBrief.trim();
  if (mood.length > 0) lines.push(`Overall mood stays: ${mood}`);
  lines.push("Only compose NEW layout/content; never redesign the system.");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Template filling (templates.rs `fill`)
// ---------------------------------------------------------------------------

/** Extract `{slot}` tokens (`[A-Za-z0-9_]+`); other braces pass through. */
export function extractSlots(text: string): string[] {
  const slots: string[] = [];
  const pattern = /\{([A-Za-z0-9_.]+)\}/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text)) !== null) {
    slots.push(match[1]);
  }
  return slots;
}

type Vars = Record<string, string>;

/**
 * Fill a skeleton following templates.rs rules: unknown slot → error;
 * a line whose slots ALL resolve empty is dropped; 3+ newline runs collapse
 * to one blank line; the result is trimmed.
 */
export function fillTemplate(template: TemplateJson, vars: Vars): string {
  const lines: string[] = [];
  for (const line of template.skeleton.split("\n")) {
    const slots = extractSlots(line);
    let rendered = line;
    let allEmpty = slots.length > 0;
    for (const slot of slots) {
      const value = vars[slot];
      if (value === undefined) {
        throw new Error(
          `template \`${template.id}\` uses slot \`{${slot}}\` which is not available here`,
        );
      }
      if (value.trim().length > 0) allEmpty = false;
      rendered = rendered.replaceAll(`{${slot}}`, value);
    }
    if (allEmpty) continue;
    lines.push(rendered);
  }
  return lines.join("\n").replaceAll(/\n{3,}/g, "\n\n").trim();
}

// ---------------------------------------------------------------------------
// Composition (prompt.rs compose_board / compose_page / compose_component)
// ---------------------------------------------------------------------------

function baseVars(project: PromptProject): Vars {
  return {
    "project.name": project.name.trim(),
    "project.brandBrief": project.brandBrief.trim(),
    "project.styleBrief": project.styleBrief.trim(),
    "project.negativeHints": (project.negativeHints ?? []).join("; "),
    "project.invariants": renderInvariants(project.styleBrief),
    "canvas.w": String(project.canvasW),
    "canvas.h": String(project.canvasH),
  };
}

/** Compose the design-system board prompt (ARCHITECTURE §5.1). */
export function composeBoardPrompt(project: PromptProject, template?: TemplateJson): string {
  const prompt = fillTemplate(template ?? BOARD_TEMPLATE, baseVars(project));
  return `${prompt}\n${renderConstraints("board", project)}`;
}

export interface PromptPage {
  slug: string;
  brief: string;
}

/** Compose a page edit prompt: anchor reference + brief + invariants. */
export function composePagePrompt(
  project: PromptProject,
  page: PromptPage,
  template?: TemplateJson,
): string {
  const { labels, kept } = extractVerbatimLabels(page.brief.trim());
  const vars: Vars = {
    ...baseVars(project),
    "page.slug": page.slug,
    "page.brief": kept,
    "anchor.reference": ANCHOR_REFERENCE,
  };
  let prompt = fillTemplate(template ?? PAGE_TEMPLATE, vars);
  if (labels.length > 0) prompt += `\n${verbatimConstraint(labels)}`;
  return `${prompt}\n${renderConstraints("page", project)}`;
}

export interface PromptComponent {
  name: string;
  /** Component type label (`buttons` …), the `kind` slot of the skeleton. */
  kind: string;
  brief: string;
}

/** Compose a component edit prompt (ARCHITECTURE §5.2). */
export function composeComponentPrompt(
  project: PromptProject,
  component: PromptComponent,
  template?: TemplateJson,
): string {
  const { labels, kept } = extractVerbatimLabels(component.brief.trim());
  const vars: Vars = {
    ...baseVars(project),
    "component.name": component.name,
    "component.kind": component.kind.trim(),
    "component.brief": kept,
    "anchor.reference": ANCHOR_REFERENCE,
  };
  let prompt = fillTemplate(template ?? COMPONENT_TEMPLATE, vars);
  if (labels.length > 0) prompt += `\n${verbatimConstraint(labels)}`;
  return `${prompt}\n${renderConstraints("component", project)}`;
}
