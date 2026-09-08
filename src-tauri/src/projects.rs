//! Desktop-side project catalog: `~/Rudder/projects` resolution, id → root
//! lookup, `~` expansion for user-typed export paths and export file scans.

use rudder_core::error::{Result as CoreResult, RudderError};
use rudder_core::store::{self, Project};
use std::path::{Path, PathBuf};

use crate::view::ExportFileDto;

/// `~/Rudder/projects` (ARCHITECTURE §3); created on demand.
pub fn projects_root() -> CoreResult<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        RudderError::InvalidArg {
            detail: "home directory not found; cannot resolve ~/Rudder".into(),
        }
    })?;
    let root = home.join("Rudder").join("projects");
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

/// Fresh unique project directory `~/Rudder/projects/<uuid>`; the id inside
/// `project.json` is the lookup key (dir name is a stable unique label).
pub fn new_project_dir() -> CoreResult<PathBuf> {
    Ok(projects_root()?.join(uuid::Uuid::new_v4().to_string()))
}

/// Every project found under the projects root, newest first.
pub fn list_projects() -> CoreResult<Vec<(PathBuf, Project)>> {
    let root = projects_root()?;
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir = entry.path();
        if !store::is_project_dir(&dir) {
            continue;
        }
        // Corrupt neighbors must not hide healthy projects.
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
        let dir = new_project_dir().unwrap();
        let project = Project::new("probe", rudder_core::canvas::CanvasSize::new(1536, 1024, None), "", "");
        store::create_project(&dir, &project).unwrap();
        let found = find_project_root(&project.id).unwrap();
        assert_eq!(found, dir);
        let err = find_project_root("nope").unwrap_err();
        assert_eq!(err.code(), "NOT_FOUND");
        std::fs::remove_dir_all(&dir).ok();
    }
}
