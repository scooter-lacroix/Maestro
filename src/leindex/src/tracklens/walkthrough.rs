// TrackLens Walkthrough Generator
//
// This module provides walkthrough generation for completed tracks:
// - Extract completed tasks from plan.md
// - Get changed files from git history
// - Generate spec summary
// - Create walkthrough markdown with diffs and snippets

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

// ─── Walkthrough Configuration ────────────────────────────────────────────────

/// Walkthrough generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkthroughConfig {
    /// Include file snippets in walkthrough
    pub include_snippets: bool,
    /// Include full diffs in walkthrough
    pub include_diffs: bool,
    /// Max snippet lines per file
    pub max_snippet_lines: usize,
}

impl Default for WalkthroughConfig {
    fn default() -> Self {
        Self {
            include_snippets: true,
            include_diffs: false,
            max_snippet_lines: 30,
        }
    }
}

// ─── Walkthrough Content ──────────────────────────────────────────────────────

/// Generated walkthrough content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Walkthrough {
    /// Track ID
    pub track_id: String,
    /// Track metadata
    pub metadata: TrackMetadata,
    /// Completed tasks
    pub completed_tasks: Vec<String>,
    /// Changed files
    pub files: Vec<ChangedFile>,
    /// Spec summary
    pub spec_summary: String,
}

/// Track metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    /// Track title
    pub title: String,
    /// Track description
    pub description: String,
    /// Completion timestamp
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Is this a subtrack?
    pub is_subtrack: bool,
    /// Parent track ID (if subtrack)
    pub parent_track_id: Option<String>,
}

/// Changed file in walkthrough
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    /// File path
    pub path: String,
    /// Change status
    pub status: FileChangeStatus,
    /// Programming language
    pub language: String,
    /// Number of lines added
    pub additions: u32,
    /// Number of lines deleted
    pub deletions: u32,
    /// Optional diff content
    pub diff: Option<String>,
    /// Optional code snippet
    pub snippet: Option<String>,
}

/// File change status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

// ─── Walkthrough Generator ────────────────────────────────────────────────────

/// Walkthrough generator
pub struct WalkthroughGenerator {
    /// Root directory for git operations
    root: std::path::PathBuf,
    /// Generator configuration
    config: WalkthroughConfig,
}

impl WalkthroughGenerator {
    /// Create a new walkthrough generator
    pub fn new(root: &Path, config: WalkthroughConfig) -> Self {
        Self {
            root: root.to_path_buf(),
            config,
        }
    }

    /// Generate walkthrough for a track
    pub fn generate(
        &self,
        track_id: &str,
        spec: &str,
        plan: &str,
    ) -> anyhow::Result<Walkthrough> {
        // Extract completed tasks
        let completed_tasks = self.extract_completed_tasks(plan)?;

        // Get changed files
        let files = self.get_changed_files(track_id)?;

        // Generate spec summary
        let spec_summary = self.extract_spec_summary(spec);

        // Parse metadata
        let metadata = self.parse_metadata(spec, track_id)?;

        Ok(Walkthrough {
            track_id: track_id.to_string(),
            metadata,
            completed_tasks,
            files,
            spec_summary,
        })
    }

    /// Convert walkthrough to markdown
    pub fn to_markdown(&self, walkthrough: &Walkthrough) -> String {
        let mut doc = format!("# Track Walkthrough: {}\n\n", walkthrough.metadata.title);

        // Metadata
        doc.push_str(&format!("**Track ID**: {}\n", walkthrough.track_id));
        doc.push_str(&format!("**Completed**: {}\n\n", walkthrough.metadata.completed_at));

        // Spec summary
        doc.push_str("## Spec Summary\n\n");
        doc.push_str(&walkthrough.spec_summary);
        doc.push_str("\n\n---\n\n");

        // Completed tasks
        doc.push_str("## Completed Tasks\n\n");
        for task in &walkthrough.completed_tasks {
            doc.push_str(&format!("- [x] {}\n", task));
        }
        doc.push_str("\n---\n\n");

        // Changed files
        doc.push_str("## Files Changed\n\n");
        doc.push_str("| Status | File | +/- |\n");
        doc.push_str("|--------|------|-----|\n");
        for file in &walkthrough.files {
            let icon = self.status_icon(file.status);
            doc.push_str(&format!(
                "| {} | [`{}`]({}) | +{} / -{} |\n",
                icon, file.path, file.path, file.additions, file.deletions
            ));
        }

        // Detailed changes
        if self.config.include_snippets || self.config.include_diffs {
            doc.push_str("\n## Detailed Changes\n\n");
            for file in &walkthrough.files {
                doc.push_str(&format!("### {}\n\n", file.path));

                if self.config.include_snippets {
                    if let Some(ref snippet) = file.snippet {
                        doc.push_str(&format!("```{}\n{}\n```\n\n", file.language, snippet));
                    }
                }

                if self.config.include_diffs {
                    if let Some(ref diff) = file.diff {
                        doc.push_str(&format!(
                            "<details><summary>Full diff ({} lines)</summary>\n\n```diff\n{}\n```\n</details>\n\n",
                            diff.lines().count(),
                            diff
                        ));
                    }
                }
            }
        }

        doc.push_str("\n---\n\n");
        doc.push_str("> Review this walkthrough. Annotate any issues for remediation.\n");

        doc
    }

    // ─── Private Helpers ──────────────────────────────────────────────────────

    fn extract_completed_tasks(&self, plan: &str) -> anyhow::Result<Vec<String>> {
        let tasks = plan
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                    Some(
                        trimmed
                            .trim_start_matches("- [x]")
                            .trim_start_matches("- [X]")
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .collect();
        Ok(tasks)
    }

    fn extract_spec_summary(&self, spec: &str) -> String {
        spec.lines()
            .skip_while(|l| l.trim().is_empty() || l.starts_with('#'))
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn get_changed_files(&self, track_id: &str) -> anyhow::Result<Vec<ChangedFile>> {
        let output = Command::new("git")
            .args([
                "log",
                "--all",
                "--oneline",
                "--grep",
                track_id,
                "--name-status",
                "--diff-filter=ADMR",
            ])
            .current_dir(&self.root)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut files: Vec<ChangedFile> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 && !seen.contains(parts[1]) {
                let status = match parts[0].chars().next() {
                    Some('A') => FileChangeStatus::Added,
                    Some('M') => FileChangeStatus::Modified,
                    Some('D') => FileChangeStatus::Deleted,
                    Some('R') => FileChangeStatus::Renamed,
                    _ => continue,
                };
                let path = parts[1].to_string();
                let language = self.detect_language(&path);

                let (additions, deletions, diff, snippet) =
                    self.get_file_diff_info(&path, track_id);

                seen.insert(path.clone());
                files.push(ChangedFile {
                    path,
                    status,
                    language,
                    diff,
                    snippet,
                    additions,
                    deletions,
                });
            }
        }

        Ok(files)
    }

    fn get_file_diff_info(
        &self,
        file_path: &str,
        track_id: &str,
    ) -> (u32, u32, Option<String>, Option<String>) {
        let diff_output = Command::new("git")
            .args(["log", "--all", "-p", "--grep", track_id, "--", file_path])
            .current_dir(&self.root)
            .output()
            .ok();

        let diff_text = diff_output
            .as_ref()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

        let mut additions = 0u32;
        let mut deletions = 0u32;
        if let Some(ref text) = diff_text {
            for line in text.lines() {
                if line.starts_with('+') && !line.starts_with("+++") {
                    additions += 1;
                }
                if line.starts_with('-') && !line.starts_with("---") {
                    deletions += 1;
                }
            }
        }

        let snippet = std::fs::read_to_string(self.root.join(file_path))
            .ok()
            .map(|content| {
                content
                    .lines()
                    .take(self.config.max_snippet_lines)
                    .collect::<Vec<_>>()
                    .join("\n")
            });

        (additions, deletions, diff_text, snippet)
    }

    fn parse_metadata(&self, spec: &str, track_id: &str) -> anyhow::Result<TrackMetadata> {
        // Simple parsing - in real implementation would be more robust
        let title = spec
            .lines()
            .find(|l| l.starts_with("# "))
            .unwrap_or("# Unknown Track")
            .trim_start_matches("# ")
            .to_string();

        let description = spec
            .lines()
            .skip_while(|l| !l.starts_with("## Description"))
            .skip(1)
            .take_while(|l| !l.starts_with("##"))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(TrackMetadata {
            title,
            description,
            completed_at: chrono::Utc::now(),
            is_subtrack: track_id.contains("subtrack") || track_id.contains("maestroclaw-"),
            parent_track_id: None,
        })
    }

    fn status_icon(&self, status: FileChangeStatus) -> &'static str {
        match status {
            FileChangeStatus::Added => "🆕",
            FileChangeStatus::Modified => "✏️",
            FileChangeStatus::Deleted => "🗑️",
            FileChangeStatus::Renamed => "📝",
        }
    }

    fn detect_language(&self, path: &str) -> String {
        match path.rsplit('.').next() {
            Some("rs") => "rust",
            Some("ts") | Some("tsx") => "typescript",
            Some("js") | Some("jsx") => "javascript",
            Some("py") => "python",
            Some("go") => "go",
            Some("java") => "java",
            Some("c") | Some("h") => "c",
            Some("cpp") | Some("hpp") | Some("cc") => "cpp",
            Some("md") => "markdown",
            Some("json") => "json",
            Some("toml") => "toml",
            Some("yaml") | Some("yml") => "yaml",
            _ => "text",
        }
        .to_string()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_completed_tasks() {
        let plan = "- [x] Task 1\n- [ ] Task 2\n- [X] Task 3\n";
        let generator = WalkthroughGenerator::new(
            Path::new("/tmp"),
            WalkthroughConfig::default(),
        );
        let tasks = generator.extract_completed_tasks(plan).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0], "Task 1");
        assert_eq!(tasks[1], "Task 3");
    }
}
