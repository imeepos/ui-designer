//! Project storage (ARCHITECTURE §3): `project.json` model, atomic writes,
//! on-disk layout, candidate numbering and pick/history mechanics.
//!
//! Layout under `<project-root>/`:
//! ```text
//! project.json
//! board/candidates/NNNN.png · board/anchor.png
//! pages/<slug>/{candidates/, current.png, history/}
//! components/<name>/{candidates/, current.png, history/}
//! refs/
//! ```

use crate::canvas::CanvasSize;
use crate::error::{Result, RudderError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Marker file name at the project root.
pub const PROJECT_FILE: &str = "project.json";
/// Width of candidate ids (`0001`).
const CANDIDATE_WIDTH: usize = 4;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// Image generation parameters as recorded for lineage (manifest/PROMPTS.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenParams {
    pub model: String,
    /// `"1536x1024"` form.
    pub size: String,
    pub quality: String,
    pub n: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// One real generation batch: the exact prompt/params and the candidate ids
/// it produced. Powers per-image lineage in `manifest.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenRecord {
    pub at: String,
    /// `generations` or `edits` endpoint kind.
    pub endpoint: String,
    pub prompt: String,
    pub params: GenParams,
    pub candidate_ids: Vec<String>,
}

/// The picked design-system board (the style anchor).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub candidate_id: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub created_at: String,
}

/// A registered functional page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub slug: String,
    pub brief: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generations: Vec<GenRecord>,
}

impl Page {
    pub fn new(slug: String, brief: String) -> Page {
        Page {
            slug,
            brief,
            prompt: None,
            seed: None,
            updated_at: now_rfc3339(),
            generations: Vec::new(),
        }
    }

    pub fn dir(&self, root: &Path) -> PathBuf {
        root.join("pages").join(&self.slug)
    }
}

/// A registered component sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    /// Component type: buttons/forms/cards/navigation/icons/tables/modals, …
    #[serde(rename = "type")]
    pub kind: String,
    pub brief: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generations: Vec<GenRecord>,
}

impl Component {
    pub fn new(name: String, kind: String, brief: String) -> Component {
        Component {
            name,
            kind,
            brief,
            prompt: None,
            seed: None,
            updated_at: now_rfc3339(),
            generations: Vec::new(),
        }
    }

    pub fn dir(&self, root: &Path) -> PathBuf {
        root.join("components").join(&self.name)
    }
}

/// One entry of the append-only prompt log (PRD §3.5 / ARCHITECTURE §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptLogEntry {
    pub at: String,
    /// `board` | `page` | `component`.
    pub kind: String,
    /// Page slug / component name / empty for board.
    pub target: String,
    pub endpoint: String,
    pub prompt: String,
    pub params: GenParams,
    pub candidate_ids: Vec<String>,
    /// True when this entry came from a dry-run plan (never persisted by CLI,
    /// but kept for parity with future desktop-side logging).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dry_run: bool,
}

/// Root project metadata — `project.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(rename = "canvasSize")]
    pub canvas_size: CanvasSize,
    #[serde(default)]
    pub brand_brief: String,
    #[serde(default)]
    pub style_brief: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<Page>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    /// Lineage of board generation batches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub board_generations: Vec<GenRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_log: Vec<PromptLogEntry>,
    pub created_at: String,
}

impl Project {
    /// Fresh project with `name`/briefs/canvas; id and timestamps filled in.
    pub fn new(name: &str, canvas_size: CanvasSize, brand_brief: &str, style_brief: &str) -> Project {
        Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            canvas_size,
            brand_brief: brand_brief.to_string(),
            style_brief: style_brief.to_string(),
            anchor: None,
            pages: Vec::new(),
            components: Vec::new(),
            board_generations: Vec::new(),
            prompt_log: Vec::new(),
            created_at: now_rfc3339(),
        }
    }

    pub fn page(&self, slug: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.slug == slug)
    }

    pub fn component(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.name == name)
    }
}

/// Current UTC timestamp in RFC 3339 (storage form).
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Compact timestamp for `history/<ts>.png` file names.
pub fn now_history_ts() -> String {
    Utc::now().format("%Y%m%d-%H%M%S%3f").to_string()
}

// ---------------------------------------------------------------------------
// Atomic IO
// ---------------------------------------------------------------------------

/// Write `bytes` to `path` atomically: tempfile (same directory) + rename.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::write(&tmp, bytes)?;
    // Best effort durability before the rename.
    if let Ok(f) = std::fs::File::open(&tmp) {
        let _ = f.sync_all();
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Serialize `value` and write it to `path` atomically (pretty JSON + newline).
pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| RudderError::InvalidArg { detail: format!("serialization failed: {e}") })?;
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}

// ---------------------------------------------------------------------------
// Project lifecycle
// ---------------------------------------------------------------------------

/// `true` when `dir` contains a `project.json`.
pub fn is_project_dir(dir: &Path) -> bool {
    dir.join(PROJECT_FILE).is_file()
}

/// Create the full project directory layout and write `project.json`.
pub fn create_project(dir: &Path, project: &Project) -> Result<()> {
    if is_project_dir(dir) {
        return Err(RudderError::AlreadyExists {
            what: format!("project at {}", dir.display()),
        });
    }
    std::fs::create_dir_all(dir)?;
    for sub in [
        "board/candidates",
        "pages",
        "components",
        "refs",
    ] {
        std::fs::create_dir_all(dir.join(sub))?;
    }
    save_project(dir, project)
}

/// Load and parse `project.json` from a project root.
pub fn load_project(dir: &Path) -> Result<Project> {
    let path = dir.join(PROJECT_FILE);
    let bytes = std::fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => RudderError::NotAProject { path: dir.display().to_string() },
        _ => RudderError::Io(e),
    })?;
    serde_json::from_slice(&bytes).map_err(|e| RudderError::InvalidArg {
        detail: format!("{} is corrupt: {e}", path.display()),
    })
}

/// Persist `project.json` atomically.
pub fn save_project(dir: &Path, project: &Project) -> Result<()> {
    atomic_write_json(&dir.join(PROJECT_FILE), project)
}

/// Resolve the project root: explicit `--project`, else cwd if it is a
/// project, else the last-used project from config (ARCHITECTURE §6).
pub fn resolve_project_root(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        let dir = normalize(dir);
        if !is_project_dir(&dir) {
            return Err(RudderError::NotAProject { path: dir.display().to_string() });
        }
        return Ok(dir);
    }
    let cwd = std::env::current_dir()?;
    if is_project_dir(&cwd) {
        return Ok(cwd);
    }
    if let Some(last) = crate::config::Config::load().last_project {
        let last = normalize(&last);
        if is_project_dir(&last) {
            return Ok(last);
        }
        return Err(RudderError::NotFound {
            what: format!("last-used project at {}", last.display()),
        });
    }
    Err(RudderError::NoProject)
}

fn normalize(dir: &Path) -> PathBuf {
    // Best effort absolutization without requiring the path to exist.
    if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(dir))
            .unwrap_or_else(|_| dir.to_path_buf())
    }
}

// ---------------------------------------------------------------------------
// Candidates / picks / history
// ---------------------------------------------------------------------------

/// Directory that holds `NNNN.png` candidates for a target.
pub fn candidates_dir(target_dir: &Path) -> PathBuf {
    target_dir.join("candidates")
}

/// Highest existing candidate number in `target_dir/candidates` (0 if none).
fn max_candidate(target_dir: &Path) -> Result<u32> {
    let dir = candidates_dir(target_dir);
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut max = 0u32;
    for entry in std::fs::read_dir(&dir)? {
        let name = entry?.file_name();
        let name = name.to_string_lossy();
        if let Some(stem) = name.strip_suffix(".png") {
            if let Ok(n) = stem.parse::<u32>() {
                max = max.max(n);
            }
        }
    }
    Ok(max)
}

/// Allocate the next `CANDIDATE_WIDTH`-digit candidate id, e.g. `0002`.
pub fn next_candidate_id(target_dir: &Path) -> Result<String> {
    let next = max_candidate(target_dir)? + 1;
    Ok(format!("{:0width$}", next, width = CANDIDATE_WIDTH))
}

/// Reserve `count` consecutive candidate ids starting at the current max+1.
/// Returns the ids in order. (Allocation happens before the HTTP call so a
/// concurrent dry-run cannot collide; gaps after a failed call are fine.)
pub fn reserve_candidate_ids(target_dir: &Path, count: usize) -> Result<Vec<String>> {
    let start = max_candidate(target_dir)? + 1;
    Ok((start..)
        .take(count)
        .map(|next| format!("{next:0width$}", width = CANDIDATE_WIDTH))
        .collect())
}

/// Candidate file path for `target_dir/candidates/<id>.png`.
pub fn candidate_path(target_dir: &Path, id: &str) -> PathBuf {
    candidates_dir(target_dir).join(format!("{id}.png"))
}

/// Copy a candidate to `current.png`, moving any previous current into
/// `history/<ts>.png`. Returns the history file name if one was rotated.
pub fn promote_candidate(target_dir: &Path, id: &str, current_name: &str) -> Result<Option<String>> {
    let src = candidate_path(target_dir, id);
    if !src.is_file() {
        return Err(RudderError::NotFound {
            what: format!("candidate `{id}` under {}", candidates_dir(target_dir).display()),
        });
    }
    let current = target_dir.join(current_name);
    let mut rotated = None;
    if current.is_file() {
        let ts = now_history_ts();
        let hist = target_dir.join("history").join(format!("{ts}.png"));
        std::fs::create_dir_all(target_dir.join("history"))?;
        std::fs::rename(&current, &hist)?;
        rotated = Some(format!("history/{ts}.png"));
    }
    std::fs::copy(&src, &current)?;
    Ok(rotated)
}

/// Count `NNNN.png` files under `target_dir/candidates`.
pub fn count_candidates(target_dir: &Path) -> Result<usize> {
    let dir = candidates_dir(target_dir);
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut n = 0usize;
    for entry in std::fs::read_dir(&dir)? {
        let name = entry?.file_name();
        if name.to_string_lossy().ends_with(".png") {
            n += 1;
        }
    }
    Ok(n)
}

/// Validate a page slug / component name: `a-z0-9-`, starting alphanumeric.
pub fn validate_identifier(value: &str, what: &str) -> Result<()> {
    let ok = !value.is_empty()
        && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && value.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && !value.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(RudderError::InvalidArg {
            detail: format!(
                "invalid {what} `{value}`: use lowercase a-z, 0-9 and '-', starting and ending alphanumeric"
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rudder-store-{tag}-{}", uuid::Uuid::new_v4()))
    }

    fn sample_project() -> Project {
        Project::new("样例", CanvasSize::new(1536, 1024, Some(crate::canvas::Preset::Web)), "品牌", "风格")
    }

    #[test]
    fn create_layout_matches_architecture() {
        let root = tmp_root("layout");
        let proj = sample_project();
        create_project(&root, &proj).unwrap();
        for sub in ["project.json", "board/candidates", "pages", "components", "refs"] {
            assert!(root.join(sub).exists(), "{sub} must exist");
        }
        let loaded = load_project(&root).unwrap();
        assert_eq!(loaded, proj);
        assert_eq!(loaded.name, "样例");
        assert_eq!(loaded.canvas_size.w, 1536);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn create_twice_is_already_exists() {
        let root = tmp_root("dup");
        create_project(&root, &sample_project()).unwrap();
        let err = create_project(&root, &sample_project()).unwrap_err();
        assert_eq!(err.code(), "ALREADY_EXISTS");
        assert_eq!(err.exit_code(), 3);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn atomic_write_leaves_no_tmp_and_roundtrips() {
        let root = tmp_root("atomic");
        create_project(&root, &sample_project()).unwrap();

        // Mutate and save repeatedly; each save must replace atomically.
        for i in 1..=5u32 {
            let mut proj = load_project(&root).unwrap();
            proj.style_brief = format!("iteration-{i}");
            save_project(&root, &proj).unwrap();
            let reloaded = load_project(&root).unwrap();
            assert_eq!(reloaded.style_brief, format!("iteration-{i}"));
        }
        // No stray temp files remain in the root.
        let strays: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(strays.is_empty(), "stray temp files: {strays:?}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn atomic_write_json_serializes_pretty() {
        let dir = tmp_root("json");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data.json");
        atomic_write_json(&path, &serde_json::json!({ "a": 1 })).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with('{'));
        assert!(text.ends_with("}\n"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn candidate_ids_are_sequential_four_digit() {
        let root = tmp_root("cands");
        let board = root.join("board");
        std::fs::create_dir_all(candidates_dir(&board)).unwrap();
        assert_eq!(next_candidate_id(&board).unwrap(), "0001");
        std::fs::write(candidate_path(&board, "0001"), b"x").unwrap();
        assert_eq!(next_candidate_id(&board).unwrap(), "0002");
        let ids = reserve_candidate_ids(&board, 3).unwrap();
        assert_eq!(ids, vec!["0002", "0003", "0004"]);
        // A gap in numbering does not break allocation.
        std::fs::write(candidate_path(&board, "0042"), b"x").unwrap();
        assert_eq!(next_candidate_id(&board).unwrap(), "0043");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn promote_rotates_current_into_history() {
        let root = tmp_root("promote");
        let page_dir = root.join("pages").join("dash");
        std::fs::create_dir_all(candidates_dir(&page_dir)).unwrap();
        std::fs::write(candidate_path(&page_dir, "0001"), b"first").unwrap();
        std::fs::write(candidate_path(&page_dir, "0002"), b"second").unwrap();

        assert_eq!(promote_candidate(&page_dir, "0001", "current.png").unwrap(), None);
        assert_eq!(std::fs::read(page_dir.join("current.png")).unwrap(), b"first");

        let rotated = promote_candidate(&page_dir, "0002", "current.png").unwrap();
        let rotated = rotated.expect("old current must rotate into history");
        assert!(rotated.starts_with("history/"));
        assert_eq!(std::fs::read(page_dir.join("current.png")).unwrap(), b"second");
        assert_eq!(
            std::fs::read(page_dir.join(&rotated)).unwrap(),
            b"first",
            "old current bytes preserved in history"
        );

        let err = promote_candidate(&page_dir, "9999", "current.png").unwrap_err();
        assert_eq!(err.code(), "NOT_FOUND");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_requires_a_project() {
        let root = tmp_root("resolve");
        // Explicit but nonexistent dir.
        let err = resolve_project_root(Some(&root)).unwrap_err();
        assert_eq!(err.code(), "NOT_A_PROJECT");
        create_project(&root, &sample_project()).unwrap();
        assert_eq!(resolve_project_root(Some(&root)).unwrap(), root);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn identifier_validation() {
        assert!(validate_identifier("dashboard", "slug").is_ok());
        assert!(validate_identifier("button-set-2", "slug").is_ok());
        for bad in ["", "-lead", "trail-", "UPPER", "has space", "下划线", "a/b"] {
            assert!(validate_identifier(bad, "slug").is_err(), "{bad:?}");
        }
    }

    #[test]
    fn corrupted_project_json_is_invalid_arg() {
        let root = tmp_root("corrupt");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(PROJECT_FILE), b"{not json").unwrap();
        let err = load_project(&root).unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
        std::fs::remove_dir_all(&root).ok();
    }
}
