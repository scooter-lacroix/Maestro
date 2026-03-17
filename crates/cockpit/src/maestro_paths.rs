//! Maestro project path resolution utilities
//!
//! Provides project-aware path resolution for discovering tracks.md,
//! product.md, workflow.md and other Maestro docs regardless of where
//! the Cockpit TUI is launched from.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Potential locations to search for tracks.md relative to a starting directory
const TRACKS_SEARCH_PATHS: &[&str] = &[
    "maestro/tracks.md",        // Standard Maestro project structure
    "tracks.md",                // Legacy or flat structure
    ".maestro/tracks.md",       // Hidden config structure
    "maestro/tracks/tracks.md", // Deep structure
];

/// Maestro project structure after discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaestroProject {
    /// Root directory of the project (parent of maestro/ or where tracks.md lives)
    pub root_dir: PathBuf,
    /// Directory containing tracks.md (the "tracks_dir")
    pub tracks_dir: PathBuf,
    /// Full path to tracks.md
    pub tracks_path: PathBuf,
    /// Optional path to product.md
    pub product_md: Option<PathBuf>,
    /// Optional path to workflow.md
    pub workflow_md: Option<PathBuf>,
}

impl MaestroProject {
    /// Create a new MaestroProject from a discovered tracks.md path
    pub fn from_tracks_path(tracks_path: PathBuf) -> Option<Self> {
        // Absolute path is required for reliable parent/root determination
        let absolute_path = if tracks_path.is_absolute() {
            tracks_path
        } else {
            std::env::current_dir().ok()?.join(tracks_path)
        };

        let tracks_dir = absolute_path.parent()?.to_path_buf();

        // Determine root_dir: if tracks_dir ends with "maestro" or ".maestro", parent is root
        let dir_name = tracks_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let root_dir = if dir_name == "maestro" || dir_name == ".maestro" {
            tracks_dir.parent()?.to_path_buf()
        } else {
            tracks_dir.clone()
        };

        // Look for optional companion files
        let product_md = tracks_dir.join("product.md");
        let workflow_md = tracks_dir.join("workflow.md");

        Some(Self {
            root_dir,
            tracks_dir,
            tracks_path: absolute_path,
            product_md: product_md.exists().then_some(product_md),
            workflow_md: workflow_md.exists().then_some(workflow_md),
        })
    }

    /// Get the project name (last component of root_dir)
    pub fn name(&self) -> String {
        self.root_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }
}

/// Resolve the Maestro tracks directory by searching from the current directory upward.
///
/// This function walks up the directory tree from the starting point, looking for
/// `maestro/tracks.md` or `tracks.md` in each directory. The first match found
/// determines the tracks_dir.
///
/// # Arguments
/// * `start` - Optional starting directory. If None, uses current working directory.
///
/// # Returns
/// * `Some(PathBuf)` - The directory containing tracks.md (the tracks_dir)
/// * `None` - If no tracks.md is found in the ancestor chain
pub fn resolve_tracks_dir(start: Option<&Path>) -> Option<PathBuf> {
    resolve_maestro_project(start).map(|p| p.tracks_dir)
}

/// Resolve the full Maestro project structure by searching from the current directory upward.
///
/// # Arguments
/// * `start` - Optional starting directory. If None, uses current working directory.
///
/// # Returns
/// * `Some(MaestroProject)` - The discovered project structure
/// * `None` - If no tracks.md is found in the ancestor chain
pub fn resolve_maestro_project(start: Option<&Path>) -> Option<MaestroProject> {
    let start_dir = match start {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().ok()?,
    };

    // Walk up the directory tree
    let mut current = start_dir.as_path();

    loop {
        // Try each search path pattern
        for pattern in TRACKS_SEARCH_PATHS {
            let candidate = current.join(pattern);
            if candidate.exists() && candidate.is_file() {
                return MaestroProject::from_tracks_path(candidate);
            }
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

/// Discover Maestro projects from a list of working directories.
///
/// This is useful for discovering projects from tmux session pane working directories.
/// It deduplicates projects by their canonical root path.
///
/// # Arguments
/// * `working_dirs` - List of working directory paths to scan
///
/// # Returns
/// * `Vec<MaestroProject>` - Unique discovered projects
pub fn discover_projects_from_dirs(working_dirs: &[PathBuf]) -> Vec<MaestroProject> {
    use std::collections::HashSet;

    let mut seen_roots: HashSet<PathBuf> = HashSet::new();
    let mut projects = Vec::new();

    for dir in working_dirs {
        if let Some(project) = resolve_maestro_project(Some(dir)) {
            // Canonicalize for deduplication
            let canonical_root = project
                .root_dir
                .canonicalize()
                .unwrap_or_else(|_| project.root_dir.clone());

            if !seen_roots.contains(&canonical_root) {
                seen_roots.insert(canonical_root);
                projects.push(project);
            }
        }
    }

    projects
}

/// Discover all Maestro projects by scanning the current directory and all active tmux panes.
pub fn discover_all_projects() -> Vec<MaestroProject> {
    use leindex_core::multiplexer::TmuxMultiplexer;

    let mut working_dirs = Vec::new();

    // Add current working directory
    if let Ok(cwd) = std::env::current_dir() {
        working_dirs.push(cwd);
    }

    // Add tmux pane paths if available
    let mux = TmuxMultiplexer::new();
    if let Ok(tmux_paths) = mux.get_all_pane_paths() {
        for path in tmux_paths {
            working_dirs.push(PathBuf::from(path));
        }
    }

    discover_projects_from_dirs(&working_dirs)
}

/// Get the default tracks directory, with fallback to current directory.
///
/// This is the main entry point for the Conductor pane initialization.
/// It tries to resolve a proper Maestro project, falling back to "." if not found.
pub fn get_default_tracks_dir() -> PathBuf {
    resolve_tracks_dir(None).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_tracks_dir_maestro_structure() {
        let temp = TempDir::new().unwrap();
        let maestro_dir = temp.path().join("maestro");
        fs::create_dir_all(&maestro_dir).unwrap();
        fs::write(maestro_dir.join("tracks.md"), "# Tracks").unwrap();

        // Search from project root should find maestro/tracks.md
        let result = resolve_tracks_dir(Some(temp.path()));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), maestro_dir);
    }

    #[test]
    fn test_resolve_tracks_dir_legacy_structure() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("tracks.md"), "# Tracks").unwrap();

        let result = resolve_tracks_dir(Some(temp.path()));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp.path().to_path_buf());
    }

    #[test]
    fn test_resolve_from_subdirectory() {
        let temp = TempDir::new().unwrap();
        let maestro_dir = temp.path().join("maestro");
        let sub_dir = temp.path().join("src").join("deep").join("nested");
        fs::create_dir_all(&maestro_dir).unwrap();
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(maestro_dir.join("tracks.md"), "# Tracks").unwrap();

        // Search from deep subdirectory should find maestro/tracks.md
        let result = resolve_tracks_dir(Some(&sub_dir));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), maestro_dir);
    }

    #[test]
    fn test_no_tracks_found() {
        let temp = TempDir::new().unwrap();
        let result = resolve_tracks_dir(Some(temp.path()));
        assert!(result.is_none());
    }

    #[test]
    fn test_maestro_project_structure() {
        let temp = TempDir::new().unwrap();
        let maestro_dir = temp.path().join("maestro");
        fs::create_dir_all(&maestro_dir).unwrap();
        fs::write(maestro_dir.join("tracks.md"), "# Tracks").unwrap();
        fs::write(maestro_dir.join("product.md"), "# Product").unwrap();

        let project = resolve_maestro_project(Some(temp.path())).unwrap();
        assert_eq!(project.root_dir, temp.path().to_path_buf());
        assert_eq!(project.tracks_dir, maestro_dir);
        assert!(project.product_md.is_some());
        assert!(project.workflow_md.is_none());
    }
}
