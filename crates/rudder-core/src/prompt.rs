//! Prompt engine (ARCHITECTURE §5): three-part templates that make a whole
//! set style-consistent.
//!
//! 1. Board template — a structured "UI design system board" sheet: palette
//!    (with hex labels), type scale, component samples, icon style, spacing.
//! 2. Page/component edit template — opens with the anchor reference line
//!    「Image 1 是本产品设计系统总板」 and carries the invariant list.
//! 3. Every generation records the final prompt + params into `promptLog`.
//!
//! Assembly paths (PRD §0 产品边界 — the tool itself ships no intelligence):
//! - engine path: a [`templates::Template`] skeleton (default or
//!   `--template <id>`) is filled from project data, then the constraint
//!   table below is appended automatically (dry-run shows the full text);
//! - agent path (`--prompt-file`): the file content IS the prompt — nothing
//!   is injected; the external coding agent is fully responsible.
//!
//! The constraint table distills docs/GPT-IMAGE-2-DESIGN-KNOWLEDGE.md §七
//! (15 条防坑清单) into structured rows (rule → applicable products); the old
//! RENDER_RULES paragraph was merged into it.

use crate::error::Result;
use crate::store::{Component, GenParams, Page, Project, PromptLogEntry};
use crate::templates::{self, Template, Vars};
use std::collections::BTreeMap;

/// The literal anchor reference every page/component prompt starts with
/// (docs/ARCHITECTURE.md §5).
pub const ANCHOR_REFERENCE: &str = "Image 1 是本产品设计系统总板";

/// Prompt source recorded in lineage: engine-assembled prompt.
pub const SOURCE_ENGINE: &str = "engine";
/// Prompt source recorded in lineage: agent-authored prompt file.
pub const SOURCE_AGENT_FILE: &str = "agent-file";

// ---------------------------------------------------------------------------
// Constraint table (KNOWLEDGE §七 防坑清单 → rule × product kind)
// ---------------------------------------------------------------------------

/// One structured constraint row. `applies` lists the product kinds the row
/// is injected for; an empty list marks an advisory rule that is kept for
/// documentation but never injected (not applicable to UI sheet renders).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub id: &'static str,
    pub applies: &'static [&'static str],
    /// English rule text; `{w}`/`{h}` in `canvas-locked` are substituted.
    pub text: &'static str,
    /// Pointer into docs/GPT-IMAGE-2-DESIGN-KNOWLEDGE.md.
    pub source: &'static str,
}

/// The full table, in KNOWLEDGE §七 order. `flat-ui` carries the merged
/// legacy RENDER_RULES paragraph.
pub const CONSTRAINTS: &[Constraint] = &[
    Constraint {
        id: "text-hardcode",
        applies: &["board", "page", "component"],
        text: "every visible label is real UI text spelled exactly as briefed; CJK glyphs \
               must be correct — no lorem ipsum, no gibberish glyphs, no placeholder boxes",
        source: "KNOWLEDGE §七-1 文字硬编码",
    },
    Constraint {
        id: "explicit-negatives",
        applies: &["board", "page", "component"],
        text: "forbidden: watermarks, invented brand logos, random decorative icons, \
               gibberish text",
        source: "KNOWLEDGE §七-2 负面约束具体化",
    },
    Constraint {
        id: "canvas-locked",
        applies: &["board", "page", "component"],
        text: "compose for exactly {w}x{h} and keep every element inside the canvas",
        source: "KNOWLEDGE §七-3 比例前置锁定",
    },
    Constraint {
        id: "explicit-counts",
        applies: &["board", "page", "component"],
        text: "every repeated element (nav items, cards, buttons, icons, sections) appears \
               exactly as many times as the brief states — never invent or drop items",
        source: "KNOWLEDGE §七-4 数量显式化",
    },
    Constraint {
        id: "consistency-first",
        applies: &["page", "component"],
        text: "consistency beats novelty: reuse the anchor's palette, typography, corner \
               radii, component styling and spacing before adding anything new",
        source: "KNOWLEDGE §七-5 一致性前置",
    },
    Constraint {
        id: "board-cohesion",
        applies: &["board"],
        text: "all sections share one palette and one type system; the sheet reads as a \
               single spec page, not a collage",
        source: "KNOWLEDGE §七-5 一致性前置",
    },
    Constraint {
        id: "identity-lock",
        applies: &["component"],
        text: "all cells show the same component family: identical geometry and styling; \
               only the labeled state or variant changes between cells",
        source: "KNOWLEDGE §七-6 同体锁定",
    },
    Constraint {
        id: "flat-ui",
        applies: &["board", "page", "component"],
        text: "flat vector UI mockup, high fidelity, crisp edges, generous whitespace; no \
               photorealistic scenes, no 3D perspective, no device frames beyond the canvas",
        source: "RENDER_RULES（并入）+ KNOWLEDGE §七-7 材质+光影",
    },
    Constraint {
        id: "photography-params",
        applies: &[],
        text: "when a brief asks for photographic touches, prefer lens/aperture numbers over \
               adjectives",
        source: "KNOWLEDGE §七-8 参数说话（UI 出图不注入，仅存档）",
    },
    Constraint {
        id: "hero-dominance",
        applies: &[],
        text: "poster-like renders benefit from a dominant subject at 50-70% of the frame",
        source: "KNOWLEDGE §七-9 主体放大（UI 出图不注入，仅存档）",
    },
    Constraint {
        id: "single-output",
        applies: &["board", "page", "component"],
        text: "render exactly ONE finished sheet — no moodboard, no multiple alternative \
               concepts, no process or comparison shots",
        source: "KNOWLEDGE §七-10 single 输出",
    },
    Constraint {
        id: "brand-accent-only",
        applies: &["page", "component"],
        text: "brand color works as accents (lines, labels, icons, buttons), not full-bleed \
               color fields",
        source: "KNOWLEDGE §七-11 品牌点缀律",
    },
    Constraint {
        id: "realism-imperfection",
        applies: &[],
        text: "photoreal subjects gain realism from natural imperfections (texture, grain)",
        source: "KNOWLEDGE §七-12 加瑕疵增真（UI 出图不注入，仅存档）",
    },
    Constraint {
        id: "ab-alternates",
        applies: &[],
        text: "for A/B variants, generate separate candidates (the tool's --n) instead of \
               one combined sheet",
        source: "KNOWLEDGE §七-13 A/B 输出（由 --n 承担，不注入）",
    },
    Constraint {
        id: "style-feature-not-name",
        applies: &["board", "page", "component"],
        text: "interpret style references through their features (palette, stroke, mood); \
               never reproduce a named artwork's composition",
        source: "KNOWLEDGE §七-14 大师名慎用",
    },
    Constraint {
        id: "small-size-legibility",
        applies: &["board", "component"],
        text: "labels stay legible at small sizes; prefer short labels over dense micro-copy",
        source: "KNOWLEDGE §七-15 强制可读性",
    },
];

/// The rows injected for `kind` (empty `applies` rows are never injected).
pub fn constraints_for(kind: &str) -> Vec<&'static Constraint> {
    CONSTRAINTS
        .iter()
        .filter(|c| c.applies.contains(&kind))
        .collect()
}

/// Render the `Constraints:` block appended to every engine-assembled prompt.
/// Project-level `negativeHints` (UI-REVIEW weakness b) extend the
/// `explicit-negatives` row; the canvas values substitute into
/// `canvas-locked`.
pub fn render_constraints(kind: &str, project: &Project) -> String {
    let mut lines = vec!["Constraints:".to_string()];
    for constraint in constraints_for(kind) {
        let mut text = constraint.text.to_string();
        if constraint.id == "canvas-locked" {
            text = text
                .replace("{w}", &project.canvas_size.w.to_string())
                .replace("{h}", &project.canvas_size.h.to_string());
        }
        if constraint.id == "explicit-negatives" && !project.negative_hints.is_empty() {
            text.push_str(&format!(
                "; additionally forbidden per project: {}",
                project.negative_hints.join("; ")
            ));
        }
        lines.push(format!("- {}: {}", constraint.id, text));
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Verbatim labels (`Labels: a|b|c` lines inside briefs — UI-REVIEW weakness c)
// ---------------------------------------------------------------------------

/// Extract verbatim labels from `Labels: a|b|c` markers in a brief and
/// return them together with the brief text with those markers stripped (the
/// labels travel into the prompt as a dedicated constraint, not as prose).
///
/// Two accepted forms: a dedicated `Labels:` line, or a trailing `Labels: …`
/// segment at the end of a brief line (the common single-line brief). The
/// marker must be the literal `Labels:`; the segment extends to end of line.
pub fn extract_verbatim_labels(brief: &str) -> (Vec<String>, String) {
    const MARKER: &str = "Labels:";
    let mut labels = Vec::new();
    let mut kept = Vec::new();
    for line in brief.lines() {
        match line.find(MARKER) {
            Some(pos) => {
                let head = line[..pos].trim_end();
                if !head.trim().is_empty() {
                    kept.push(head);
                }
                for part in line[pos + MARKER.len()..].split('|') {
                    let part = part.trim();
                    if !part.is_empty() && !labels.iter().any(|l| l == part) {
                        labels.push(part.to_string());
                    }
                }
            }
            None => kept.push(line),
        }
    }
    (labels, kept.join("\n").trim().to_string())
}

/// The verbatim-render constraint appended when a brief declares labels.
fn verbatim_constraint(labels: &[String]) -> String {
    let quoted: Vec<String> = labels.iter().map(|l| format!("\"{l}\"")).collect();
    format!(
        "Verbatim labels: the following must appear exactly as written, \
         character-for-character, one per element — {}; never translate, reword, add or \
         drop them.",
        quoted.join(" ")
    )
}

// ---------------------------------------------------------------------------
// Template-based assembly (engine path)
// ---------------------------------------------------------------------------

/// Base variables shared by every kind.
fn base_vars(project: &Project) -> Vars {
    let mut vars = BTreeMap::new();
    vars.insert("project.name".into(), project.name.trim().to_string());
    vars.insert("project.brandBrief".into(), project.brand_brief.trim().to_string());
    vars.insert("project.styleBrief".into(), project.style_brief.trim().to_string());
    vars.insert("project.negativeHints".into(), project.negative_hints.join("; "));
    vars.insert("project.invariants".into(), invariants(project));
    vars.insert("canvas.w".into(), project.canvas_size.w.to_string());
    vars.insert("canvas.h".into(), project.canvas_size.h.to_string());
    vars
}

/// Compose the design-system **board** prompt (ARCHITECTURE §5.1).
///
/// `template` selects the skeleton; `None` uses the built-in default
/// (`board-design-system`, the parameterized upgrade of the original
/// hand-written skeleton). Constraints are appended automatically.
pub fn compose_board(project: &Project, template: Option<&Template>) -> Result<String> {
    let template = match template {
        Some(t) => t,
        None => &templates::load_builtin(templates::BOARD_TEMPLATE_ID)
            .expect("builtin board template must parse"),
    };
    if template.applies_to != "board" {
        return Err(crate::error::RudderError::InvalidArg {
            detail: format!(
                "template `{}` applies to `{}`, not `board`",
                template.id, template.applies_to
            ),
        });
    }
    let vars = base_vars(project);
    let mut prompt = templates::fill(template, &vars)?;
    prompt.push('\n');
    prompt.push_str(&render_constraints("board", project));
    Ok(prompt)
}

/// Compose a **page** edit prompt (ARCHITECTURE §5.2): anchor reference +
/// layout brief + invariant list (+ verbatim labels + constraints).
pub fn compose_page(project: &Project, page: &Page, template: Option<&Template>) -> Result<String> {
    let template = match template {
        Some(t) => t,
        None => &templates::load_builtin(templates::PAGE_TEMPLATE_ID)
            .expect("builtin page template must parse"),
    };
    if template.applies_to != "page" {
        return Err(crate::error::RudderError::InvalidArg {
            detail: format!(
                "template `{}` applies to `{}`, not `page`",
                template.id, template.applies_to
            ),
        });
    }
    let (labels, brief) = extract_verbatim_labels(page.brief.trim());
    let mut vars = base_vars(project);
    vars.insert("page.slug".into(), page.slug.clone());
    vars.insert("page.brief".into(), brief);
    vars.insert("anchor.reference".into(), ANCHOR_REFERENCE.into());

    let mut prompt = templates::fill(template, &vars)?;
    if !labels.is_empty() {
        prompt.push('\n');
        prompt.push_str(&verbatim_constraint(&labels));
    }
    prompt.push('\n');
    prompt.push_str(&render_constraints("page", project));
    Ok(prompt)
}

/// Compose a **component** edit prompt (ARCHITECTURE §5.2).
pub fn compose_component(
    project: &Project,
    component: &Component,
    template: Option<&Template>,
) -> Result<String> {
    let template = match template {
        Some(t) => t,
        None => &templates::load_builtin(templates::COMPONENT_TEMPLATE_ID)
            .expect("builtin component template must parse"),
    };
    if template.applies_to != "component" {
        return Err(crate::error::RudderError::InvalidArg {
            detail: format!(
                "template `{}` applies to `{}`, not `component`",
                template.id, template.applies_to
            ),
        });
    }
    let (labels, brief) = extract_verbatim_labels(component.brief.trim());
    let mut vars = base_vars(project);
    vars.insert("component.name".into(), component.name.clone());
    vars.insert("component.kind".into(), component.kind.trim().to_string());
    vars.insert("component.brief".into(), brief);
    vars.insert("anchor.reference".into(), ANCHOR_REFERENCE.into());

    let mut prompt = templates::fill(template, &vars)?;
    if !labels.is_empty() {
        prompt.push('\n');
        prompt.push_str(&verbatim_constraint(&labels));
    }
    prompt.push('\n');
    prompt.push_str(&render_constraints("component", project));
    Ok(prompt)
}

// ---------------------------------------------------------------------------
// Legacy wrappers (engine path with the default skeleton)
// ---------------------------------------------------------------------------

/// Compose the board prompt with the default skeleton.
pub fn board_prompt(project: &Project) -> String {
    compose_board(project, None).expect("default board template must fill")
}

/// Compose the page prompt with the default skeleton.
pub fn page_prompt(project: &Project, page: &Page) -> String {
    compose_page(project, page, None).expect("default page template must fill")
}

/// Compose the component prompt with the default skeleton.
pub fn component_prompt(project: &Project, component: &Component) -> String {
    compose_component(project, component, None).expect("default component template must fill")
}

/// The invariant list — the core of set-wide consistency (ARCHITECTURE §5.2).
/// Exposed to templates via the `{project.invariants}` slot.
fn invariants(project: &Project) -> String {
    let mut s = String::new();
    s.push_str("Invariants (do NOT change): strictly reuse Image 1's exact\n");
    s.push_str("- color palette (same hex values for primary/background/text/accent),\n");
    s.push_str("- typography family, sizes and weights,\n");
    s.push_str("- corner radii and border treatment,\n");
    s.push_str("- component styling (buttons, inputs, cards), icon style and stroke weight,\n");
    s.push_str("- spacing rhythm and density.\n");
    if !project.style_brief.trim().is_empty() {
        s.push_str(&format!(
            "Overall mood stays: {}\n",
            project.style_brief.trim()
        ));
    }
    s.push_str("Only compose NEW layout/content; never redesign the system.");
    s
}

/// Build a `promptLog` entry for a generation batch (ARCHITECTURE §5.3).
/// `source` distinguishes engine-assembled prompts from agent-authored
/// prompt files; `template_id` records the skeleton used (either assembly
/// or, with `--prompt-file`, the declared reproducibility reference).
#[allow(clippy::too_many_arguments)]
pub fn log_entry(
    kind: &str,
    target: &str,
    endpoint: &str,
    prompt: &str,
    params: &GenParams,
    candidate_ids: &[String],
    dry_run: bool,
    source: &str,
    template_id: Option<&str>,
) -> PromptLogEntry {
    PromptLogEntry {
        at: crate::store::now_rfc3339(),
        kind: kind.to_string(),
        target: target.to_string(),
        endpoint: endpoint.to_string(),
        prompt: prompt.to_string(),
        params: params.clone(),
        candidate_ids: candidate_ids.to_vec(),
        dry_run,
        source: Some(source.to_string()),
        template_id: template_id.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{CanvasSize, Preset};

    fn project() -> Project {
        Project::new(
            "远洋航运 SaaS",
            CanvasSize::new(1536, 1024, Some(Preset::Web)),
            "远洋航运行业，专业克制",
            "海军蓝 + 黄铜点缀，圆角 8px，紧凑密度",
        )
    }

    // ---- snapshot-style tests: deterministic + key markers ---------------

    #[test]
    fn board_prompt_is_deterministic_and_structured() {
        let p = project();
        let a = board_prompt(&p);
        let b = board_prompt(&p);
        assert_eq!(a, b, "same input must render byte-identical prompts");

        // 用途 first line.
        assert!(a.starts_with("Purpose: a UI design system board"), "{a}");
        // All five sections present.
        for marker in [
            "COLOR PALETTE",
            "TYPOGRAPHY",
            "COMPONENT SAMPLES",
            "ICON STYLE",
            "SPACING & GRID RULES",
        ] {
            assert!(a.contains(marker), "missing {marker} in:\n{a}");
        }
        // Hex-label instruction and brief injection + canvas size.
        assert!(a.contains("hex code printed"));
        assert!(a.contains("海军蓝 + 黄铜点缀"));
        assert!(a.contains("Canvas: 1536x1024"));
        assert!(a.contains("远洋航运行业"));
        // Constraints block injected with board-only rows.
        assert!(a.contains("Constraints:"), "{a}");
        assert!(a.contains("- board-cohesion:"));
        assert!(a.contains("- single-output:"));
        assert!(a.contains("1536x1024"), "canvas-locked substitution: {a}");
        assert!(!a.contains("- consistency-first:"), "page-only row must stay out");
        assert!(!a.contains("- identity-lock:"), "component-only row must stay out");
        assert!(a.contains("- flat-ui:"));
    }

    #[test]
    fn page_prompt_opens_with_anchor_reference_and_invariants() {
        let mut p = project();
        let mut page = Page::new("dashboard".into(), "顶部指标卡x4，中部折线图区".into());
        p.pages.push(page.clone());
        let text = page_prompt(&p, &page);
        assert!(text.starts_with(ANCHOR_REFERENCE), "{text}");
        assert!(text.contains("Layout brief: 顶部指标卡x4"));
        // Invariant list covers palette/type/radius/components/spacing.
        for marker in [
            "Invariants (do NOT change)",
            "color palette",
            "typography",
            "corner radii",
            "component styling",
            "spacing rhythm",
        ] {
            assert!(text.contains(marker), "missing {marker} in:\n{text}");
        }
        assert!(text.contains("never redesign the system"));
        // Page-kind constraint rows.
        assert!(text.contains("- consistency-first:"));
        assert!(text.contains("- brand-accent-only:"));
        assert!(!text.contains("- board-cohesion:"));
        // Determinism is independent of Page field mutation below.
        page.updated_at = "changed".into();
        assert_eq!(page_prompt(&p, &page), text, "timestamps must not leak into prompts");
    }

    #[test]
    fn component_prompt_covers_variants_and_states() {
        let mut p = project();
        let comp = Component::new("button-set".into(), "buttons".into(), "主/次/幽灵按钮三态".into());
        p.components.push(comp.clone());
        let text = component_prompt(&p, &comp);
        assert!(text.starts_with(ANCHOR_REFERENCE), "{text}");
        assert!(text.contains("\"buttons\" family"));
        assert!(text.contains("default / hover / active / disabled / focus"));
        assert!(text.contains("Invariants (do NOT change)"));
        assert!(text.contains("- identity-lock:"), "{text}");
        assert!(text.contains("- small-size-legibility:"));
        assert!(!text.contains("- board-cohesion:"));
    }

    #[test]
    fn constraints_differ_by_kind_and_stay_deterministic() {
        let p = project();
        let board = render_constraints("board", &p);
        let page = render_constraints("page", &p);
        let component = render_constraints("component", &p);
        assert_eq!(board, render_constraints("board", &p));
        // board rows: no anchor-consistency, has board-cohesion.
        assert!(board.contains("board-cohesion"));
        assert!(!board.contains("consistency-first"));
        // page rows: anchor consistency, brand accents, no identity lock.
        assert!(page.contains("consistency-first"));
        assert!(page.contains("brand-accent-only"));
        assert!(!page.contains("identity-lock"));
        // component rows: identity lock + small-size legibility.
        assert!(component.contains("identity-lock"));
        assert!(component.contains("small-size-legibility"));
        // The advisory rows stay out of every injected block.
        for advisory in ["photography-params", "hero-dominance", "realism-imperfection"] {
            assert!(!board.contains(advisory), "{advisory} must not inject");
            assert!(!page.contains(advisory), "{advisory} must not inject");
        }
        // But they remain documented in the full table.
        assert_eq!(CONSTRAINTS.len(), 16, "15 防坑条目 + flat-ui 合并行");
        assert!(constraints_for("board").iter().all(|c| !c.applies.is_empty()));
    }

    #[test]
    fn project_negative_hints_extend_explicit_negatives() {
        let mut p = project();
        p.negative_hints = vec!["不要通用放大镜图标".into(), "no dark mode".into()];
        let page = render_constraints("page", &p);
        assert!(
            page.contains(
                "additionally forbidden per project: 不要通用放大镜图标; no dark mode"
            ),
            "{page}"
        );
        // Clean project: no trailing marker.
        let clean = render_constraints("page", &project());
        assert!(!clean.contains("additionally forbidden"));
    }

    #[test]
    fn verbatim_labels_extract_strip_and_constrain() {
        // Dedicated line form.
        let brief = "顶部导航 3 项\nLabels: 概览|报表|设置\n主区折线图";
        let (labels, cleaned) = extract_verbatim_labels(brief);
        assert_eq!(labels, vec!["概览", "报表", "设置"]);
        assert_eq!(cleaned, "顶部导航 3 项\n主区折线图");

        // Trailing inline segment (single-line brief) — the common case.
        let inline = "顶部导航 3 项，主区折线图。Labels: 概览|报表|设置";
        let (labels_inline, cleaned_inline) = extract_verbatim_labels(inline);
        assert_eq!(labels_inline, vec!["概览", "报表", "设置"]);
        assert_eq!(cleaned_inline, "顶部导航 3 项，主区折线图。");

        // Dedup + whitespace tolerance + no labels → empty.
        let (labels2, cleaned2) = extract_verbatim_labels("Labels: a | a | b");
        assert_eq!(labels2, vec!["a", "b"]);
        assert!(cleaned2.is_empty());
        let (none, _) = extract_verbatim_labels("没有标签的简报");
        assert!(none.is_empty());

        // The constraint quotes every label verbatim.
        let constraint = verbatim_constraint(&labels);
        assert!(constraint.contains("\"概览\" \"报表\" \"设置\""), "{constraint}");
        assert!(constraint.contains("exactly as written"));

        // End-to-end: page prompt carries the constraint, not the raw line.
        let mut p = project();
        let page = Page::new("dashboard".into(), inline.into());
        p.pages.push(page.clone());
        let text = page_prompt(&p, &page);
        assert!(text.contains("Verbatim labels:"), "{text}");
        assert!(!text.contains("Labels: 概览"), "raw Labels line must be stripped:\n{text}");
        assert!(text.contains("顶部导航 3 项，主区折线图。"), "brief prose stays");
    }

    #[test]
    fn compose_with_explicit_template_overrides_the_skeleton() {
        let mut p = project();
        let page = Page::new(
            "landing".into(),
            "导航 4 项（产品/定价/客户/博客），hero 双按钮，3 特性段".into(),
        );
        p.pages.push(page.clone());
        let landing = templates::load_builtin("page-landing-sections").unwrap();
        let text = compose_page(&p, &page, Some(&landing)).unwrap();
        assert!(text.contains("Section contract:"), "{text}");
        assert!(text.contains("closing CTA band"));
        assert!(text.contains("- consistency-first:"), "constraints still injected");
        // Deterministic and different from the default skeleton.
        assert_eq!(compose_page(&p, &page, Some(&landing)).unwrap(), text);
        assert_ne!(text, page_prompt(&p, &page));
    }

    #[test]
    fn compose_rejects_template_kind_mismatch() {
        let p = project();
        let page_template = templates::load_builtin("page-ui-standard").unwrap();
        let err = compose_board(&p, Some(&page_template)).unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
        assert_eq!(err.exit_code(), 1, "template mistakes are argument errors");
        assert!(err.to_string().contains("not `board`"), "{err}");
    }

    #[test]
    fn log_entry_records_prompt_and_params() {
        let params = GenParams {
            model: "gpt-image-2".into(),
            size: "1536x1024".into(),
            quality: "low".into(),
            n: 2,
            seed: Some(42),
            thinking: Some("medium".into()),
        };
        let entry = log_entry(
            "board",
            "",
            "generations",
            "PROMPT",
            &params,
            &["0001".into()],
            false,
            SOURCE_ENGINE,
            Some(templates::BOARD_TEMPLATE_ID),
        );
        assert_eq!(entry.kind, "board");
        assert_eq!(entry.candidate_ids, vec!["0001"]);
        assert!(!entry.dry_run);
        assert_eq!(entry.params.seed, Some(42));
        assert_eq!(entry.source.as_deref(), Some("engine"));
        assert_eq!(entry.template_id.as_deref(), Some("board-design-system"));
        // Serializes into the project.json promptLog shape (camelCase not
        // used here; params keep their stored field names).
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["params"]["model"], "gpt-image-2");
        assert_eq!(json["candidate_ids"][0], "0001");
        assert_eq!(json["prompt"], "PROMPT");
        assert_eq!(json["source"], "engine");
        // PromptLogEntry deliberately keeps snake_case keys (project.json
        // storage shape — see the doc comment on the struct).
        assert_eq!(json["template_id"], "board-design-system");
    }

    #[test]
    fn agent_file_source_round_trips() {
        let params = GenParams {
            model: "gpt-image-2".into(),
            size: "1536x1024".into(),
            quality: "low".into(),
            n: 1,
            seed: Some(1),
            thinking: None,
        };
        let entry = log_entry(
            "page",
            "dash",
            "edits",
            "AGENT PROMPT",
            &params,
            &["0001".into()],
            false,
            SOURCE_AGENT_FILE,
            None,
        );
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["source"], "agent-file");
        assert!(json.get("templateId").is_none(), "absent template must not serialize");
    }
}

// ---------------------------------------------------------------------------
// Golden parity harness (C6): the Rust engine is the source of truth for the
// checked-in goldens under `src/lib/generation/__goldens__/`; the vitest
// suite (`src/lib/generation/prompt.golden.test.ts`) asserts the TS port
// reproduces them byte-for-byte, so CLI/desktop prompt drift fails a test.
//
// One command regenerates the goldens after an INTENDED engine/template
// change (then update the TS side in the same commit):
//
//     RUDDER_UPDATE_GOLDENS=1 cargo test -p rudder-core --lib goldens
//
// Without the env var this test re-asserts the Rust output against the
// checked-in files.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod goldens {
    use super::*;
    use crate::canvas::CanvasSize;

    /// `src/lib/generation/__goldens__/` next to the TS port under test.
    fn goldens_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src/lib/generation/__goldens__")
    }

    /// Fixture A: the representative full spec — real briefs, web canvas, no
    /// negative hints, no verbatim labels.
    fn fixture_a() -> (Project, Page, Component) {
        (
            Project::new(
                "远洋航运 SaaS",
                CanvasSize::new(1536, 1024, None),
                "远洋航运行业，专业克制",
                "海军蓝 + 黄铜点缀，圆角 8px，紧凑密度",
            ),
            Page::new("dashboard".into(), "顶部指标卡x4，中部折线图区".into()),
            Component::new("button-set".into(), "buttons".into(), "主/次/幽灵按钮三态".into()),
        )
    }

    /// Fixture B: the edge paths — untrimmed name (trim), EMPTY style brief
    /// (invariants lose the mood line, empty-slot line drop), project
    /// negative hints (explicit-negatives extension), a custom portrait
    /// canvas (canvas-locked substitution) and verbatim `Labels:` in both
    /// target briefs.
    fn fixture_b() -> (Project, Page, Component) {
        let mut project = Project::new(
            "  学术写作台  ",
            CanvasSize::new(1024, 1364, None),
            "论文写作工作流",
            "",
        );
        project.negative_hints = vec!["不要通用放大镜图标".into(), "no dark mode".into()];
        (
            project,
            Page::new(
                "landing".into(),
                "导航 4 项（产品/定价/客户/博客），hero 双按钮，3 特性段。Labels: 产品|定价|客户|博客".into(),
            ),
            Component::new(
                "input-set".into(),
                "inputs".into(),
                "默认/禁用/错误三态。Labels: 常规|禁用|错误".into(),
            ),
        )
    }

    /// Every builtin template × fixture pair: 5 templates × 2 fixtures.
    fn golden_cases() -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (tag, fixture) in [("a", fixture_a()), ("b", fixture_b())] {
            let (project, page, component) = fixture;
            for template_id in [
                templates::BOARD_TEMPLATE_ID,
                "brand-identity-lite",
                templates::PAGE_TEMPLATE_ID,
                "page-landing-sections",
                templates::COMPONENT_TEMPLATE_ID,
            ] {
                let template = templates::load_builtin(template_id).expect("builtin template");
                let text = match template.applies_to.as_str() {
                    "board" => compose_board(&project, Some(&template)),
                    "page" => compose_page(&project, &page, Some(&template)),
                    _ => compose_component(&project, &component, Some(&template)),
                }
                .expect("golden case composes");
                out.push((format!("{template_id}--{tag}.txt"), text));
            }
        }
        out
    }

    #[test]
    fn goldens_match_the_checked_in_fixtures() {
        let cases = golden_cases();
        assert_eq!(cases.len(), 10, "every builtin template × fixture pair has a golden");
        let update = std::env::var("RUDDER_UPDATE_GOLDENS").ok().as_deref() == Some("1");
        let dir = goldens_dir();
        std::fs::create_dir_all(&dir).expect("create goldens dir");
        for (name, text) in cases {
            let path = dir.join(&name);
            if update {
                std::fs::write(&path, &text).expect("write golden file");
                continue;
            }
            let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "golden `{name}` unreadable ({e}); regenerate with \
                     `RUDDER_UPDATE_GOLDENS=1 cargo test -p rudder-core --lib goldens`"
                )
            });
            assert_eq!(text, expected, "Rust engine drifted from golden `{name}`");
        }
    }
}
