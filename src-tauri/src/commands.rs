//! Tauri commands: thin async wrappers over `rudder-core::ops` running on
//! `tauri::async_runtime` (docs/ARCHITECTURE.md §7).
//!
//! - Every command resolves the project by its `project.json` id under
//!   `~/Rudder/projects`.
//! - Errors are normalized to `{code, message, hint}` (`error.rs`); `code`
//!   uses the frontend `ApiErrorCode` vocabulary.
//! - Generate/export commands emit `rudder://job` progress events with
//!   `started` / `finished` / `failed` phases so the UI can drive skeleton
//!   copy; `jobId` correlates events with the invoking call.

use rudder_core::config::{credential, resolve_base_url, Config};
use rudder_core::image::ImageClient;
use rudder_core::ops::{self, GenerateOptions, Target};
use rudder_core::store::{self, Project};
use rudder_core::{export, RudderError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{codes, CommandError};
use crate::projects;
use crate::view::{
    self, ExportResultDto, GeneratePayloadDto, ProjectDetailDto, ProjectSummaryDto,
};

/// Reply payload of the `ping` command.
#[derive(Serialize)]
pub struct PingReply {
    pub message: String,
    pub version: String,
}

/// Liveness probe used by the frontend to verify the Rust bridge.
#[tauri::command]
pub fn ping() -> PingReply {
    PingReply {
        message: "pong".to_string(),
        version: rudder_core::VERSION.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Job progress events (skeleton-screen copy for 30s-3min generations)
// ---------------------------------------------------------------------------

/// Event channel for job lifecycle updates.
pub const JOB_EVENT: &str = "rudder://job";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobEvent<'a> {
    job_id: &'a str,
    /// `started` | `finished` | `failed`.
    phase: &'a str,
    /// `board` | `page` | `component` | `export` (matches UI job kinds).
    kind: &'a str,
    /// Page slug / component name; empty for board/export.
    target: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
}

/// Emits `started` before and `finished`/`failed` after the wrapped call.
struct JobTracker {
    app: AppHandle,
    job_id: String,
    kind: String,
    target: String,
}

impl JobTracker {
    fn new(app: &AppHandle, job_id: Option<&str>, kind: &str, target: &str) -> Self {
        JobTracker {
            app: app.clone(),
            job_id: job_id
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            kind: kind.to_string(),
            target: target.to_string(),
        }
    }

    fn emit(&self, phase: &'static str, message: Option<String>) {
        let _ = self.app.emit(
            JOB_EVENT,
            JobEvent {
                job_id: &self.job_id,
                phase,
                kind: &self.kind,
                target: &self.target,
                message: message.as_deref(),
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Command inputs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeInput {
    pub w: u32,
    pub h: u32,
    /// `web` | `mobile` | `desktop` | `custom`.
    pub preset: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub size: SizeInput,
    #[serde(default)]
    pub brand_brief: String,
    #[serde(default)]
    pub style_brief: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardBriefInput {
    pub brand_keywords: String,
    pub color_direction: String,
    pub font_mood: String,
    pub radius_density: String,
    #[serde(default)]
    pub reference: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenOptionsInput {
    /// Candidate count 1-4 (`None` → core default: board 4, rest 1).
    #[serde(default)]
    pub count: Option<u32>,
    /// Generation quality tier (`None` → core default: low).
    #[serde(default)]
    pub quality: Option<String>,
    /// Correlates `rudder://job` progress events with this call.
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptionsInput {
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPageInput {
    pub slug: String,
    pub brief: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddComponentInput {
    pub name: String,
    #[serde(rename = "type")]
    pub component_type: String,
    pub brief: String,
}

/// Mirrors `DeleteTarget` from `src/lib/api/types.ts`.
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DeleteTargetInput {
    BoardCandidate { candidate_id: String },
    Page { slug: String },
    PageCurrent { slug: String },
    PageHistory { slug: String, ts: i64 },
    Component { name: String },
    ComponentCurrent { name: String },
    ComponentHistory { name: String, ts: i64 },
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn into_command(err: RudderError) -> CommandError {
    CommandError::from_core(&err)
}

/// Load a project by id → `(root, project)`.
fn resolve_project(project_id: &str) -> Result<(PathBuf, Project), CommandError> {
    let root = projects::find_project_root(project_id).map_err(into_command)?;
    let project = store::load_project(&root).map_err(into_command)?;
    Ok((root, project))
}

/// Load the project and serialize the full detail view.
fn detail_view(root: &Path) -> Result<ProjectDetailDto, CommandError> {
    let project = store::load_project(root).map_err(into_command)?;
    view::build_detail(root, &project)
        .map_err(|e| CommandError::from_core(&RudderError::Io(e)))
}

/// The board brief injected into `project.json` before generation, matching
/// the mock adapter's brand/style merge.
fn style_brief_from(brief: &BoardBriefInput) -> String {
    let mut style = [brief.color_direction.trim(), brief.font_mood.trim(), brief.radius_density.trim()]
        .iter()
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join(" / ");
    if !brief.reference.trim().is_empty() {
        if !style.is_empty() {
            style.push_str(" | ");
        }
        style.push_str(brief.reference.trim());
    }
    style
}

fn generate_payload(
    root: &Path,
    report: ops::GenerateReport,
) -> Result<GeneratePayloadDto, CommandError> {
    let project = store::load_project(root).map_err(into_command)?;
    let detail = view::build_detail(root, &project)
        .map_err(|e| CommandError::from_core(&RudderError::Io(e)))?;
    let candidates = report
        .candidates
        .into_iter()
        .map(|c| {
            let path = root.join(&c.file);
            view::CandidateDto {
                seed: c.seed,
                path: path.display().to_string(),
                created_at: view::file_mtime_ms(&path),
                id: c.id,
            }
        })
        .collect();
    Ok(GeneratePayloadDto {
        candidates,
        project: detail,
    })
}

/// One real generation call (desktop buttons are the explicit `--yes`).
/// Runs on the async runtime; the core client handles timeouts/retries.
async fn run_generation(
    app: &AppHandle,
    project_id: &str,
    kind: &str,
    target_label: &str,
    target: Target,
    options: Option<GenOptionsInput>,
) -> Result<GeneratePayloadDto, CommandError> {
    let options = options.unwrap_or_default();
    let tracker = JobTracker::new(app, options.job_id.as_deref(), kind, target_label);
    let root = projects::find_project_root(project_id).map_err(into_command)?;
    tracker.emit("started", None);
    let client = ImageClient::from_config(false).map_err(into_command);
    let result = match client {
        Ok(client) => {
            let generate = ops::generate(
                &root,
                &target,
                GenerateOptions {
                    n: options.count,
                    quality: options.quality,
                    ..Default::default()
                },
                &client,
            )
            .await;
            match generate {
                Ok(report) => generate_payload(&root, report),
                Err(err) => Err(into_command(err)),
            }
        }
        Err(err) => Err(err),
    };
    match &result {
        Ok(_) => tracker.emit("finished", None),
        Err(err) => tracker.emit("failed", Some(err.message.clone())),
    }
    result
}

// ---------------------------------------------------------------------------
// Project lifecycle commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn create_project(input: CreateProjectInput) -> Result<ProjectDetailDto, CommandError> {
    let size_spec = match input.size.preset.as_str() {
        "web" | "mobile" | "desktop" => input.size.preset.clone(),
        _ => format!("{}x{}", input.size.w, input.size.h),
    };
    let dir = projects::new_project_dir().map_err(into_command)?;
    ops::init_project(&dir, input.name.trim(), &size_spec, Some(input.style_brief.trim()))
        .map_err(into_command)?;
    let mut project = store::load_project(&dir).map_err(into_command)?;
    project.brand_brief = input.brand_brief.trim().to_string();
    store::save_project(&dir, &project).map_err(into_command)?;
    detail_view(&dir)
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<ProjectSummaryDto>, CommandError> {
    let found = projects::list_projects().map_err(into_command)?;
    Ok(found
        .into_iter()
        .map(|(_, project)| view::build_summary(&project))
        .collect())
}

#[tauri::command]
pub async fn get_project(project_id: String) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    detail_view(&root)
}

// ---------------------------------------------------------------------------
// Board / page / component flow commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn generate_board(
    app: AppHandle,
    project_id: String,
    brief: BoardBriefInput,
    options: Option<GenOptionsInput>,
) -> Result<GeneratePayloadDto, CommandError> {
    // Persist the brief first so the prompt engine sees it (mirrors MockApi).
    let (root, mut project) = resolve_project(&project_id)?;
    project.brand_brief = brief.brand_keywords.trim().to_string();
    project.style_brief = style_brief_from(&brief);
    store::save_project(&root, &project).map_err(into_command)?;
    run_generation(&app, &project_id, "board", "", Target::Board, options).await
}

#[tauri::command]
pub async fn pick_anchor(
    project_id: String,
    candidate_id: String,
) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    ops::board_pick(&root, &candidate_id).map_err(into_command)?;
    detail_view(&root)
}

#[tauri::command]
pub async fn add_page(
    project_id: String,
    input: AddPageInput,
) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    ops::page_add(&root, input.slug.trim(), input.brief.trim())
        .map_err(|e| CommandError::with_overrides(&e, codes::DUPLICATE_SLUG, codes::UNKNOWN))?;
    detail_view(&root)
}

#[tauri::command]
pub async fn generate_page(
    app: AppHandle,
    project_id: String,
    slug: String,
    options: Option<GenOptionsInput>,
) -> Result<GeneratePayloadDto, CommandError> {
    let target_label = slug.clone();
    run_generation(
        &app,
        &project_id,
        "page",
        &target_label,
        Target::Page(slug),
        options,
    )
    .await
}

#[tauri::command]
pub async fn pick_page(
    project_id: String,
    slug: String,
    candidate_id: String,
) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    ops::page_pick(&root, &slug, &candidate_id).map_err(into_command)?;
    detail_view(&root)
}

#[tauri::command]
pub async fn add_component(
    project_id: String,
    input: AddComponentInput,
) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    ops::component_add(&root, input.name.trim(), input.component_type.trim(), input.brief.trim())
        .map_err(|e| CommandError::with_overrides(&e, codes::DUPLICATE_NAME, codes::UNKNOWN))?;
    detail_view(&root)
}

#[tauri::command]
pub async fn generate_component(
    app: AppHandle,
    project_id: String,
    name: String,
    options: Option<GenOptionsInput>,
) -> Result<GeneratePayloadDto, CommandError> {
    let target_label = name.clone();
    run_generation(
        &app,
        &project_id,
        "component",
        &target_label,
        Target::Component(name),
        options,
    )
    .await
}

#[tauri::command]
pub async fn pick_component(
    project_id: String,
    name: String,
    candidate_id: String,
) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    ops::component_pick(&root, &name, &candidate_id).map_err(into_command)?;
    detail_view(&root)
}

// ---------------------------------------------------------------------------
// Export / delete
// ---------------------------------------------------------------------------

async fn run_export(root: &Path, out_dir: &str) -> Result<ExportResultDto, CommandError> {
    let export_failed = |e: RudderError| {
        CommandError::with_overrides(&e, codes::EXPORT_FAILED, codes::EXPORT_FAILED)
    };
    let out = projects::expand_home(out_dir).map_err(export_failed)?;
    let root_owned = root.to_path_buf();
    let out_owned = out.clone();
    let report = tauri::async_runtime::spawn_blocking(move || -> Result<export::ExportReport, RudderError> {
        export::validate_out_dir(&root_owned, &out_owned)?;
        export::export_project(&root_owned, &out_owned)
    })
    .await
    .map_err(|e| {
        CommandError::new(
            codes::EXPORT_FAILED,
            format!("export task failed: {e}"),
            "retry the export",
        )
    })?
    .map_err(export_failed)?;
    let files = projects::scan_export_files(&report.out).map_err(export_failed)?;
    Ok(ExportResultDto {
        out_dir: report.out.display().to_string(),
        files,
    })
}

#[tauri::command]
pub async fn export_project(
    app: AppHandle,
    project_id: String,
    out_dir: String,
    options: Option<ExportOptionsInput>,
) -> Result<ExportResultDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    let tracker = JobTracker::new(
        &app,
        options.as_ref().and_then(|o| o.job_id.as_deref()),
        "export",
        "",
    );
    tracker.emit("started", None);
    let result = run_export(&root, &out_dir).await;
    match &result {
        Ok(_) => tracker.emit("finished", None),
        Err(err) => tracker.emit("failed", Some(err.message.clone())),
    }
    if result.is_ok() {
        // Exported files must be viewable via the asset protocol even when
        // they live outside ~/Rudder (best effort, session-scoped).
        if let Ok(out) = projects::expand_home(&out_dir) {
            let _ = app.asset_protocol_scope().allow_directory(&out, true);
        }
    }
    result
}

fn require_page(project: &Project, slug: &str) -> Result<(), CommandError> {
    if project.page(slug).is_some() {
        Ok(())
    } else {
        Err(CommandError::from_core(&RudderError::NotFound {
            what: format!("page `{slug}`"),
        }))
    }
}

fn require_component(project: &Project, name: &str) -> Result<(), CommandError> {
    if project.component(name).is_some() {
        Ok(())
    } else {
        Err(CommandError::from_core(&RudderError::NotFound {
            what: format!("component `{name}`"),
        }))
    }
}

fn remove_existing_file(path: &Path, what: &str) -> Result<(), CommandError> {
    if !path.is_file() {
        return Err(CommandError::from_core(&RudderError::NotFound {
            what: what.to_string(),
        }));
    }
    std::fs::remove_file(path)
        .map_err(|e| CommandError::from_core(&RudderError::Io(e)))
}

/// The `history/<ts>.png` file whose parsed ts equals `ts`.
fn history_file_for_ts(dir: &Path, ts: i64) -> Result<PathBuf, CommandError> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".png") && view::history_ts_from_name(&name) == Some(ts) {
                return Ok(dir.join(name));
            }
        }
    }
    Err(CommandError::from_core(&RudderError::NotFound {
        what: format!("history entry {ts}"),
    }))
}

/// Apply one `deleteArtifact` mutation on disk + `project.json`.
fn apply_delete(root: &Path, target: DeleteTargetInput) -> Result<Project, CommandError> {
    let mut project = store::load_project(root).map_err(into_command)?;
    match target {
        DeleteTargetInput::BoardCandidate { candidate_id } => {
            let is_anchor = project
                .anchor
                .as_ref()
                .map(|a| a.candidate_id == candidate_id)
                .unwrap_or(false);
            if is_anchor {
                return Err(CommandError::anchor_locked(format!(
                    "board candidate `{candidate_id}` is the current anchor"
                )));
            }
            let path = store::candidate_path(&root.join("board"), &candidate_id);
            remove_existing_file(&path, format!("board candidate `{candidate_id}`").as_str())?;
        }
        DeleteTargetInput::Page { slug } => {
            require_page(&project, &slug)?;
            let dir = root.join("pages").join(&slug);
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir)
                    .map_err(|e| CommandError::from_core(&RudderError::Io(e)))?;
            }
            project.pages.retain(|p| p.slug != slug);
            store::save_project(root, &project).map_err(into_command)?;
        }
        DeleteTargetInput::PageCurrent { slug } => {
            require_page(&project, &slug)?;
            let path = root.join("pages").join(&slug).join("current.png");
            remove_existing_file(&path, format!("current image of page `{slug}`").as_str())?;
        }
        DeleteTargetInput::PageHistory { slug, ts } => {
            require_page(&project, &slug)?;
            let path = history_file_for_ts(&root.join("pages").join(&slug).join("history"), ts)?;
            remove_existing_file(&path, format!("history entry of page `{slug}`").as_str())?;
        }
        DeleteTargetInput::Component { name } => {
            require_component(&project, &name)?;
            let dir = root.join("components").join(&name);
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir)
                    .map_err(|e| CommandError::from_core(&RudderError::Io(e)))?;
            }
            project.components.retain(|c| c.name != name);
            store::save_project(root, &project).map_err(into_command)?;
        }
        DeleteTargetInput::ComponentCurrent { name } => {
            require_component(&project, &name)?;
            let path = root.join("components").join(&name).join("current.png");
            remove_existing_file(&path, format!("current image of component `{name}`").as_str())?;
        }
        DeleteTargetInput::ComponentHistory { name, ts } => {
            require_component(&project, &name)?;
            let path =
                history_file_for_ts(&root.join("components").join(&name).join("history"), ts)?;
            remove_existing_file(&path, format!("history entry of component `{name}`").as_str())?;
        }
    }
    Ok(project)
}

#[tauri::command]
pub async fn delete_artifact(
    project_id: String,
    target: DeleteTargetInput,
) -> Result<ProjectDetailDto, CommandError> {
    let (root, _) = resolve_project(&project_id)?;
    let root_clone = root.to_path_buf();
    let project = tauri::async_runtime::spawn_blocking(move || apply_delete(&root_clone, target))
        .await
        .map_err(|e| {
            CommandError::new(
                codes::UNKNOWN,
                format!("delete task failed: {e}"),
                "retry the deletion",
            )
        })??;
    view::build_detail(&root, &project).map_err(|e| CommandError::from_core(&RudderError::Io(e)))
}

// ---------------------------------------------------------------------------
// Credential / settings commands (keychain is the only secret store)
// ---------------------------------------------------------------------------

/// Shell-level credential status. Never carries the key itself — only the
/// effective base URL, the source label and the masked tail (≤4 chars).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatusDto {
    /// Effective base URL after the env → config.json → default chain.
    pub base_url: String,
    /// `env` | `keychain` | `none`.
    pub key_source: String,
    /// Last 4 characters of the resolved key, when one is configured.
    pub key_tail: Option<String>,
}

fn credential_status() -> CredentialStatusDto {
    credential_status_from(resolve_base_url(), &credential::resolve_api_key())
}

/// Pure shaping of the status DTO (unit-testable without a keychain).
fn credential_status_from(base_url: String, resolution: &credential::ApiKeyResolution) -> CredentialStatusDto {
    CredentialStatusDto {
        base_url,
        key_source: resolution.source.as_str().to_string(),
        key_tail: resolution.key.as_deref().map(credential::tail4),
    }
}

/// Helper mirroring `delete_artifact`: run blocking keychain/config work on
/// the blocking pool and flatten join errors into `UNKNOWN`.
async fn blocking<T>(
    what: &str,
    hint: &str,
    task: impl FnOnce() -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError>
where
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| CommandError::new(codes::UNKNOWN, format!("{what} task failed: {e}"), hint))?
}

#[tauri::command]
pub async fn get_credential_status() -> Result<CredentialStatusDto, CommandError> {
    blocking("status", "retry", || Ok(credential_status())).await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveApiKeyInput {
    pub api_key: String,
}

/// Store the API key in the OS keychain (the only persistent secret store).
#[tauri::command]
pub async fn save_api_key(input: SaveApiKeyInput) -> Result<CredentialStatusDto, CommandError> {
    blocking("keychain", "retry the save", move || {
        credential::set_api_key(&input.api_key).map_err(into_command)?;
        Ok(credential_status())
    })
    .await
}

/// Remove the stored API key (idempotent; env keys are unaffected).
#[tauri::command]
pub async fn clear_api_key() -> Result<CredentialStatusDto, CommandError> {
    blocking("keychain", "retry the clear", || {
        credential::clear_api_key().map_err(into_command)?;
        Ok(credential_status())
    })
    .await
}

/// Persist the non-sensitive base URL override into `~/Rudder/config.json`
/// (empty string resets to the env/default chain).
#[tauri::command]
pub async fn save_base_url(base_url: String) -> Result<CredentialStatusDto, CommandError> {
    blocking("config", "retry the save", move || {
        let mut config = Config::load();
        config.set("base_url", &base_url).map_err(into_command)?;
        config.save().map_err(into_command)?;
        Ok(credential_status())
    })
    .await
}

/// Draft values from the Settings dialog; `None`/empty falls back to the
/// stored/resolved chain so the user can test before saving.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionInput {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Reply of the `test_connection` command (free `/v1/models` probe).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestDto {
    pub base_url: String,
    pub http_status: u16,
    pub model_count: usize,
}

/// Probe `GET {base}/v1/models`: free, no image spend. Fails with
/// `NO_CREDENTIALS` when no key is available to test with.
#[tauri::command]
pub async fn test_connection(
    input: Option<TestConnectionInput>,
) -> Result<ConnectionTestDto, CommandError> {
    let input = input.unwrap_or_default();
    let base = input
        .base_url
        .map(|base| base.trim().trim_end_matches('/').to_string())
        .filter(|base| !base.is_empty())
        .unwrap_or_else(resolve_base_url);
    let key = match input.api_key.map(|key| key.trim().to_string()).filter(|key| !key.is_empty()) {
        Some(key) => key,
        None => credential::resolve_api_key().key.ok_or_else(|| {
            CommandError::new(
                codes::NO_CREDENTIALS,
                "no API key configured",
                "save a key in Settings first",
            )
        })?,
    };
    let report = credential::test_connection(&base, &key)
        .await
        .map_err(into_command)?;
    Ok(ConnectionTestDto {
        base_url: report.base_url,
        http_status: report.http_status,
        model_count: report.model_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rudder_core::canvas::CanvasSize;
    use rudder_core::store::candidate_path;

    fn tmp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rudder-cmd-{tag}-{}", uuid::Uuid::new_v4()))
    }

    fn sample_project() -> Project {
        Project::new("样例", CanvasSize::new(1536, 1024, None), "brand", "style")
    }

    fn write_png(path: &Path) {
        store::atomic_write(path, b"png").unwrap();
    }

    #[test]
    fn ping_replies_pong_with_version() {
        let reply = ping();
        assert_eq!(reply.message, "pong");
        assert_eq!(reply.version, rudder_core::VERSION);
    }

    #[test]
    fn style_brief_merges_like_the_mock() {
        let brief = BoardBriefInput {
            brand_keywords: "acme".into(),
            color_direction: "deep blue".into(),
            font_mood: "".into(),
            radius_density: "rounded".into(),
            reference: "dribbble shot".into(),
        };
        assert_eq!(style_brief_from(&brief), "deep blue / rounded | dribbble shot");
    }

    #[test]
    fn delete_board_candidate_blocks_anchor_and_removes_otherwise() {
        let root = tmp_root("board-cand");
        let mut project = sample_project();
        store::create_project(&root, &project).unwrap();
        write_png(&candidate_path(&root.join("board"), "0001"));
        write_png(&candidate_path(&root.join("board"), "0002"));
        project.anchor = Some(rudder_core::store::Anchor {
            candidate_id: "0001".into(),
            prompt: "p".into(),
            seed: None,
            created_at: store::now_rfc3339(),
        });
        store::save_project(&root, &project).unwrap();

        let locked = apply_delete(
            &root,
            DeleteTargetInput::BoardCandidate { candidate_id: "0001".into() },
        )
        .unwrap_err();
        assert_eq!(locked.code, codes::ANCHOR_LOCKED);
        assert!(candidate_path(&root.join("board"), "0001").is_file());

        apply_delete(
            &root,
            DeleteTargetInput::BoardCandidate { candidate_id: "0002".into() },
        )
        .unwrap();
        assert!(!candidate_path(&root.join("board"), "0002").is_file());

        let missing = apply_delete(
            &root,
            DeleteTargetInput::BoardCandidate { candidate_id: "9999".into() },
        )
        .unwrap_err();
        assert_eq!(missing.code, codes::NOT_FOUND);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_page_removes_dir_and_model_entry() {
        let root = tmp_root("page");
        let mut project = sample_project();
        store::create_project(&root, &project).unwrap();
        let page = rudder_core::store::Page::new("dashboard".into(), "b".into());
        project.pages.push(page.clone());
        store::save_project(&root, &project).unwrap();
        write_png(&root.join("pages/dashboard/candidates/0001.png"));

        apply_delete(&root, DeleteTargetInput::Page { slug: "dashboard".into() }).unwrap();
        assert!(!root.join("pages/dashboard").exists());
        assert!(store::load_project(&root).unwrap().pages.is_empty());

        let missing = apply_delete(&root, DeleteTargetInput::Page { slug: "dashboard".into() })
            .unwrap_err();
        assert_eq!(missing.code, codes::NOT_FOUND);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_history_targets_file_by_ts() {
        let root = tmp_root("history");
        let mut project = sample_project();
        store::create_project(&root, &project).unwrap();
        let mut page = rudder_core::store::Page::new("dash".into(), "b".into());
        page.updated_at = store::now_rfc3339();
        project.pages.push(page);
        store::save_project(&root, &project).unwrap();
        let name = format!("{}.png", store::now_history_ts());
        write_png(&root.join("pages/dash/history").join(&name));
        let ts = view::history_ts_from_name(&name).unwrap();

        // A wrong ts must not delete anything.
        let missing = apply_delete(
            &root,
            DeleteTargetInput::PageHistory { slug: "dash".into(), ts: ts + 1_000 },
        )
        .unwrap_err();
        assert_eq!(missing.code, codes::NOT_FOUND);

        apply_delete(
            &root,
            DeleteTargetInput::PageHistory { slug: "dash".into(), ts },
        )
        .unwrap();
        assert!(!root.join("pages/dash/history").join(&name).exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_target_deserializes_frontend_tags() {
        let cases = [
            r#"{"kind":"boardCandidate","candidateId":"0001"}"#,
            r#"{"kind":"page","slug":"dash"}"#,
            r#"{"kind":"pageCurrent","slug":"dash"}"#,
            r#"{"kind":"pageHistory","slug":"dash","ts":42}"#,
            r#"{"kind":"component","name":"btn"}"#,
            r#"{"kind":"componentCurrent","name":"btn"}"#,
            r#"{"kind":"componentHistory","name":"btn","ts":7}"#,
        ];
        for json in cases {
            let parsed: DeleteTargetInput = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("parse {json}: {e}"));
            assert!(matches!(
                parsed,
                DeleteTargetInput::BoardCandidate { .. }
                    | DeleteTargetInput::Page { .. }
                    | DeleteTargetInput::PageCurrent { .. }
                    | DeleteTargetInput::PageHistory { .. }
                    | DeleteTargetInput::Component { .. }
                    | DeleteTargetInput::ComponentCurrent { .. }
                    | DeleteTargetInput::ComponentHistory { .. }
            ));
        }
        let unknown = r#"{"kind":"project"}"#;
        assert!(serde_json::from_str::<DeleteTargetInput>(unknown).is_err());
    }

    #[test]
    fn credential_status_exposes_only_source_and_masked_tail() {
        let resolution = credential::ApiKeyResolution {
            key: Some("abcd1234".into()),
            source: credential::KeySource::Keychain,
        };
        let status = credential_status_from("https://proxy.example.com".into(), &resolution);
        assert_eq!(status.key_source, "keychain");
        assert_eq!(status.key_tail.as_deref(), Some("1234"));
        assert_eq!(status.base_url, "https://proxy.example.com");
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["keySource"], "keychain");
        assert_eq!(json["keyTail"], "1234");
        assert_eq!(json["baseUrl"], "https://proxy.example.com");
        // The full key must never appear anywhere in the DTO.
        let rendered = json.to_string();
        assert!(!rendered.contains("abcd1234"), "DTO leaked the key: {rendered}");

        let none = credential::ApiKeyResolution { key: None, source: credential::KeySource::None };
        let status = credential_status_from("https://api.openai.com".into(), &none);
        assert_eq!(status.key_source, "none");
        assert_eq!(status.key_tail, None);
    }

    #[test]
    fn test_connection_input_deserializes_camel_case_drafts() {
        let parsed: TestConnectionInput = serde_json::from_str(
            r#"{"baseUrl":"https://proxy.example.com/","apiKey":" draft-key-9 "}"#,
        )
        .unwrap();
        assert_eq!(parsed.base_url.as_deref(), Some("https://proxy.example.com/"));
        assert_eq!(parsed.api_key.as_deref(), Some(" draft-key-9 "));
        let empty: TestConnectionInput = serde_json::from_str("{}").unwrap();
        assert!(empty.base_url.is_none() && empty.api_key.is_none());
    }
}
