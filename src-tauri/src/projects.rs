//! Desktop-side project catalog: `~/Rudder/projects` resolution, id → root
//! lookup, `~` expansion for user-typed export paths and export file scans.
//!
//! The catalog lists two sources (ARCHITECTURE §3/§6): projects under the
//! scan root `~/Rudder/projects` **plus** CLI-created projects registered
//! in `~/Rudder/registry.json` ([`rudder_core::registry`]) — so work done
//! through the CLI surfaces in the desktop app and vice versa.

use rudder_core::error::{Result as CoreResult, RudderError};
use rudder_core::registry;
use rudder_core::store::{self, Project};
use std::path::{Path, PathBuf};

use crate::view::ExportFileDto;

/// `~/Rudder/projects` (ARCHITECTURE §3); created on demand.
pub fn projects_root() -> CoreResult<PathBuf> {
    registry::projects_root()
}

/// Fresh unique project directory `~/Rudder/projects/<uuid>`; the id inside
/// `project.json` is the lookup key (dir name is a stable unique label).
pub fn new_project_dir() -> CoreResult<PathBuf> {
    registry::new_project_dir()
}

/// Canonical form when the path exists, else the input unchanged.
fn canonical(dir: &Path) -> PathBuf {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

/// Every project visible to the desktop catalog, newest first: the scan
/// root plus registered external roots (self-healed, deduped by canonical
/// path). Corrupt neighbors must not hide healthy projects.
pub fn list_projects() -> CoreResult<Vec<(PathBuf, Project)>> {
    let root = projects_root()?;
    // Drop registry entries whose project directory vanished (best effort).
    let _ = registry::prune();

    let mut roots: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir = entry.path();
        if !store::is_project_dir(&dir) {
            continue;
        }
        seen.push(canonical(&dir));
        roots.push(dir);
    }
    for dir in registry::list_valid().unwrap_or_default() {
        let canon = canonical(&dir);
        if !seen.contains(&canon) {
            seen.push(canon);
            roots.push(dir);
        }
    }

    let mut out = Vec::new();
    for dir in roots {
        if let Ok(project) = store::load_project(&dir) {
            out.push((dir, project));
        }
    }
    out.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
    Ok(out)
}

/// Find a project root by its `project.json` id.
pub fn find_project_root(project_id: &str) -> CoreResult<PathBuf> {
    list_projects()?
        .into_iter()
        .map(|(dir, project)| (dir, project.id))
        .find(|(_, id)| id == project_id)
        .map(|(dir, _)| dir)
        .ok_or_else(|| RudderError::NotFound {
            what: format!("project `{project_id}`"),
        })
}

/// Expand a leading `~` (the desktop UI seeds export dirs like
/// `~/Rudder/exports/<project>`); everything else passes through.
pub fn expand_home(input: &str) -> CoreResult<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RudderError::InvalidArg {
            detail: "export directory must not be empty".into(),
        });
    }
    if trimmed == "~" {
        return dirs::home_dir().ok_or_else(|| RudderError::InvalidArg {
            detail: "home directory not found; expand the path manually".into(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        let home = dirs::home_dir().ok_or_else(|| RudderError::InvalidArg {
            detail: "home directory not found; expand the path manually".into(),
        })?;
        return Ok(home.join(rest));
    }
    Ok(PathBuf::from(trimmed))
}

/// Classification for export artifacts (`ExportedFile.kind`).
fn export_kind(rel: &str) -> &'static str {
    match rel {
        "manifest.json" => "manifest",
        "PROMPTS.md" => "prompts",
        "DESIGN.template.md" => "template",
        _ => "image",
    }
}

fn scan_export_dir(dir: &Path, rel_prefix: &str, out: &mut Vec<ExportFileDto>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = dir.join(&name);
        let rel = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        if path.is_dir() {
            scan_export_dir(&path, &rel, out);
        } else if path.is_file() {
            out.push(ExportFileDto {
                bytes: path.metadata().map(|m| m.len()).unwrap_or(0),
                path: rel,
                kind: export_kind(&name).to_string(),
            });
        }
    }
}

/// Export bundle files sorted by relative path (paths relative to the export
/// root, mirroring the mock adapter's `ExportedFile.path`).
pub fn scan_export_files(out_dir: &Path) -> CoreResult<Vec<ExportFileDto>> {
    let mut files = Vec::new();
    scan_export_dir(out_dir, "", &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    /// Redirect `~/Rudder` resolution into a per-run temp directory so the
    /// catalog tests never touch the developer's real home.
    fn ensure_test_home() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let dir =
                std::env::temp_dir().join(format!("rudder-tauri-home-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).expect("create test home");
            // Safety: single-threaded init via Once; other tests only read
            // the override after it is set.
            std::env::set_var("RUDDER_HOME", &dir);
        });
    }

    fn sample_project(name: &str, dir: &Path) -> Project {
        let project = Project::new(name, rudder_core::canvas::CanvasSize::new(1536, 1024, None), "", "");
        store::create_project(dir, &project).unwrap();
        project
    }

    #[test]
    fn expand_home_handles_tilde_and_empty() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_home("~/Rudder/exports").unwrap(), home.join("Rudder/exports"));
        assert_eq!(expand_home("~").unwrap(), home);
        assert_eq!(expand_home("/tmp/out").unwrap(), PathBuf::from("/tmp/out"));
        assert!(expand_home("  ").is_err());
    }

    #[test]
    fn export_scan_classifies_kinds_and_sorts() {
        let root = std::env::temp_dir().join(format!("rudder-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("board/candidates")).unwrap();
        std::fs::write(root.join("board/anchor.png"), b"x").unwrap();
        std::fs::write(root.join("board/candidates/0001.png"), b"x").unwrap();
        std::fs::write(root.join("manifest.json"), b"{}").unwrap();
        std::fs::write(root.join("PROMPTS.md"), b"# prompts").unwrap();
        std::fs::write(root.join("DESIGN.template.md"), b"# design").unwrap();
        let files = scan_export_files(&root).unwrap();
        let kinds: Vec<(&str, &str)> = files
            .iter()
            .map(|f| (f.path.as_str(), f.kind.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                ("DESIGN.template.md", "template"),
                ("PROMPTS.md", "prompts"),
                ("board/anchor.png", "image"),
                ("board/candidates/0001.png", "image"),
                ("manifest.json", "manifest"),
            ]
        );
        assert!(files.iter().all(|f| f.bytes > 0));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_project_root_roundtrips_by_id() {
        ensure_test_home();
        let dir = new_project_dir().unwrap();
        let project = sample_project("probe", &dir);
        let found = find_project_root(&project.id).unwrap();
        assert_eq!(found, dir);
        let err = find_project_root("nope").unwrap_err();
        assert_eq!(err.code(), "NOT_FOUND");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_merges_registered_external_projects_and_self_heals() {
        ensure_test_home();
        // A project under the scan root (GUI-style creation).
        let scan_dir = new_project_dir().unwrap();
        let scan_project = sample_project("扫描内", &scan_dir);
        // A project outside the scan root (CLI `--dir` style) → registered.
        let external = std::env::temp_dir()
            .join(format!("rudder-tauri-ext-{}", uuid::Uuid::new_v4()));
        let external_project = sample_project("外部登记", &external);
        assert!(registry::register(&external).unwrap(), "external init registers");

        let listed = list_projects().unwrap();
        let names: Vec<&str> = listed.iter().map(|(_, p)| p.name.as_str()).collect();
        assert!(names.contains(&"扫描内"), "scan-root project listed: {names:?}");
        assert!(names.contains(&"外部登记"), "registered project merged: {names:?}");
        assert!(
            listed.iter().any(|(dir, _)| dir == &external.canonicalize().unwrap()),
            "external entry uses its canonical path"
        );

        // find_project_root resolves registered external projects by id too.
        assert_eq!(
            find_project_root(&external_project.id).unwrap(),
            external.canonicalize().unwrap()
        );
        assert_eq!(find_project_root(&scan_project.id).unwrap(), scan_dir);

        // Self-heal: once the external directory vanishes it disappears from
        // the catalog while the scan-root project remains.
        std::fs::remove_dir_all(&external).unwrap();
        let after = list_projects().unwrap();
        let names: Vec<&str> = after.iter().map(|(_, p)| p.name.as_str()).collect();
        assert!(!names.contains(&"外部登记"), "stale entry self-healed: {names:?}");
        assert!(names.contains(&"扫描内"));
        std::fs::remove_dir_all(&scan_dir).ok();
    }
}
