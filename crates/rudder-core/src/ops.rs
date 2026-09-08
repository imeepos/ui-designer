//! High-level operations shared by the CLI and the desktop shell
//! (docs/ARCHITECTURE.md §6 flows). Each function loads the project, runs
//! one step of the 四步流程, persists state atomically and returns a
//! serializable report for the CLI `--json` envelope.

use crate::config::{Config, QUALITY_LEVELS};
use crate::error::{Result, RudderError};
use crate::image::{
    ImageClient, ImageRef, GenerateParams, RequestPlan, MODEL,
};
use crate::prompt;
use crate::store::{
    self, candidate_path, candidates_dir, load_project, promote_candidate, reserve_candidate_ids,
    save_project, Component, GenParams, GenRecord, Page, Project, PromptLogEntry,
};
use crate::canvas::{CanvasSize, Preset};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Default candidate count for board generate (docs/ARCHITECTURE.md §6).
pub const BOARD_DEFAULT_N: u32 = 4;

/// Options for one generate call.
#[derive(Debug, Clone, Default)]
pub struct GenerateOptions {
    /// `None` → per-kind default (board 4, page/component 1, config `n`).
    pub n: Option<u32>,
    /// `None` → config default (`high`).
    pub quality: Option<String>,
    pub seed: Option<u64>,
    /// `None` → config default (`medium`).
    pub thinking: Option<String>,
    /// Extra layout reference images (passed after the anchor).
    pub refs: Vec<PathBuf>,
    /// Final dry-run decision (CLI: `--dry-run` or no `--yes`).
    pub dry_run: bool,
    /// e2e self-test only: build page/component plans before a board exists.
    pub assume_anchor: bool,
}

/// One produced candidate (or a dry-run placeholder row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateReport {
    pub id: String,
    /// Path relative to the project root.
    pub file: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub size: String,
    pub quality: String,
}

/// Result of a generate call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReport {
    pub dry_run: bool,
    pub kind: String,
    pub target: String,
    pub prompt: String,
    pub plan: RequestPlan,
    /// Empty in dry-run mode.
    pub candidates: Vec<CandidateReport>,
}

/// Which entity a generate call targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Board,
    Page(String),
    Component(String),
}

impl Target {
    fn kind(&self) -> &'static str {
        match self {
            Target::Board => "board",
            Target::Page(_) => "page",
            Target::Component(_) => "component",
        }
    }

    fn target_name(&self) -> String {
        match self {
            Target::Board => String::new(),
            Target::Page(s) | Target::Component(s) => s.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// init / add / pick / list
// ---------------------------------------------------------------------------

/// `rudder init` — create the project directory + project.json.
pub fn init_project(
    dir: &Path,
    name: &str,
    size_spec: &str,
    brief: Option<&str>,
) -> Result<PathBuf> {
    if name.trim().is_empty() {
        return Err(RudderError::InvalidArg { detail: "project name must not be empty".into() });
    }
    let canvas = CanvasSize::parse(size_spec)?;
    let project = Project::new(
        name.trim(),
        canvas,
        "",
        brief.unwrap_or(""),
    );
    store::create_project(dir, &project)?;
    // Best effort: an unwritable ~/Rudder must not fail `init`.
    let _ = remember_last_project(dir);
    Ok(dir.to_path_buf())
}

/// Persist the last-used project pointer (best effort; never fatal).
fn remember_last_project(dir: &Path) -> Result<()> {
    let mut config = Config::load();
    config.last_project = Some(dir.to_path_buf());
    config.save()
}
/// `rudder page add` — register a page.
pub fn page_add(root: &Path, slug: &str, brief: &str) -> Result<Page> {
    store::validate_identifier(slug, "page slug")?;
    if brief.trim().is_empty() {
        return Err(RudderError::InvalidArg { detail: "page brief must not be empty".into() });
    }
    let mut project = load_project(root)?;
    if project.page(slug).is_some() {
        return Err(RudderError::AlreadyExists { what: format!("page `{slug}`") });
    }
    let page = Page::new(slug.to_string(), brief.trim().to_string());
    std::fs::create_dir_all(candidates_dir(&page.dir(root)))?;
    project.pages.push(page.clone());
    project
        .prompt_log
        .push(log_marker("page-add", slug));
    save_project(root, &project)?;
    Ok(page)
}

/// `rudder component add` — register a component sheet.
pub fn component_add(root: &Path, name: &str, kind: &str, brief: &str) -> Result<Component> {
    store::validate_identifier(name, "component name")?;
    if kind.trim().is_empty() {
        return Err(RudderError::InvalidArg { detail: "component type must not be empty".into() });
    }
    if brief.trim().is_empty() {
        return Err(RudderError::InvalidArg { detail: "component brief must not be empty".into() });
    }
    let mut project = load_project(root)?;
    if project.component(name).is_some() {
        return Err(RudderError::AlreadyExists { what: format!("component `{name}`") });
    }
    let component = Component::new(name.to_string(), kind.trim().to_string(), brief.trim().to_string());
    std::fs::create_dir_all(candidates_dir(&component.dir(root)))?;
    project.components.push(component.clone());
    project
        .prompt_log
        .push(log_marker("component-add", name));
    save_project(root, &project)?;
    Ok(component)
}

/// Marker entries let PROMPTS.md show non-generation project events too.
fn log_marker(kind: &str, target: &str) -> PromptLogEntry {
    PromptLogEntry {
        at: store::now_rfc3339(),
        kind: kind.to_string(),
        target: target.to_string(),
        endpoint: "-".into(),
        prompt: format!("[{kind}] {target}"),
        params: GenParams {
            model: "-".into(),
            size: "-".into(),
            quality: "-".into(),
            n: 0,
            seed: None,
            thinking: None,
        },
        candidate_ids: Vec::new(),
        dry_run: false,
    }
}

/// Normalize a candidate reference: `0001.png` (an `ls` listing) and `0001`
/// (the documented id) both address the same candidate.
fn normalize_candidate_id(candidate_id: &str) -> &str {
    candidate_id.strip_suffix(".png").unwrap_or(candidate_id)
}

/// `rudder board pick <candidate-id>` — set the anchor.
pub fn board_pick(root: &Path, candidate_id: &str) -> Result<String> {
    let candidate_id = normalize_candidate_id(candidate_id);
    let mut project = load_project(root)?;
    let board_dir = root.join("board");
    promote_candidate(&board_dir, candidate_id, "anchor.png")?;
    let lineage = project
        .board_generations
        .iter()
        .rev()
        .find(|r| r.candidate_ids.iter().any(|id| id == candidate_id));
    project.anchor = Some(store::Anchor {
        candidate_id: candidate_id.to_string(),
        prompt: lineage.map(|r| r.prompt.clone()).unwrap_or_default(),
        seed: lineage.and_then(|r| r.params.seed),
        created_at: store::now_rfc3339(),
    });
    save_project(root, &project)?;
    Ok("board/anchor.png".to_string())
}

/// `rudder page pick <slug> <candidate-id>` — promote to current.
pub fn page_pick(root: &Path, slug: &str, candidate_id: &str) -> Result<String> {
    let candidate_id = normalize_candidate_id(candidate_id);
    let mut project = load_project(root)?;
    let page = project
        .page(slug)
        .cloned()
        .ok_or_else(|| RudderError::NotFound { what: format!("page `{slug}`") })?;
    let rotated = promote_candidate(&page.dir(root), candidate_id, "current.png")?;
    set_page_lineage(&mut project, slug, candidate_id, rotated.as_deref())?;
    save_project(root, &project)?;
    Ok(format!("pages/{slug}/current.png"))
}

fn set_page_lineage(
    project: &mut Project,
    slug: &str,
    candidate_id: &str,
    rotated: Option<&str>,
) -> Result<()> {
    let page = project
        .pages
        .iter_mut()
        .find(|p| p.slug == slug)
        .ok_or_else(|| RudderError::NotFound { what: format!("page `{slug}`") })?;
    page.updated_at = store::now_rfc3339();
    if let Some(rotated) = rotated {
        project
            .prompt_log
            .push(PromptLogEntry {
                at: store::now_rfc3339(),
                kind: "page-pick".into(),
                target: slug.to_string(),
                endpoint: "-".into(),
                prompt: format!("[page-pick] {slug} {candidate_id} (previous → {rotated})"),
                params: GenParams {
                    model: "-".into(),
                    size: "-".into(),
                    quality: "-".into(),
                    n: 0,
                    seed: None,
                    thinking: None,
                },
                candidate_ids: vec![candidate_id.to_string()],
                dry_run: false,
            });
    }
    Ok(())
}

/// `rudder component pick <name> <candidate-id>` — promote to current.
pub fn component_pick(root: &Path, name: &str, candidate_id: &str) -> Result<String> {
    let candidate_id = normalize_candidate_id(candidate_id);
    let mut project = load_project(root)?;
    let component = project
        .component(name)
        .cloned()
        .ok_or_else(|| RudderError::NotFound { what: format!("component `{name}`") })?;
    promote_candidate(&component.dir(root), candidate_id, "current.png")?;
    if let Some(c) = project.components.iter_mut().find(|c| c.name == name) {
        c.updated_at = store::now_rfc3339();
    }
    save_project(root, &project)?;
    Ok(format!("components/{name}/current.png"))
}

/// Status overview for `rudder list`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStatus {
    pub name: String,
    pub brief: String,
    pub candidates: usize,
    pub has_current: bool,
    pub last_generated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub id: String,
    pub name: String,
    pub dir: String,
    pub canvas_size: String,
    pub anchor: Option<String>,
    pub pages: Vec<TargetStatus>,
    pub components: Vec<TargetStatus>,
    pub prompt_log_entries: usize,
}

/// `rudder list` — status overview.
pub fn status(root: &Path) -> Result<StatusReport> {
    let project = load_project(root)?;
    let mut pages = Vec::new();
    for page in &project.pages {
        let dir = page.dir(root);
        pages.push(TargetStatus {
            name: page.slug.clone(),
            brief: page.brief.clone(),
            candidates: store::count_candidates(&dir)?,
            has_current: dir.join("current.png").is_file(),
            last_generated_at: page.generations.last().map(|g| g.at.clone()),
        });
    }
    let mut components = Vec::new();
    for component in &project.components {
        let dir = component.dir(root);
        components.push(TargetStatus {
            name: component.name.clone(),
            brief: component.brief.clone(),
            candidates: store::count_candidates(&dir)?,
            has_current: dir.join("current.png").is_file(),
            last_generated_at: component.generations.last().map(|g| g.at.clone()),
        });
    }
    Ok(StatusReport {
        id: project.id,
        name: project.name.clone(),
        dir: root.display().to_string(),
        canvas_size: project.canvas_size.to_api_string(),
        anchor: project.anchor.as_ref().map(|a| a.candidate_id.clone()),
        pages,
        components,
        prompt_log_entries: project.prompt_log.len(),
    })
}

// ---------------------------------------------------------------------------
// generation (board / page / component)
// ---------------------------------------------------------------------------

/// Validate quality/n against the documented enums, resolve defaults.
fn resolve_generation_params(
    opts: &GenerateOptions,
    config: &Config,
    prompt: String,
    canvas: &CanvasSize,
    n_default: u32,
) -> Result<GenerateParams> {
    let quality = opts.quality.clone().unwrap_or_else(|| config.effective_quality().to_string());
    if !QUALITY_LEVELS.contains(&quality.as_str()) {
        return Err(RudderError::InvalidArg {
            detail: format!("quality must be one of {}", QUALITY_LEVELS.join("|")),
        });
    }
    let n = opts.n.unwrap_or(n_default);
    if !(1..=4).contains(&n) {
        return Err(RudderError::InvalidArg { detail: "n must be in 1..=4".into() });
    }
    let thinking = opts
        .thinking
        .clone()
        .unwrap_or_else(|| config.effective_thinking().to_string());
    Ok(GenerateParams {
        prompt,
        size: canvas.to_api_string(),
        quality,
        n,
        seed: opts.seed,
        thinking: Some(thinking),
    })
}

fn n_default_for(kind: &Target, config: &Config) -> u32 {
    match kind {
        Target::Board => config.n.map(u32::from).unwrap_or(BOARD_DEFAULT_N),
        _ => u32::from(config.effective_n()),
    }
}

fn require_anchor(project: &Project, opts: &GenerateOptions, dry_run: bool) -> Result<()> {
    if project.anchor.is_some() {
        return Ok(());
    }
    if opts.assume_anchor && dry_run {
        return Ok(());
    }
    Err(RudderError::NoAnchor)
}

fn reference_images(
    root: &Path,
    project: &Project,
    opts: &GenerateOptions,
    dry_run: bool,
) -> Result<Vec<ImageRef>> {
    let mut refs = Vec::new();
    // The anchor rides first; e2e dry-run planning may assume it exists.
    if project.anchor.is_some() || (opts.assume_anchor && dry_run) {
        refs.push(ImageRef {
            role: "anchor".into(),
            path: root.join("board/anchor.png"),
        });
    }
    for path in &opts.refs {
        if !dry_run && !path.is_file() {
            return Err(RudderError::InvalidArg {
                detail: format!("reference image {} does not exist", path.display()),
            });
        }
        refs.push(ImageRef { role: "ref".into(), path: path.clone() });
    }
    Ok(refs)
}

/// Unified generate for board / page / component.
pub async fn generate(root: &Path, target: &Target, opts: GenerateOptions, client: &ImageClient) -> Result<GenerateReport> {
    let config = Config::load();
    let project = load_project(root)?;

    let prompt = match target {
        Target::Board => prompt::board_prompt(&project),
        Target::Page(slug) => {
            let page = project
                .page(slug)
                .ok_or_else(|| RudderError::NotFound { what: format!("page `{slug}`") })?;
            require_anchor(&project, &opts, opts.dry_run)?;
            prompt::page_prompt(&project, page)
        }
        Target::Component(name) => {
            let component = project
                .component(name)
                .ok_or_else(|| RudderError::NotFound { what: format!("component `{name}`") })?;
            require_anchor(&project, &opts, opts.dry_run)?;
            prompt::component_prompt(&project, component)
        }
    };

    let params = resolve_generation_params(&opts, &config, prompt.clone(), &project.canvas_size, n_default_for(target, &config))?;

    // generations endpoint when there is nothing to attach (board without
    // refs); otherwise edits (anchor and/or refs present).
    let refs = reference_images(root, &project, &opts, opts.dry_run)?;
    let use_edits = !refs.is_empty();

    let run_output = if use_edits {
        client.run_edits(&params, &refs).await?
    } else {
        client.run_generations(&params).await?
    };
    let plan = run_output.plan;

    if opts.dry_run || client.is_dry_run() {
        return Ok(GenerateReport {
            dry_run: true,
            kind: target.kind().to_string(),
            target: target.target_name(),
            prompt,
            plan,
            candidates: Vec::new(),
        });
    }

    let images = run_output
        .images
        .ok_or_else(|| RudderError::BadResponse { detail: "no images decoded".into() })?;

    // Real run: persist candidates, lineage and the prompt log atomically.
    let target_dir = match target {
        Target::Board => root.join("board"),
        Target::Page(slug) => project
            .page(slug)
            .ok_or_else(|| RudderError::NotFound { what: format!("page `{slug}`") })?
            .dir(root),
        Target::Component(name) => project
            .component(name)
            .ok_or_else(|| RudderError::NotFound { what: format!("component `{name}`") })?
            .dir(root),
    };
    let ids = reserve_candidate_ids(&target_dir, images.len())?;
    let mut written = Vec::new();
    for (id, bytes) in ids.iter().zip(images.iter()) {
        let path = candidate_path(&target_dir, id);
        store::atomic_write(&path, bytes)?;
        written.push((id.clone(), path));
    }

    let record = GenRecord {
        at: store::now_rfc3339(),
        endpoint: plan.endpoint.clone(),
        prompt: prompt.clone(),
        params: GenParams {
            model: MODEL.to_string(),
            size: params.size.clone(),
            quality: params.quality.clone(),
            n: params.n,
            seed: params.seed,
            thinking: params.thinking.clone(),
        },
        candidate_ids: ids.clone(),
    };

    let mut project = load_project(root)?;
    match target {
        Target::Board => {
            project.board_generations.push(record.clone());
        }
        Target::Page(slug) => {
            if let Some(page) = project.pages.iter_mut().find(|p| p.slug == *slug) {
                page.prompt = Some(prompt.clone());
                page.seed = params.seed;
                page.updated_at = store::now_rfc3339();
                page.generations.push(record.clone());
            }
        }
        Target::Component(name) => {
            if let Some(component) = project.components.iter_mut().find(|c| c.name == *name) {
                component.prompt = Some(prompt.clone());
                component.seed = params.seed;
                component.updated_at = store::now_rfc3339();
                component.generations.push(record.clone());
            }
        }
    }
    project.prompt_log.push(prompt::log_entry(
        target.kind(),
        &target.target_name(),
        &plan.endpoint,
        &prompt,
        &record.params,
        &ids,
        false,
    ));
    save_project(root, &project)?;

    let file_base = match target {
        Target::Board => "board".to_string(),
        Target::Page(slug) => format!("pages/{slug}"),
        Target::Component(name) => format!("components/{name}"),
    };
    let candidates = written
        .into_iter()
        .map(|(id, _path)| CandidateReport {
            file: format!("{file_base}/candidates/{id}.png"),
            id,
            prompt: prompt.clone(),
            seed: params.seed,
            size: params.size.clone(),
            quality: params.quality.clone(),
        })
        .collect();

    Ok(GenerateReport {
        dry_run: false,
        kind: target.kind().to_string(),
        target: target.target_name(),
        prompt,
        plan,
        candidates,
    })
}

// ---------------------------------------------------------------------------
// e2e self-test
// ---------------------------------------------------------------------------

/// One labelled step of `rudder e2e`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct E2eStep {
    pub step: String,
    pub dry_run: bool,
    pub detail: String,
}

/// Result of `rudder e2e`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct E2eReport {
    pub dry_run: bool,
    pub project_dir: String,
    pub export_dir: Option<String>,
    pub steps: Vec<E2eStep>,
}

/// `rudder e2e` — sample project → board → 1 page → 1 component → export.
/// Without a real client (`--yes`) every image step yields only its plan.
pub async fn run_e2e(client: &ImageClient, quality: &str) -> Result<E2eReport> {
    let dry_run = client.is_dry_run();
    let mut steps = Vec::new();
    let work = std::env::temp_dir().join(format!("rudder-e2e-{}", uuid::Uuid::new_v4()));
    let project_dir = work.join("proj");

    init_project(&project_dir, "e2e-sample", "web", Some("rudder self-test, clean neutral style"))?;
    steps.push(E2eStep {
        step: "init".into(),
        dry_run: false,
        detail: project_dir.display().to_string(),
    });

    let board_opts = GenerateOptions {
        n: Some(1),
        quality: Some(quality.to_string()),
        dry_run,
        ..Default::default()
    };
    let board = generate(&project_dir, &Target::Board, board_opts, client).await?;
    steps.push(E2eStep {
        step: "board generate".into(),
        dry_run: board.dry_run,
        detail: if board.dry_run {
            format!("plan → {}", board.plan.url)
        } else {
            format!("candidates: {}", board.candidates.iter().map(|c| c.id.clone()).collect::<Vec<_>>().join(", "))
        },
    });

    if !dry_run {
        let first = board
            .candidates
            .first()
            .map(|c| c.id.clone())
            .ok_or_else(|| RudderError::BadResponse { detail: "board generation returned no candidates".into() })?;
        let anchor_file = board_pick(&project_dir, &first)?;
        steps.push(E2eStep { step: "board pick".into(), dry_run: false, detail: anchor_file });
    }

    let page = page_add(&project_dir, "dashboard", "top KPI cards x4, main chart area, right task list")?;
    steps.push(E2eStep {
        step: "page add".into(),
        dry_run: false,
        detail: format!("pages/{}", page.slug),
    });
    let page_gen = generate(
        &project_dir,
        &Target::Page("dashboard".into()),
        GenerateOptions { n: Some(1), quality: Some(quality.to_string()), dry_run, assume_anchor: dry_run, ..Default::default() },
        client,
    )
    .await?;
    steps.push(E2eStep {
        step: "page generate".into(),
        dry_run: page_gen.dry_run,
        detail: if page_gen.dry_run {
            "plan only (anchor assumed)".into()
        } else {
            format!("candidates: {}", page_gen.candidates.iter().map(|c| c.id.clone()).collect::<Vec<_>>().join(", "))
        },
    });

    let component = component_add(&project_dir, "button-set", "buttons", "primary/secondary/ghost, 3 states each")?;
    steps.push(E2eStep {
        step: "component add".into(),
        dry_run: false,
        detail: format!("components/{}", component.name),
    });
    let comp_gen = generate(
        &project_dir,
        &Target::Component("button-set".into()),
        GenerateOptions { n: Some(1), quality: Some(quality.to_string()), dry_run, assume_anchor: dry_run, ..Default::default() },
        client,
    )
    .await?;
    steps.push(E2eStep {
        step: "component generate".into(),
        dry_run: comp_gen.dry_run,
        detail: if comp_gen.dry_run {
            "plan only (anchor assumed)".into()
        } else {
            format!("candidates: {}", comp_gen.candidates.iter().map(|c| c.id.clone()).collect::<Vec<_>>().join(", "))
        },
    });

    let export_dir = work.join("export");
    let report = crate::export::export_project(&project_dir, &export_dir)?;
    steps.push(E2eStep {
        step: "export".into(),
        dry_run: false,
        detail: format!("{} files → {}", report.files, export_dir.display()),
    });

    Ok(E2eReport {
        dry_run,
        project_dir: project_dir.display().to_string(),
        export_dir: Some(export_dir.display().to_string()),
        steps,
    })
}

/// Preset name for a canvas size (`init` echoes it in `--json` data).
pub fn preset_name(canvas: &CanvasSize) -> Option<&'static str> {
    canvas.preset.map(Preset::as_str)
}

#[cfg(test)]
mod tests;
