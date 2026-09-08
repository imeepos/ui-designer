//! Desktop DTO view: serializes the on-disk project state (ARCHITECTURE §3
//! layout + `project.json`) into shapes mirroring `src/lib/api/types.ts`.
//!
//! DTOs carry absolute file paths; the frontend converts them to `asset://`
//! URLs via `convertFileSrc` so asset-protocol knowledge stays client-side.

use chrono::{NaiveDateTime, TimeZone, Utc};
use rudder_core::canvas::Preset;
use rudder_core::store::{GenRecord, Project};
use serde::Serialize;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasDto {
    pub w: u32,
    pub h: u32,
    /// `web` | `mobile` | `desktop` | `custom`.
    pub preset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDto {
    pub id: String,
    /// Absolute path; the frontend maps it to an asset URL.
    pub path: String,
    /// Epoch millis.
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentDto {
    pub path: String,
    pub candidate_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDto {
    /// Epoch millis derived from the `history/<ts>.png` file name.
    pub ts: i64,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorDto {
    pub candidate_id: String,
    pub path: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub slug: String,
    pub brief: String,
    pub candidates: Vec<CandidateDto>,
    pub current: Option<CurrentDto>,
    pub history: Vec<HistoryDto>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDto {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub brief: String,
    pub candidates: Vec<CandidateDto>,
    pub current: Option<CurrentDto>,
    pub history: Vec<HistoryDto>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub size: CanvasDto,
    pub created_at: i64,
    pub has_anchor: bool,
    pub page_count: usize,
    pub component_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetailDto {
    pub id: String,
    pub name: String,
    pub size: CanvasDto,
    pub brand_brief: String,
    pub style_brief: String,
    pub anchor: Option<AnchorDto>,
    pub board_candidates: Vec<CandidateDto>,
    pub pages: Vec<PageDto>,
    pub components: Vec<ComponentDto>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePayloadDto {
    pub candidates: Vec<CandidateDto>,
    pub project: ProjectDetailDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFileDto {
    /// Path relative to the export root.
    pub path: String,
    pub bytes: u64,
    /// `image` | `manifest` | `prompts` | `template`.
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResultDto {
    pub out_dir: String,
    pub files: Vec<ExportFileDto>,
}

/// `web` / `mobile` / `desktop`, or `custom` for arbitrary WxH.
pub fn preset_name(preset: Option<Preset>) -> String {
    preset
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "custom".to_string())
}

pub fn canvas_view(size: &rudder_core::canvas::CanvasSize) -> CanvasDto {
    CanvasDto {
        w: size.w,
        h: size.h,
        preset: preset_name(size.preset),
    }
}

/// RFC 3339 → epoch millis (0 when unparseable — display-only field).
pub fn rfc3339_to_ms(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

/// File mtime in epoch millis (0 when unavailable).
pub fn file_mtime_ms(path: &Path) -> i64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Parse a `history/<ts>.png` name (`%Y%m%d-%H%M%S%3f`, UTC, from
/// `store::now_history_ts`) into epoch millis.
pub fn history_ts_from_name(file_name: &str) -> Option<i64> {
    let stem = file_name.strip_suffix(".png")?;
    let naive = NaiveDateTime::parse_from_str(stem, "%Y%m%d-%H%M%S%3f").ok()?;
    Some(Utc.from_utc_datetime(&naive).timestamp_millis())
}

/// Seed of the batch that produced `candidate_id` (latest match wins).
fn lineage_seed(records: &[GenRecord], candidate_id: &str) -> Option<u64> {
    records
        .iter()
        .rev()
        .find(|r| r.candidate_ids.iter().any(|id| id == candidate_id))
        .and_then(|r| r.params.seed)
}

/// List `<dir>/candidates/*.png` sorted by id, with mtime timestamps and
/// seed lineage from `records`.
pub fn scan_candidates(dir: &Path, records: &[GenRecord]) -> io::Result<Vec<CandidateDto>> {
    let mut out = Vec::new();
    if dir.is_dir() {
        for name in sorted_png_names(dir)? {
            let path = dir.join(&name);
            let id = name.trim_end_matches(".png").to_string();
            out.push(CandidateDto {
                seed: lineage_seed(records, &id),
                path: path.display().to_string(),
                created_at: file_mtime_ms(&path),
                id,
            });
        }
    }
    Ok(out)
}

/// List `<dir>/history/*.png` with ts parsed from file names.
pub fn scan_history(dir: &Path) -> io::Result<Vec<HistoryDto>> {
    let mut out = Vec::new();
    if dir.is_dir() {
        for name in sorted_png_names(dir)? {
            let path = dir.join(&name);
            let ts = history_ts_from_name(&name).unwrap_or_else(|| file_mtime_ms(&path));
            out.push(HistoryDto {
                path: path.display().to_string(),
                ts,
            });
        }
    }
    out.sort_by_key(|h| h.ts);
    Ok(out)
}

fn sorted_png_names(dir: &Path) -> io::Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".png"))
        .collect();
    names.sort();
    Ok(names)
}

/// The candidate id `current.png` was promoted from: the last
/// `page-pick`/`component-pick` entry for that target. The core keeps
/// candidates on disk after a pick, so the id is lineage-only.
fn picked_candidate_id(
    log: &[rudder_core::store::PromptLogEntry],
    pick_kind: &str,
    target: &str,
) -> String {
    log.iter()
        .rev()
        .find(|e| e.kind == pick_kind && e.target == target)
        .and_then(|e| e.prompt.split_whitespace().nth(2))
        .map(|id| id.trim_end_matches(".png").to_string())
        .unwrap_or_default()
}

fn page_view(root: &Path, project: &Project, page: &rudder_core::store::Page) -> io::Result<PageDto> {
    let dir = page.dir(root);
    let candidates = scan_candidates(&dir.join("candidates"), &page.generations)?;
    let current_png = dir.join("current.png");
    let current = if current_png.is_file() {
        Some(CurrentDto {
            path: current_png.display().to_string(),
            candidate_id: picked_candidate_id(&project.prompt_log, "page-pick", &page.slug),
        })
    } else {
        None
    };
    let history = scan_history(&dir.join("history"))?;
    Ok(PageDto {
        slug: page.slug.clone(),
        brief: page.brief.clone(),
        candidates,
        current,
        history,
        updated_at: rfc3339_to_ms(&page.updated_at),
    })
}

fn component_view(
    root: &Path,
    project: &Project,
    component: &rudder_core::store::Component,
) -> io::Result<ComponentDto> {
    let dir = component.dir(root);
    let candidates = scan_candidates(&dir.join("candidates"), &component.generations)?;
    let current_png = dir.join("current.png");
    let current = if current_png.is_file() {
        Some(CurrentDto {
            path: current_png.display().to_string(),
            candidate_id: picked_candidate_id(
                &project.prompt_log,
                "component-pick",
                &component.name,
            ),
        })
    } else {
        None
    };
    let history = scan_history(&dir.join("history"))?;
    Ok(ComponentDto {
        name: component.name.clone(),
        kind: component.kind.clone(),
        brief: component.brief.clone(),
        candidates,
        current,
        history,
        updated_at: rfc3339_to_ms(&component.updated_at),
    })
}

/// `ProjectSummary` (left column list).
pub fn build_summary(project: &Project) -> ProjectSummaryDto {
    ProjectSummaryDto {
        id: project.id.clone(),
        name: project.name.clone(),
        size: canvas_view(&project.canvas_size),
        created_at: rfc3339_to_ms(&project.created_at),
        has_anchor: project.anchor.is_some(),
        page_count: project.pages.len(),
        component_count: project.components.len(),
    }
}

/// `ProjectDetail` (full gallery view): project.json + on-disk scans.
pub fn build_detail(root: &Path, project: &Project) -> io::Result<ProjectDetailDto> {
    let board_candidates = scan_candidates(
        &root.join("board").join("candidates"),
        &project.board_generations,
    )?;
    let anchor = project.anchor.as_ref().map(|a| AnchorDto {
        candidate_id: a.candidate_id.clone(),
        path: root.join("board").join("anchor.png").display().to_string(),
        created_at: rfc3339_to_ms(&a.created_at),
    });
    let mut pages = Vec::with_capacity(project.pages.len());
    for page in &project.pages {
        pages.push(page_view(root, project, page)?);
    }
    let mut components = Vec::with_capacity(project.components.len());
    for component in &project.components {
        components.push(component_view(root, project, component)?);
    }
    Ok(ProjectDetailDto {
        id: project.id.clone(),
        name: project.name.clone(),
        size: canvas_view(&project.canvas_size),
        brand_brief: project.brand_brief.clone(),
        style_brief: project.style_brief.clone(),
        anchor,
        board_candidates,
        pages,
        components,
        created_at: rfc3339_to_ms(&project.created_at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rudder_core::canvas::CanvasSize;
    use rudder_core::store::{self, GenParams, GenRecord, Page, Project, PromptLogEntry};

    fn tmp_root(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rudder-view-{tag}-{}", uuid::Uuid::new_v4()))
    }

    fn record(id: &str, seed: Option<u64>) -> GenRecord {
        GenRecord {
            at: store::now_rfc3339(),
            endpoint: "generations".into(),
            prompt: "p".into(),
            params: GenParams {
                model: "gpt-image-2".into(),
                size: "1536x1024".into(),
                quality: "high".into(),
                n: 1,
                seed,
                thinking: None,
            },
            candidate_ids: vec![id.to_string()],
        }
    }

    #[test]
    fn history_names_parse_to_epoch_millis() {
        let ms = history_ts_from_name("20260908-012345678.png").expect("parses");
        let parsed = Utc.timestamp_millis_opt(ms).unwrap();
        assert_eq!(parsed.format("%Y%m%d-%H%M%S%3f").to_string(), "20260908-012345678");
        assert!(history_ts_from_name("not-a-ts.png").is_none());
        assert!(history_ts_from_name("anchor.png").is_none());
    }

    #[test]
    fn presets_normalize_to_custom_when_unnamed() {
        assert_eq!(preset_name(Some(Preset::Web)), "web");
        assert_eq!(preset_name(None), "custom");
        let canvas = CanvasSize::new(1536, 1024, None);
        assert_eq!(canvas_view(&canvas).preset, "custom");
    }

    #[test]
    fn candidates_scan_sorted_with_seed_lineage() {
        let root = tmp_root("cands");
        let board = root.join("board");
        store::atomic_write(&store::candidate_path(&board, "0002"), b"x").unwrap();
        store::atomic_write(&store::candidate_path(&board, "0001"), b"x").unwrap();
        let records = vec![record("0002", Some(42))];
        let out = scan_candidates(&board.join("candidates"), &records).unwrap();
        assert_eq!(out.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["0001", "0002"]);
        assert_eq!(out[1].seed, Some(42));
        assert!(out[0].seed.is_none());
        assert!(out[1].path.ends_with("candidates/0002.png"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn detail_view_mirrors_frontend_shapes() {
        let root = tmp_root("detail");
        let mut project = Project::new(
            "样例",
            CanvasSize::new(1536, 1024, Some(Preset::Web)),
            "brand",
            "style",
        );
        store::create_project(&root, &project).unwrap();
        store::atomic_write(&store::candidate_path(&root.join("board"), "0001"), b"x").unwrap();
        project.board_generations.push(record("0001", Some(7)));
        let mut page = Page::new("dashboard".into(), "brief".into());
        page.generations.push(record("0001", None));
        project.pages.push(page);
        // anchor picked from candidate 0001
        project.anchor = Some(rudder_core::store::Anchor {
            candidate_id: "0001".into(),
            prompt: "p".into(),
            seed: Some(7),
            created_at: store::now_rfc3339(),
        });
        // page-pick lineage entry: "[page-pick] dashboard 0001 (previous → …)"
        project.prompt_log.push(PromptLogEntry {
            at: store::now_rfc3339(),
            kind: "page-pick".into(),
            target: "dashboard".into(),
            endpoint: "-".into(),
            prompt: "[page-pick] dashboard 0001 (previous → history/x.png)".into(),
            params: GenParams {
                model: "-".into(),
                size: "-".into(),
                quality: "-".into(),
                n: 0,
                seed: None,
                thinking: None,
            },
            candidate_ids: vec!["0001".into()],
            dry_run: false,
        });
        store::save_project(&root, &project).unwrap();
        store::atomic_write(&root.join("pages/dashboard/current.png"), b"current").unwrap();

        let detail = build_detail(&root, &project).unwrap();
        assert_eq!(detail.size.preset, "web");
        let summary = build_summary(&project);
        assert!(summary.has_anchor);
        assert_eq!(summary.page_count, 1);
        let anchor = detail.anchor.as_ref().expect("anchor present");
        assert_eq!(anchor.candidate_id, "0001");
        assert!(anchor.path.ends_with("board/anchor.png"));
        assert_eq!(detail.board_candidates[0].seed, Some(7));
        let page = &detail.pages[0];
        assert_eq!(page.slug, "dashboard");
        assert_eq!(page.current.as_ref().unwrap().candidate_id, "0001");
        let json = serde_json::to_value(&detail).unwrap();
        assert!(json["boardCandidates"].is_array());
        assert_eq!(json["pages"][0]["current"]["candidateId"], "0001");
        std::fs::remove_dir_all(&root).ok();
    }
}
