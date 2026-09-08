//! Prompt engine (ARCHITECTURE §5): three-part templates that make a whole
//! set style-consistent.
//!
//! 1. Board template — a structured "UI design system board" sheet: palette
//!    (with hex labels), type scale, component samples, icon style, spacing.
//! 2. Page/component edit template — opens with the anchor reference line
//!    「Image 1 是本产品设计系统总板」 and carries the invariant list.
//! 3. Every generation records the final prompt + params into `promptLog`.

use crate::store::{Component, GenParams, Page, Project, PromptLogEntry};

/// The literal anchor reference every page/component prompt starts with
/// (docs/ARCHITECTURE.md §5).
pub const ANCHOR_REFERENCE: &str = "Image 1 是本产品设计系统总板";

/// Shared style guard appended to every prompt so renders stay usable as
/// implementation references (not posters).
const RENDER_RULES: &str = "\
Rendering rules: flat vector UI mockup, high fidelity, crisp edges, realistic \
UI label text (no lorem ipsum, no gibberish glyphs), generous whitespace, \
no photorealistic scenes, no 3D perspective, no device frames beyond the \
canvas itself.";

/// Compose the design-system **board** prompt (ARCHITECTURE §5.1).
///
/// Structure: 用途 → 色板(含 hex 标签) → 字体层级 → 组件样本 → 图标风格 → 间距规则，
/// with the project's brand/style brief injected.
pub fn board_prompt(project: &Project) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "Purpose: a UI design system board (风格总板) for the product \"{name}\".\n",
        name = project.name
    ));
    if !project.brand_brief.trim().is_empty() {
        s.push_str(&format!("Brand brief: {}\n", project.brand_brief.trim()));
    }
    if !project.style_brief.trim().is_empty() {
        s.push_str(&format!("Style brief: {}\n", project.style_brief.trim()));
    }
    s.push_str(&format!(
        "Canvas: {w}x{h}. Lay out ONE labeled sheet with these sections:\n\
         1. COLOR PALETTE — 5-8 swatch chips as rounded rectangles, each with \
         its exact hex code printed as a label under or inside the chip.\n\
         2. TYPOGRAPHY — type hierarchy specimen: display / heading / subheading \
         / body / caption rows, each labeled with size and weight.\n\
         3. COMPONENT SAMPLES — a row of buttons (primary, secondary, ghost; \
         default/hover/disabled states), one text input, one select, one card, \
         one toggle, one checkbox, labeled.\n\
         4. ICON STYLE — a row of 6-8 sample icons showing stroke weight and \
         corner treatment.\n\
         5. SPACING & GRID RULES — a spacing scale diagram (4/8/16/24/32px) and \
         corner-radius samples per element class.\n\
         Sections separated by thin dividers, generous margins, engineering \
         spec-sheet clarity.\n",
        w = project.canvas_size.w,
        h = project.canvas_size.h
    ));
    s.push_str(RENDER_RULES);
    s
}

/// Compose a **page** edit prompt (ARCHITECTURE §5.2): anchor reference +
/// layout brief + invariant list.
pub fn page_prompt(project: &Project, page: &Page) -> String {
    let mut s = String::new();
    s.push_str(ANCHOR_REFERENCE);
    s.push_str("。严格沿用它的设计语言，只组合新的页面内容。\n");
    s.push_str(&format!(
        "Task: design the full \"{slug}\" page as one high-fidelity UI mockup.\n\
         Layout brief: {brief}\n",
        slug = page.slug,
        brief = page.brief.trim()
    ));
    s.push_str(&invariants(project));
    s.push_str(RENDER_RULES);
    s
}

/// Compose a **component** edit prompt (ARCHITECTURE §5.2).
pub fn component_prompt(project: &Project, component: &Component) -> String {
    let mut s = String::new();
    s.push_str(ANCHOR_REFERENCE);
    s.push_str("。严格沿用它的设计语言，为单个组件族出细节图。\n");
    s.push_str(&format!(
        "Task: a component detail sheet for the \"{kind}\" family: {brief}\n\
         Show every variant and state in a tidy grid, each labeled \
         (e.g. default / hover / active / disabled / focus).\n",
        kind = component.kind.trim(),
        brief = component.brief.trim()
    ));
    s.push_str(&invariants(project));
    s.push_str(RENDER_RULES);
    s
}

/// The invariant list — the core of set-wide consistency (ARCHITECTURE §5.2).
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
    s.push_str("Only compose NEW layout/content; never redesign the system.\n");
    s
}

/// Build a `promptLog` entry for a generation batch (ARCHITECTURE §5.3).
pub fn log_entry(
    kind: &str,
    target: &str,
    endpoint: &str,
    prompt: &str,
    params: &GenParams,
    candidate_ids: &[String],
    dry_run: bool,
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
        let entry = log_entry("board", "", "generations", "PROMPT", &params, &["0001".into()], false);
        assert_eq!(entry.kind, "board");
        assert_eq!(entry.candidate_ids, vec!["0001"]);
        assert!(!entry.dry_run);
        assert_eq!(entry.params.seed, Some(42));
        // Serializes into the project.json promptLog shape (camelCase not
        // used here; params keep their stored field names).
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["params"]["model"], "gpt-image-2");
        assert_eq!(json["candidate_ids"][0], "0001");
        assert_eq!(json["prompt"], "PROMPT");
    }
}
