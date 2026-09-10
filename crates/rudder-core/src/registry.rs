//! Shared project catalog: `~/Rudder/registry.json` (ARCHITECTURE §3/§6).
//!
//! The desktop shell lists projects by scanning `~/Rudder/projects`; the
//! CLI creates projects wherever the operator points it (`--dir`, agent
//! workspaces). This module is the bridge: projects living **outside** the
//! scan root are registered here so both frontends surface the same catalog.
//!
//! Best practices encoded here:
//! - entries are canonical absolute paths (relative/symlink aliases dedupe)
//! - writes are atomic (tempfile + rename, [`crate::store::atomic_write_json`])
//! - a corrupt registry file degrades to an empty registry (never panics)
//! - reads are self-healing: entries whose project directory vanished are
//!   filtered; [`prune_at`] persists that cleanup ([`list_at`] stays pure)

use crate::error::{Result, RudderError};
use crate::store::PROJECT_FILE;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Serializes registry read-modify-write cycles within one process (the
/// test harness runs many inits in parallel; the atomic rename alone would
/// not prevent lost updates). Cross-process writers are rare and the next
/// registration self-heals the catalog.
static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

/// Registry file name under the Rudder home (`~/Rudder/registry.json`).
pub const REGISTRY_FILE: &str = "registry.json";

/// Registered external project roots, persisted as absolute canonical paths.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectRegistry {
    /// Canonical project roots in registration order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<PathBuf>,
}

/// Rudder data directory (`~/Rudder`); `RUDDER_HOME` overrides it directly
/// (same convention as [`crate::config::Config::path`]: the env var names
/// the Rudder directory itself, not the OS home).
pub fn rudder_home() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("RUDDER_HOME") {
        if !home.trim().is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    dirs::home_dir().map(|home| home.join("Rudder"))
}

/// `~/Rudder/registry.json` path; `None` when no home directory is known.
pub fn registry_path() -> Option<PathBuf> {
    rudder_home().map(|home| home.join(REGISTRY_FILE))
}

/// The well-known GUI scan root `~/Rudder/projects`, created on demand.
/// Shared by the desktop shell and the CLI's default `init` location.
pub fn projects_root() -> Result<PathBuf> {
    let home = rudder_home().ok_or_else(|| RudderError::InvalidArg {
        detail: "home directory not found; cannot resolve ~/Rudder".into(),
    })?;
    projects_root_at(&home)
}

/// [`projects_root`] for an explicit home directory (testable form).
pub fn projects_root_at(home: &Path) -> Result<PathBuf> {
    let root = home.join("Rudder").join("projects");
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

/// Fresh unique project directory under the projects root
/// (`~/Rudder/projects/<uuid>`); the id inside `project.json` is the lookup
/// key, the directory name is a stable unique label.
pub fn new_project_dir() -> Result<PathBuf> {
    new_project_dir_at(&rudder_home().ok_or_else(|| RudderError::InvalidArg {
        detail: "home directory not found; cannot resolve ~/Rudder".into(),
    })?)
}

/// [`new_project_dir`] for an explicit home directory (testable form).
pub fn new_project_dir_at(home: &Path) -> Result<PathBuf> {
    Ok(projects_root_at(home)?.join(uuid::Uuid::new_v4().to_string()))
}

/// Best-effort absolutization without requiring the path to exist.
fn absolutize(dir: &Path) -> PathBuf {
    if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(dir))
            .unwrap_or_else(|_| dir.to_path_buf())
    }
}

/// Canonical form when the path exists; when the leaf is gone (unregister
/// after delete), canonicalize the nearest existing ancestor and re-join the
/// leaf name so symlinked ancestors (`/tmp` → `/private/tmp`, macOS) still
/// match stored forms. Falls back to the absolutized input.
fn canonical_or_absolutize(dir: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(dir) {
        return canonical;
    }
    if let Some(parent) = dir.parent() {
        if let Ok(parent_canonical) = std::fs::canonicalize(parent) {
            if let Some(name) = dir.file_name() {
                return parent_canonical.join(name);
            }
        }
    }
    absolutize(dir)
}

/// Load the registry stored at `path`; a missing or corrupt file yields the
/// default (empty) registry — the catalog must never hard-fail a listing.
pub fn load_at(path: Option<&Path>) -> ProjectRegistry {
    let Some(path) = path else { return ProjectRegistry::default() };
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => ProjectRegistry::default(),
    }
}

/// Persist the registry atomically (tempfile + rename).
pub fn save_at(path: Option<&Path>, registry: &ProjectRegistry) -> Result<()> {
    let Some(path) = path else {
        return Err(RudderError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no home directory known; cannot locate ~/Rudder/registry.json",
        )));
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::store::atomic_write_json(path, registry)
}

/// Register an existing project root (canonicalized, deduped). Idempotent:
/// re-registering a known root keeps the entry in its original position and
/// reports `false`.
pub fn register_at(path: Option<&Path>, dir: &Path) -> Result<bool> {
    let canonical = canonical_or_absolutize(dir);
    if !canonical.join(PROJECT_FILE).is_file() {
        return Err(RudderError::NotAProject {
            path: canonical.display().to_string(),
        });
    }
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut registry = load_at(path);
    if registry.projects.iter().any(|p| p == &canonical) {
        return Ok(false);
    }
    registry.projects.push(canonical);
    save_at(path, &registry)?;
    Ok(true)
}

/// Remove a project root from the registry. Matches the canonical form of
/// the input, so it keeps working after the directory (or a symlink target)
/// was deleted. Returns `true` when an entry was removed; missing entries
/// are not an error.
pub fn unregister_at(path: Option<&Path>, dir: &Path) -> Result<bool> {
    let canonical = canonical_or_absolutize(dir);
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut registry = load_at(path);
    let before = registry.projects.len();
    registry.projects.retain(|p| p != &canonical);
    let removed = registry.projects.len() != before;
    if removed {
        save_at(path, &registry)?;
    }
    Ok(removed)
}

/// Registry entries whose project directory still exists (self-healing
/// read; does not persist the filtered-out entries).
pub fn list_valid_at(path: Option<&Path>) -> Result<Vec<PathBuf>> {
    Ok(load_at(path)
        .projects
        .into_iter()
        .filter(|p| p.join(PROJECT_FILE).is_file())
        .collect())
}

/// Persist the self-healing cleanup: drop entries whose project directory
/// vanished. Returns the number of removed entries.
pub fn prune_at(path: Option<&Path>) -> Result<usize> {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut registry = load_at(path);
    let before = registry.projects.len();
    registry.projects.retain(|p| p.join(PROJECT_FILE).is_file());
    let removed = before - registry.projects.len();
    if removed > 0 {
        save_at(path, &registry)?;
    }
    Ok(removed)
}

// ---------------------------------------------------------------------------
// Home-based wrappers (production entry points)
// ---------------------------------------------------------------------------

/// [`register_at`] against the real `~/Rudder/registry.json`.
pub fn register(dir: &Path) -> Result<bool> {
    register_at(registry_path().as_deref(), dir)
}

/// [`unregister_at`] against the real `~/Rudder/registry.json`.
pub fn unregister(dir: &Path) -> Result<bool> {
    unregister_at(registry_path().as_deref(), dir)
}

/// [`list_valid_at`] against the real `~/Rudder/registry.json`.
pub fn list_valid() -> Result<Vec<PathBuf>> {
    list_valid_at(registry_path().as_deref())
}

/// [`prune_at`] against the real `~/Rudder/registry.json`.
pub fn prune() -> Result<usize> {
    prune_at(registry_path().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;

    fn tmp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rudder-registry-{tag}-{}", uuid::Uuid::new_v4()))
    }

    fn sample_project(dir: &Path) {
        let project = crate::store::Project::new(
            "probe",
            crate::canvas::CanvasSize::new(1536, 1024, None),
            "",
            "",
        );
        store::create_project(dir, &project).unwrap();
    }

    #[test]
    fn register_canonicalizes_dedupes_and_is_idempotent() {
        let root = tmp_root("dedupe");
        let project_dir = root.join("real");
        sample_project(&project_dir);

        // Alias forms of the same directory: relative, symlink, trailing junk.
        let alias = root.join("link");
        std::os::unix::fs::symlink(&project_dir, &alias).unwrap();

        let file = root.join(REGISTRY_FILE);
        assert!(register_at(Some(&file), &project_dir).unwrap());
        // Re-register the same root: kept, no duplicate, reports false.
        assert!(!register_at(Some(&file), &project_dir).unwrap());
        // Symlink alias resolves to the same canonical entry.
        assert!(!register_at(Some(&file), &alias).unwrap());

        let registry = load_at(Some(&file));
        assert_eq!(registry.projects, vec![project_dir.canonicalize().unwrap()]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn register_requires_existing_project_dir() {
        let root = tmp_root("reject");
        let file = root.join(REGISTRY_FILE);

        // Not a directory at all.
        let err = register_at(Some(&file), &root.join("missing")).unwrap_err();
        assert_eq!(err.code(), "NOT_A_PROJECT");

        // Directory exists but has no project.json.
        std::fs::create_dir_all(root.join("plain")).unwrap();
        let err = register_at(Some(&file), &root.join("plain")).unwrap_err();
        assert_eq!(err.code(), "NOT_A_PROJECT");

        assert!(load_at(Some(&file)).projects.is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unregister_is_idempotent_and_matches_deleted_dirs() {
        let root = tmp_root("unregister");
        let project_dir = root.join("gone");
        sample_project(&project_dir);
        let file = root.join(REGISTRY_FILE);

        assert!(register_at(Some(&file), &project_dir).unwrap());
        // Delete the directory first, then unregister by the stale path.
        std::fs::remove_dir_all(&project_dir).unwrap();
        assert!(unregister_at(Some(&file), &project_dir).unwrap());
        assert!(!unregister_at(Some(&file), &project_dir).unwrap());
        assert!(load_at(Some(&file)).projects.is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_valid_filters_missing_and_prune_persists() {
        let root = tmp_root("selfheal");
        let kept = root.join("kept");
        let dropped = root.join("dropped");
        sample_project(&kept);
        sample_project(&dropped);
        let file = root.join(REGISTRY_FILE);

        register_at(Some(&file), &kept).unwrap();
        register_at(Some(&file), &dropped).unwrap();
        std::fs::remove_dir_all(&dropped).unwrap();

        let valid = list_valid_at(Some(&file)).unwrap();
        assert_eq!(valid, vec![kept.canonicalize().unwrap()]);

        // prune persists the cleanup.
        assert_eq!(prune_at(Some(&file)).unwrap(), 1);
        assert_eq!(prune_at(Some(&file)).unwrap(), 0, "second prune is a no-op");
        assert_eq!(load_at(Some(&file)).projects, vec![kept.canonicalize().unwrap()]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn corrupt_registry_file_degrades_to_default() {
        let root = tmp_root("corrupt");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join(REGISTRY_FILE);
        std::fs::write(&file, b"{not json").unwrap();
        assert!(load_at(Some(&file)).projects.is_empty());
        // list_valid over a corrupt file is empty, not an error.
        assert!(list_valid_at(Some(&file)).unwrap().is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_registry_file_is_an_empty_catalog() {
        let root = tmp_root("absent");
        let file = root.join("nested").join(REGISTRY_FILE);
        assert!(load_at(Some(&file)).projects.is_empty());
        assert!(list_valid_at(Some(&file)).unwrap().is_empty());
    }

    #[test]
    fn save_at_roundtrips_and_none_path_errors() {
        let root = tmp_root("save");
        let file = root.join(REGISTRY_FILE);
        let mut registry = ProjectRegistry::default();
        registry.projects.push(root.canonicalize().unwrap_or_else(|_| root.clone()));
        save_at(Some(&file), &registry).unwrap();
        assert_eq!(load_at(Some(&file)), registry);

        let err = save_at(None, &registry).unwrap_err();
        assert_eq!(err.code(), "IO_ERROR");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn projects_root_lives_under_rudder_home() {
        let home = tmp_root("projroot");
        // projects_root_at creates the directory on demand.
        let projects = projects_root_at(&home).unwrap();
        assert_eq!(projects, home.join("Rudder").join("projects"));
        assert!(projects.is_dir());
        // new_project_dir_at hands out unique directories under the root.
        let a = new_project_dir_at(&home).unwrap();
        let b = new_project_dir_at(&home).unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with(&projects) && b.starts_with(&projects));
        std::fs::remove_dir_all(&home).ok();
    }
}
