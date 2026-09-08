//! Asset bundle export (PRD §3.5 / ARCHITECTURE §6 `rudder export`).
//!
//! Produces:
//! - `board/anchor.png` (+ board candidates), `pages/<slug>/current.png`,
//!   `components/<name>/current.png` under the output directory;
//! - `manifest.json` — project metadata plus per-image lineage
//!   (prompt / model / seed / size / quality / endpoint);
//! - `PROMPTS.md` — the full prompt log as readable markdown;
//! - `DESIGN.template.md` — the skeleton an AI agent fills into DESIGN.md.

use crate::error::Result;
use crate::store::{self, Component, GenRecord, Page, Project};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// What `rudder export` produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub out: PathBuf,
    pub manifest: PathBuf,
    pub files: usize,
}

/// Per-image lineage record inside the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestImage {
    pub id: String,
    /// Path relative to the export root.
    pub file: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub model: String,
    pub size: String,
    pub quality: String,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
}

/// Copy `src` to `<out>/<rel>`; returns 1 on success, 0 when missing.
fn copy_into(out: &Path, src: &Path, rel: &str) -> Result<usize> {
    if !src.is_file() {
        return Ok(0);
    }
    let dest = out.join(rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, &dest)?;
    Ok(1)
}

/// Find the GenRecord that produced `candidate_id`.
fn lineage<'a>(records: &'a [GenRecord], candidate_id: &str) -> Option<&'a GenRecord> {
    records
        .iter()
        .find(|r| r.candidate_ids.iter().any(|id| id == candidate_id))
}

fn board_image(project: &Project, id: &str, file: &str) -> ManifestImage {
    let record = lineage(&project.board_generations, id);
    ManifestImage {
        id: id.to_string(),
        file: file.to_string(),
        prompt: record.map(|r| r.prompt.clone()).unwrap_or_default(),
        seed: record.and_then(|r| r.params.seed),
        model: record.map(|r| r.params.model.clone()).unwrap_or_else(|| crate::image::MODEL.to_string()),
        size: record.map(|r| r.params.size.clone()).unwrap_or_else(|| project.canvas_size.to_api_string()),
        quality: record.map(|r| r.params.quality.clone()).unwrap_or_default(),
        endpoint: record.map(|r| r.endpoint.clone()).unwrap_or_default(),
        generated_at: record.map(|r| r.at.clone()),
    }
}

/// Render PROMPTS.md from the project's prompt log.
pub fn render_prompts_md(project: &Project) -> String {
    let mut md = String::new();
    md.push_str(&format!("# PROMPTS.md — {}\n\n", project.name));
    md.push_str(&format!(
        "Full prompt/parameter log, newest last. Project `{}` (id `{}`), canvas {}.\n\n",
        project.name,
        project.id,
        project.canvas_size.to_api_string()
    ));
    if project.prompt_log.is_empty() {
        md.push_str("_No generations logged yet._\n");
        return md;
    }
    for (i, entry) in project.prompt_log.iter().enumerate() {
        let target = if entry.target.is_empty() {
            String::new()
        } else {
            format!(" `{}`", entry.target)
        };
        md.push_str(&format!(
            "## {num}. {kind}{target} — {endpoint}\n\n",
            num = i + 1,
            kind = entry.kind,
            endpoint = entry.endpoint
        ));
        md.push_str(&format!("- at: `{}`\n", entry.at));
        md.push_str(&format!(
            "- params: model=`{model}` size=`{size}` quality=`{quality}` n={n}",
            model = entry.params.model,
            size = entry.params.size,
            quality = entry.params.quality,
            n = entry.params.n
        ));
        if let Some(seed) = entry.params.seed {
            md.push_str(&format!(" seed=`{seed}`"));
        }
        if let Some(thinking) = &entry.params.thinking {
            md.push_str(&format!(" thinking=`{thinking}`"));
        }
        md.push('\n');
        if !entry.candidate_ids.is_empty() {
            md.push_str(&format!("- candidates: {}\n", entry.candidate_ids.join(", ")));
        }
        if entry.dry_run {
            md.push_str("- _dry-run plan (no images produced)_\n");
        }
        md.push_str(&format!("\n```\n{}\n```\n\n", entry.prompt));
    }
    md
}

/// Render DESIGN.template.md (the contract draft for DESIGN.md, per
/// skill/rudder-design/references/design-md.md).
pub fn render_design_template(project: &Project, manifest_rel: &str) -> String {
    let mut md = String::new();
    md.push_str(&format!("# DESIGN.md — {}（待填模板）\n\n", project.name));
    md.push_str(&format!(
        "Source: {manifest_rel} (generated {today}, rudder v{ver})\n\n",
        manifest_rel = manifest_rel,
        today = chrono::Utc::now().date_naive(),
        ver = crate::VERSION
    ));
    md.push_str(
        "> Fill every table ONLY with values readable from the exported images. \
         If the images don't define a value, ask the user — never invent.\n\n",
    );
    md.push_str("## Tokens\n\n### Palette\n\n| token | hex | usage |\n|---|---|---|\n");
    md.push_str("| primary | #… | buttons, active nav, links |\n");
    md.push_str("| bg | #… | app background |\n");
    md.push_str("| … | | (one row per swatch on the board) |\n\n");

    md.push_str("### Typography\n\n");
    md.push_str("family: …  \n");
    md.push_str("| role | size | weight | notes |\n|---|---|---|---|\n");
    md.push_str("| display | … | … | |\n| heading | … | … | |\n");
    md.push_str("| body | … | … | |\n| caption | … | … | |\n\n");

    md.push_str("### Geometry\n\n");
    md.push_str("radius: card …px · button …px · input …px · spacing base …px · borders …px\n\n");

    md.push_str("## Components\n\n");
    if project.components.is_empty() {
        md.push_str("_No components in this project._\n\n");
    }
    for c in &project.components {
        md.push_str(&format!(
            "### {name} ({kind})\n\nvariants(…) × states(default/hover/disabled/…) — cite `components/{name}/current.png`\n\n",
            name = c.name,
            kind = c.kind
        ));
    }

    md.push_str("## Layout\n\n");
    if project.pages.is_empty() {
        md.push_str("_No pages in this project._\n\n");
    }
    for p in &project.pages {
        md.push_str(&format!(
            "### {slug}\n\n{brief}\n\ncite `pages/{slug}/current.png` — regions, widths, alignment.\n\n",
            slug = p.slug,
            brief = p.brief
        ));
    }

    md.push_str("## Known deviations\n\n- (anything the pages do that the board doesn't define)\n");
    md
}

/// Export the project at `root` into `out`, creating directories as needed.
pub fn export_project(root: &Path, out: &Path) -> Result<ExportReport> {
    let project = store::load_project(root)?;
    if !out.exists() {
        std::fs::create_dir_all(out)?;
    }
    let mut files = 0usize;

    // ---- board -----------------------------------------------------------
    files += copy_into(out, &root.join("board/anchor.png"), "board/anchor.png")?;
    let candidates_dir = root.join("board/candidates");
    let mut board_candidate_entries: Vec<ManifestImage> = Vec::new();
    if candidates_dir.is_dir() {
        let mut names: Vec<String> = std::fs::read_dir(&candidates_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".png"))
            .collect();
        names.sort();
        for name in names {
            files += copy_into(out, &candidates_dir.join(&name), &format!("board/candidates/{name}"))?;
            let id = name.trim_end_matches(".png").to_string();
            board_candidate_entries.push(board_image(&project, &id, &format!("board/candidates/{name}")));
        }
    }

    // ---- pages / components (currents only, per cli.md contract) ----------
    let mut page_entries = Vec::new();
    for page in &project.pages {
        let rel = format!("pages/{}/current.png", page.slug);
        files += copy_into(out, &root.join(&rel), &rel)?;
        if root.join(&rel).is_file() {
            let record = page
                .generations
                .iter()
                .rev()
                .find(|r| page.prompt.as_deref() == Some(r.prompt.as_str()))
                .or_else(|| page.generations.last());
            page_entries.push(page_manifest(page, &rel, record));
        }
    }
    let mut component_entries = Vec::new();
    for component in &project.components {
        let rel = format!("components/{}/current.png", component.name);
        files += copy_into(out, &root.join(&rel), &rel)?;
        if root.join(&rel).is_file() {
            let record = component.generations.last();
            component_entries.push(component_manifest(component, &rel, record));
        }
    }

    // ---- manifest.json -----------------------------------------------------
    let manifest = serde_json::json!({
        "rudderVersion": crate::VERSION,
        "generatedAt": store::now_rfc3339(),
        "project": {
            "id": project.id,
            "name": project.name,
            "canvasSize": {
                "w": project.canvas_size.w,
                "h": project.canvas_size.h,
                "preset": project.canvas_size.preset.map(|p| p.as_str()),
            },
            "brandBrief": project.brand_brief,
            "styleBrief": project.style_brief,
        },
        "board": {
            "anchor": project.anchor.as_ref().map(|a| {
                board_image(&project, &a.candidate_id, "board/anchor.png")
            }),
            "candidates": board_candidate_entries,
        },
        "pages": page_entries,
        "components": component_entries,
        "promptLog": project.prompt_log,
    });
    let manifest_path = out.join("manifest.json");
    store::atomic_write_json(&manifest_path, &manifest)?;

    // ---- PROMPTS.md / DESIGN.template.md -----------------------------------
    store::atomic_write(&out.join("PROMPTS.md"), render_prompts_md(&project).as_bytes())?;
    store::atomic_write(
        &out.join("DESIGN.template.md"),
        render_design_template(&project, "manifest.json").as_bytes(),
    )?;

    Ok(ExportReport {
        out: out.to_path_buf(),
        manifest: manifest_path,
        files,
    })
}

fn page_manifest(page: &Page, rel: &str, record: Option<&GenRecord>) -> serde_json::Value {
    serde_json::json!({
        "slug": page.slug,
        "brief": page.brief,
        "file": rel,
        "prompt": record.map(|r| r.prompt.clone()).unwrap_or_default(),
        "seed": record.and_then(|r| r.params.seed),
        "model": record.map(|r| r.params.model.clone()).unwrap_or_else(|| crate::image::MODEL.to_string()),
        "size": record.map(|r| r.params.size.clone()).unwrap_or_default(),
        "quality": record.map(|r| r.params.quality.clone()).unwrap_or_default(),
        "endpoint": record.map(|r| r.endpoint.clone()).unwrap_or_default(),
        "generatedAt": record.map(|r| r.at.clone()),
        "updatedAt": page.updated_at,
    })
}

fn component_manifest(component: &Component, rel: &str, record: Option<&GenRecord>) -> serde_json::Value {
    serde_json::json!({
        "name": component.name,
        "type": component.kind,
        "brief": component.brief,
        "file": rel,
        "prompt": record.map(|r| r.prompt.clone()).unwrap_or_default(),
        "seed": record.and_then(|r| r.params.seed),
        "model": record.map(|r| r.params.model.clone()).unwrap_or_else(|| crate::image::MODEL.to_string()),
        "size": record.map(|r| r.params.size.clone()).unwrap_or_default(),
        "quality": record.map(|r| r.params.quality.clone()).unwrap_or_default(),
        "endpoint": record.map(|r| r.endpoint.clone()).unwrap_or_default(),
        "generatedAt": record.map(|r| r.at.clone()),
        "updatedAt": component.updated_at,
    })
}

/// Ensure `out` doesn't silently sit inside the project root (export would
/// otherwise copy the project into itself).
pub fn validate_out_dir(root: &Path, out: &Path) -> Result<()> {
    if out.starts_with(root) {
        return Err(crate::error::RudderError::InvalidArg {
            detail: format!(
                "--out {} is inside the project {}; pick a directory outside it",
                out.display(),
                root.display()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
