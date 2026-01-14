//! Project Scanner
//!
//! Parallel filesystem scanner for discovering Maestro projects.
//! Optimized with rayon for concurrent directory traversal.

// anyhow imports cleaned up
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{info, warn};
use walkdir::WalkDir;

use super::models::{ProjectScanInfo, ScanResult, TrackStatus};

/// Directories to skip during scanning
const SKIP_DIRS: &[&str] = &[
    "node_modules", "__pycache__", "venv", "env", ".git", 
    "dist", "build", "target", ".cargo", ".rustup", ".cache",
    ".npm", ".yarn", "vendor", ".venv", "site-packages",
];

/// Project scanner with parallel traversal
#[derive(Clone)]
pub struct Scanner {
    skip_dirs: HashSet<String>,
    track_pattern: Regex,
    _task_pattern: Regex,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            skip_dirs: SKIP_DIRS.iter().map(|s| s.to_string()).collect(),
            track_pattern: Regex::new(r"\[([x ~])\]\s+Track:\s*(.+?)(?:\(([^)]+)\))?$").unwrap(),
            _task_pattern: Regex::new(r"^\s*-\s*\[([x ])\]").unwrap(),
        }
    }

    /// Scan directories for Maestro projects
    pub fn scan(&self, base_dirs: &[PathBuf], max_depth: usize) -> ScanResult {
        let start = Instant::now();
        let projects = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        info!("Scanning {} directories with max_depth={}", base_dirs.len(), max_depth);

        // Process each base directory
        for base_dir in base_dirs {
            if !base_dir.exists() {
                warn!("Directory does not exist: {}", base_dir.display());
                errors.lock().unwrap().push(format!("Directory not found: {}", base_dir.display()));
                continue;
            }

            // Find all potential Maestro projects
            let found = self.find_maestro_projects(base_dir, max_depth);
            
            // Process projects in parallel
            let batch_projects: Vec<ProjectScanInfo> = found
                .par_iter()
                .filter_map(|path| self.parse_project(path))
                .collect();

            projects.lock().unwrap().extend(batch_projects);
        }

        let projects = Arc::try_unwrap(projects).unwrap().into_inner().unwrap();
        let errors = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();

        let track_count: usize = projects.iter().map(|p| p.track_count).sum();

        info!(
            "Scan complete: {} projects, {} tracks in {:?}",
            projects.len(),
            track_count,
            start.elapsed()
        );

        ScanResult {
            projects_found: projects.len(),
            tracks_found: track_count,
            projects,
            errors,
            scan_method: "filesystem".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Find all directories containing Maestro markers
    fn find_maestro_projects(&self, base: &Path, max_depth: usize) -> Vec<PathBuf> {
        let mut projects = Vec::new();

        for entry in WalkDir::new(base)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| self.should_traverse(e))
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if entry.file_type().is_dir() && self.is_maestro_project(path) {
                // Check not nested in existing project
                let is_nested = projects.iter().any(|p: &PathBuf| path.starts_with(p));
                if !is_nested {
                    projects.push(path.to_path_buf());
                }
            }
        }

        projects
    }

    fn should_traverse(&self, entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_string_lossy();
        // Allow hidden directories at depth 0 (the base path itself)
        if entry.depth() == 0 {
            return true;
        }
        if name.starts_with('.') {
            return name == ".maestro"; // Allow .maestro directory
        }
        !self.skip_dirs.contains(name.as_ref())
    }

    /// Check if a directory is a Maestro project
    fn is_maestro_project(&self, path: &Path) -> bool {
        let maestro_dir = path.join("maestro");
        let dot_maestro = path.join(".maestro");

        // .maestro config directory
        if dot_maestro.is_dir() {
            return true;
        }

        // maestro/ with project files
        if maestro_dir.is_dir() {
            if maestro_dir.join("product.md").exists()
                || maestro_dir.join("tracks.md").exists()
                || maestro_dir.join("workflow.md").exists()
                || maestro_dir.join("tracks").is_dir()
            {
                return true;
            }
        }

        // Alternative: product.md + tracks.md at root
        if path.join("product.md").exists() && path.join("tracks.md").exists() {
            return true;
        }

        // tracks directory at root with product.md
        if path.join("tracks").is_dir() && path.join("product.md").exists() {
            return true;
        }

        false
    }

    /// Parse project information from a directory
    fn parse_project(&self, path: &Path) -> Option<ProjectScanInfo> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let maestro_dir = path.join("maestro");
        let dot_maestro = path.join(".maestro");

        let mut project_type = None;
        let mut description = None;

        // Read product.md for description
        let product_file = if maestro_dir.join("product.md").exists() {
            maestro_dir.join("product.md")
        } else {
            path.join("product.md")
        };

        if product_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&product_file) {
                // Extract first non-heading paragraph
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        description = Some(line.chars().take(200).collect::<String>());
                        break;
                    }
                }
                project_type = Some("greenfield".to_string());
            }
        }

        // Read .maestro/config.json
        let config_file = dot_maestro.join("config.json");
        if config_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(t) = config.get("type").and_then(|v| v.as_str()) {
                        project_type = Some(t.to_string());
                    }
                }
            }
        }

        // Determine type if not set
        if project_type.is_none() {
            project_type = if maestro_dir.is_dir() {
                Some("brownfield".to_string())
            } else if dot_maestro.is_dir() {
                Some("generic".to_string())
            } else {
                Some("unknown".to_string())
            };
        }

        // Parse tracks
        let tracks = self.parse_tracks(path);

        Some(ProjectScanInfo {
            path: path.to_string_lossy().to_string(),
            name,
            description,
            project_type,
            track_count: tracks.len(),
        })
    }

    /// Parse tracks from a project directory
    fn parse_tracks(&self, path: &Path) -> Vec<TrackInfo> {
        let mut tracks = Vec::new();

        // Try both maestro/tracks.md and tracks.md
        let tracks_file = if path.join("maestro/tracks.md").exists() {
            path.join("maestro/tracks.md")
        } else {
            path.join("tracks.md")
        };

        if tracks_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&tracks_file) {
                for caps in self.track_pattern.captures_iter(&content) {
                    let status_char = caps.get(1).map(|m| m.as_str()).unwrap_or(" ");
                    let title = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                    let track_id = caps.get(3).map(|m| m.as_str().trim());

                    let status = match status_char {
                        "x" => TrackStatus::Completed,
                        "~" => TrackStatus::InProgress,
                        _ => TrackStatus::New,
                    };

                    tracks.push(TrackInfo {
                        track_id: track_id.map(String::from),
                        _title: title.to_string(),
                        _status: status,
                    });
                }
            }
        }

        // Also check tracks directory
        let tracks_dir = if path.join("maestro/tracks").is_dir() {
            path.join("maestro/tracks")
        } else {
            path.join("tracks")
        };

        if tracks_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&tracks_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let track_path = entry.path();
                    if track_path.extension().map(|e| e == "md").unwrap_or(false) {
                        let track_id = track_path.file_stem()
                            .map(|s| s.to_string_lossy().to_string());
                        
                        if let Some(id) = track_id {
                            // Only add if not already in list
                            if !tracks.iter().any(|t| t.track_id.as_ref() == Some(&id)) {
                                tracks.push(TrackInfo {
                                    track_id: Some(id),
                                    _title: "".to_string(),
                                    _status: TrackStatus::InProgress,
                                });
                            }
                        }
                    }
                }
            }
        }

        tracks
    }
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct TrackInfo {
    track_id: Option<String>,
    _title: String,
    _status: TrackStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_scanner_empty_dir() {
        let dir = tempdir().unwrap();
        let scanner = Scanner::new();
        let result = scanner.scan(&[dir.path().to_path_buf()], 5);
        assert_eq!(result.projects_found, 0);
    }

    #[test]
    fn test_scanner_finds_project() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("myproject");
        let maestro_dir = project_dir.join("maestro");
        fs::create_dir_all(&maestro_dir).unwrap();
        fs::write(maestro_dir.join("product.md"), "# My Project").unwrap();
        
        let scanner = Scanner::new();
        let result = scanner.scan(&[dir.path().to_path_buf()], 5);
        assert_eq!(result.projects_found, 1);
    }
}
