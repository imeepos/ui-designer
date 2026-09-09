//! Template protocol (PRD §0 产品边界): agent-consumable prompt skeletons +
//! per-slot fill guides. Rudder ships NO intelligence of its own — an
//! external coding agent reads a skeleton plus its `fillGuide` (`rudder
//! templates show <id>`), fills the slots with its own LLM, and feeds the
//! final prompt back through `--prompt-file`.
//!
//! The engine-side path (default skeleton or `--template <id>`) uses
//! [`fill`] with strict rules:
//! - unknown slots → error (exit 1, hint lists `rudder templates list`);
//! - a line whose slots ALL resolve to empty is dropped (optional briefs);
//! - templates must declare every slot their skeleton uses.

use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default board template id (also the engine default skeleton).
pub const BOARD_TEMPLATE_ID: &str = "board-design-system";
/// Default page template id.
pub const PAGE_TEMPLATE_ID: &str = "page-ui-standard";
/// Default component template id.
pub const COMPONENT_TEMPLATE_ID: &str = "component-sheet-grid";

/// Embedded template assets (single source of truth: `/templates/*.json`).
const EMBEDDED: &[(&str, &str)] = &[
    (
        BOARD_TEMPLATE_ID,
        include_str!("../../../templates/board-design-system.json"),
    ),
    (
        "page-ui-standard",
        include_str!("../../../templates/page-ui-standard.json"),
    ),
    (
        "page-landing-sections",
        include_str!("../../../templates/page-landing-sections.json"),
    ),
    (
        COMPONENT_TEMPLATE_ID,
        include_str!("../../../templates/component-sheet-grid.json"),
    ),
    (
        "brand-identity-lite",
        include_str!("../../../templates/brand-identity-lite.json"),
    ),
];

const MANIFEST_JSON: &str = include_str!("../../../templates/manifest.json");

/// Bilingual copy (zh primary, en secondary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bilingual {
    pub zh: String,
    pub en: String,
}

/// One declared parameter slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotDef {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Bilingual>,
}

/// One fill-guide entry: what the slot wants, a good example, common mistakes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillGuideEntry {
    pub slot: String,
    pub what: String,
    #[serde(default)]
    pub good_example: String,
    #[serde(default)]
    pub common_mistakes: Vec<String>,
}

/// The teaching content an external LLM reads before filling slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillGuide {
    pub how_to: String,
    pub slots: Vec<FillGuideEntry>,
}

/// A parameterized prompt template (one JSON file under `/templates/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    #[serde(default = "default_version")]
    pub version: u32,
    /// `board` | `page` | `component`.
    pub applies_to: String,
    pub name: Bilingual,
    pub summary: Bilingual,
    #[serde(default)]
    pub attribution: String,
    pub slots: Vec<SlotDef>,
    pub skeleton: String,
    pub fill_guide: FillGuide,
}

fn default_version() -> u32 {
    1
}

impl Template {
    /// Slot names declared by this template.
    pub fn slot_names(&self) -> Vec<&str> {
        self.slots.iter().map(|s| s.name.as_str()).collect()
    }
}

/// Attribution block of `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestAttribution {
    #[serde(rename = "basedOn")]
    pub based_on: String,
    pub url: String,
    pub license: String,
    pub usage: String,
    pub notice: String,
}

/// One entry of `manifest.json` (the quick-scan index agents list first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    pub file: String,
    pub applies_to: String,
    pub name: Bilingual,
    pub summary: Bilingual,
    pub slots: Vec<String>,
}

/// Parsed `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateManifest {
    pub version: u32,
    pub attribution: ManifestAttribution,
    #[serde(default)]
    pub slot_vocabulary: BTreeMap<String, String>,
    #[serde(default)]
    pub fill_protocol: String,
    pub templates: Vec<ManifestEntry>,
}

/// Variables one fill may reference, keyed by slot name (`project.name` …).
pub type Vars = BTreeMap<String, String>;

// ---------------------------------------------------------------------------
// Loading & validation
// ---------------------------------------------------------------------------

/// Parse + validate a template from JSON text.
pub fn parse(id: &str, json: &str) -> Result<Template> {
    let template: Template = serde_json::from_str(json).map_err(|e| {
        arg_err(format!("template `{id}` is corrupt: {e}"))
    })?;
    validate(&template)?;
    Ok(template)
}

/// Structural consistency: skeleton slots ⊆ declared slots ⊆ vocabulary,
/// fill-guide slots ⊆ declared slots, ids/fields sane.
pub fn validate(template: &Template) -> Result<()> {
    if template.id.trim().is_empty() || template.skeleton.trim().is_empty() {
        return Err(arg_err(format!(
            "template `{}` must have a non-empty id and skeleton",
            template.id
        )));
    }
    if !matches!(template.applies_to.as_str(), "board" | "page" | "component") {
        return Err(arg_err(format!(
            "template `{}` has invalid appliesTo `{}` (board|page|component)",
            template.id, template.applies_to
        )));
    }
    let mut declared = std::collections::BTreeSet::new();
    for slot in &template.slots {
        if !VOCABULARY.contains(&slot.name.as_str()) {
            return Err(arg_err(format!(
                "template `{}` declares unknown slot `{}` (see manifest slotVocabulary)",
                template.id, slot.name
            )));
        }
        if !declared.insert(slot.name.as_str()) {
            return Err(arg_err(format!(
                "template `{}` declares slot `{}` twice",
                template.id, slot.name
            )));
        }
    }
    for slot in extract_slots(&template.skeleton) {
        if !declared.contains(slot.as_str()) {
            return Err(arg_err(format!(
                "template `{}` skeleton uses undeclared slot `{{{slot}}}`",
                template.id
            )));
        }
    }
    for entry in &template.fill_guide.slots {
        if !declared.contains(entry.slot.as_str()) {
            return Err(arg_err(format!(
                "template `{}` fillGuide references undeclared slot `{}`",
                template.id, entry.slot
            )));
        }
    }
    Ok(())
}

/// Load an embedded template by id. Unknown ids are argument errors (exit 1)
/// with a hint pointing at `rudder templates list`.
pub fn load_builtin(id: &str) -> Result<Template> {
    let id = id.trim();
    let Some((_, json)) = EMBEDDED.iter().find(|(name, _)| *name == id) else {
        return Err(RudderError::TemplateNotFound {
            id: id.to_string(),
            available: builtin_ids().join(", "),
        });
    };
    parse(id, json)
}

/// Ids of all embedded templates, in manifest order.
pub fn builtin_ids() -> Vec<&'static str> {
    EMBEDDED.iter().map(|(id, _)| *id).collect()
}

/// Parse the embedded manifest.
pub fn builtin_manifest() -> Result<TemplateManifest> {
    serde_json::from_str(MANIFEST_JSON)
        .map_err(|e| arg_err(format!("templates/manifest.json is corrupt: {e}")))
}

/// Engine default skeleton id for a target kind.
pub fn default_template_id(kind: &str) -> &'static str {
    match kind {
        "board" => BOARD_TEMPLATE_ID,
        "page" => PAGE_TEMPLATE_ID,
        _ => COMPONENT_TEMPLATE_ID,
    }
}

// ---------------------------------------------------------------------------
// Filling
// ---------------------------------------------------------------------------

/// Variables available for a target kind. Page/component kinds additionally
/// expose their target slots and the anchor reference.
pub fn allowed_variables(kind: &str) -> &'static [&'static str] {
    match kind {
        "page" => PAGE_VARIABLES,
        "component" => COMPONENT_VARIABLES,
        _ => BASE_VARIABLES,
    }
}

const BASE_VARIABLES: &[&str] = &[
    "project.name",
    "project.brandBrief",
    "project.styleBrief",
    "project.negativeHints",
    "project.invariants",
    "canvas.w",
    "canvas.h",
];

const PAGE_VARIABLES: &[&str] = &[
    "project.name",
    "project.brandBrief",
    "project.styleBrief",
    "project.negativeHints",
    "project.invariants",
    "canvas.w",
    "canvas.h",
    "page.slug",
    "page.brief",
    "anchor.reference",
];

const COMPONENT_VARIABLES: &[&str] = &[
    "project.name",
    "project.brandBrief",
    "project.styleBrief",
    "project.negativeHints",
    "project.invariants",
    "canvas.w",
    "canvas.h",
    "component.name",
    "component.kind",
    "component.brief",
    "anchor.reference",
];

/// Extract `{slot}` tokens from text. Tokens matching `[A-Za-z0-9_.]+` are
/// slot references; any other braces pass through untouched.
pub fn extract_slots(text: &str) -> Vec<String> {
    let mut slots = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(close) = text[i + 1..].find('}') {
                let inner = &text[i + 1..i + 1 + close];
                let ok = !inner.is_empty()
                    && inner
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_');
                if ok {
                    slots.push(inner.to_string());
                    i += close + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    slots
}

/// Fill a template's skeleton with `vars`.
///
/// Rules (deterministic, documented in templates/README.md):
/// - every slot in the skeleton must resolve via `vars`, else exit-1 error;
/// - a line whose slots ALL resolve to empty/whitespace is dropped (that is
///   how optional briefs disappear from the rendered prompt);
/// - runs of 3+ newlines collapse to one blank line; result is trimmed.
pub fn fill(template: &Template, vars: &Vars) -> Result<String> {
    let mut lines = Vec::new();
    for line in template.skeleton.lines() {
        let slots = extract_slots(line);
        let mut rendered = line.to_string();
        let mut all_empty = !slots.is_empty();
        for slot in &slots {
            let value = vars.get(slot).ok_or_else(|| {
                arg_err(format!(
                    "template `{}` uses slot `{{{slot}}}` which is not available here \
                     (available: {})",
                    template.id,
                    vars.keys().cloned().collect::<Vec<_>>().join(", ")
                ))
            })?;
            if !value.trim().is_empty() {
                all_empty = false;
            }
            rendered = rendered.replace(&format!("{{{slot}}}"), value);
        }
        if all_empty {
            continue;
        }
        lines.push(rendered);
    }
    let mut text = lines.join("\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    Ok(text.trim().to_string())
}

fn arg_err(detail: String) -> RudderError {
    RudderError::InvalidArg { detail }
}

/// Slot vocabulary every template must draw from (mirrors manifest.json).
const VOCABULARY: &[&str] = &[
    "project.name",
    "project.brandBrief",
    "project.styleBrief",
    "project.negativeHints",
    "project.invariants",
    "canvas.w",
    "canvas.h",
    "page.slug",
    "page.brief",
    "component.name",
    "component.kind",
    "component.brief",
    "anchor.reference",
];

#[cfg(test)]
mod tests;
