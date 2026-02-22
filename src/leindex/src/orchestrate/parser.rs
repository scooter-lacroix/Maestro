//! Parser for Maestro tracks.md and plan.md files
//!
//! Lossless parsing that preserves original formatting.

use crate::orchestrate::model::*;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Parse tracks.md to extract all tracks
pub fn parse_tracks_md<P: AsRef<Path>>(path: P) -> Result<Vec<Track>> {
    let content = fs::read_to_string(path.as_ref())?;
    parse_tracks_content(&content, path.as_ref())
}

fn parse_tracks_content(content: &str, base_path: &Path) -> Result<Vec<Track>> {
    let mut tracks = Vec::new();

    // Split by track separator (---)
    let sections: Vec<&str> = content.split("\n---\n").collect();

    for section in sections {
        if let Some(track) = parse_track_section(section, base_path)? {
            tracks.push(track);
        }
    }

    Ok(tracks)
}

fn parse_track_section(section: &str, base_path: &Path) -> Result<Option<Track>> {
    let lines: Vec<&str> = section.lines().collect();
    if lines.is_empty() {
        return Ok(None);
    }

    // First line should be the track status + description
    let first_line = lines[0].trim();

    // Skip if not a track line (starts with "##")
    if !first_line.starts_with("## [") {
        return Ok(None);
    }

    // Parse status marker
    let status = if first_line.contains("[x]") {
        TrackStatus::Completed
    } else if first_line.contains("[~]") {
        TrackStatus::InProgress
    } else {
        TrackStatus::Pending
    };

    // Extract track ID from link line
    let mut link_path = None;
    let mut description = String::new();
    let mut track_id = String::new();

    // Get the parent directory of the tracks.md file, not the file itself
    let tracks_parent = base_path.parent().unwrap_or(base_path);

    for line in &lines {
        if line.contains("*Link:") {
            // Extract path from [./path/](./path/)
            if let Some(start) = line.find("](") {
                if let Some(end) = line[start..].find(')') {
                    let path_str = &line[start + 2..start + end];
                    // Join against parent directory of tracks.md, not the file itself
                    let relative_path = path_str.trim_start_matches("./").trim_start_matches(".");

                    // Intelligent path resolution:
                    // If the path starts with the same directory name as tracks_parent,
                    // it's likely relative to the project root, not the maestro/ directory.
                    let resolved_path = if let Some(dir_name) = tracks_parent.file_name() {
                        if relative_path.starts_with(dir_name.to_str().unwrap_or("")) {
                            // Relative to project root (parent of tracks_parent)
                            tracks_parent
                                .parent()
                                .unwrap_or(tracks_parent)
                                .join(relative_path)
                        } else {
                            // Relative to tracks_parent (maestro/ directory)
                            tracks_parent.join(relative_path)
                        }
                    } else {
                        tracks_parent.join(relative_path)
                    };

                    // SECURITY: Canonicalize and validate path to prevent traversal attacks
                    // If canonicalize fails (file doesn't exist), we still want to keep it
                    // but we should make it absolute if possible to make starts_with reliable.
                    let canonical_path = resolved_path.canonicalize().unwrap_or_else(|_| {
                        if resolved_path.is_relative() {
                            std::env::current_dir()
                                .unwrap_or_default()
                                .join(&resolved_path)
                        } else {
                            resolved_path.clone()
                        }
                    });

                    let canonical_base = tracks_parent.canonicalize().unwrap_or_else(|_| {
                        if tracks_parent.is_relative() {
                            std::env::current_dir()
                                .unwrap_or_default()
                                .join(tracks_parent)
                        } else {
                            tracks_parent.to_path_buf()
                        }
                    });

                    // For the check, we use the root (parent of maestro/ if named maestro)
                    let check_base_owned = if canonical_base
                        .file_name()
                        .map_or(false, |n| n == "maestro" || n == ".maestro")
                    {
                        canonical_base
                            .parent()
                            .unwrap_or(&canonical_base)
                            .to_path_buf()
                    } else {
                        canonical_base.clone()
                    };

                    // Verify the resolved path is within the project root
                    if !canonical_path.starts_with(&check_base_owned) {
                        tracing::warn!(
                            "Track path {} resolves outside project root ({:?}), rejecting",
                            path_str,
                            check_base_owned
                        );
                        continue;
                    }

                    link_path = Some(canonical_path);

                    // Extract track ID from path
                    if let Some(folder_name) = path_str.trim_end_matches('/').rsplit('/').next() {
                        track_id = folder_name.to_string();
                    }
                }
            }
        }
        if line.contains("**Description**:") {
            if let Some(desc_start) = line.find(":") {
                description = line[desc_start + 1..].trim().to_string();
            }
        }
    }

    if link_path.is_none() {
        return Ok(None);
    }

    Ok(Some(Track {
        id: track_id,
        description,
        status,
        link_path: link_path.unwrap(),
        metadata: None,
        plan: None,
    }))
}

/// Parse track metadata.json
pub fn parse_metadata<P: AsRef<Path>>(metadata_path: P) -> Result<TrackMetadata> {
    let content = fs::read_to_string(metadata_path.as_ref())?;
    let metadata: TrackMetadata = serde_json::from_str(&content)?;
    Ok(metadata)
}

/// Parse plan.md into a TrackPlan
pub fn parse_plan_md<P: AsRef<Path>>(plan_path: P) -> Result<TrackPlan> {
    let content = fs::read_to_string(plan_path.as_ref())?;
    parse_plan_content(&content, &plan_path.as_ref())
}

fn parse_plan_content(content: &str, path: &Path) -> Result<TrackPlan> {
    let mut tasks = Vec::new();
    let mut phases = Vec::new();
    let mut current_phase_tasks = Vec::new();
    let mut in_phase = false;
    let mut current_phase_name = String::new();

    // Extract track_id from path
    let track_id = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let lines: Vec<&str> = content.lines().collect();
    let mut line_number = 0;

    while line_number < lines.len() {
        let line = lines[line_number];
        let trimmed = line.trim();

        // Detect phase headers
        if trimmed.starts_with("## Phase ") {
            if in_phase && !current_phase_tasks.is_empty() {
                phases.push(Phase {
                    name: current_phase_name.clone(),
                    tasks: current_phase_tasks.clone(),
                });
                current_phase_tasks.clear();
            }

            in_phase = true;
            current_phase_name = trimmed.strip_prefix("## ").unwrap_or(trimmed).to_string();
            line_number += 1;
            continue;
        }

        // Detect task list items
        if trimmed.starts_with("### [") || trimmed.starts_with("- [") {
            let original_line = line_number;
            if let Some(task) = parse_task_line(line, &lines, &mut line_number) {
                if in_phase {
                    current_phase_tasks.push(task.id.clone());
                }
                tasks.push(task);
            }
            // parse_task_line updates line_number to point to the last line it processed
            // If no task was found, increment to continue
            if original_line == line_number {
                line_number += 1;
            }
            continue;
        }

        line_number += 1;
    }

    // Don't forget the last phase
    if in_phase && !current_phase_tasks.is_empty() {
        phases.push(Phase {
            name: current_phase_name,
            tasks: current_phase_tasks,
        });
    }

    Ok(TrackPlan {
        track_id,
        tasks,
        phases,
    })
}

fn parse_task_line(line: &str, all_lines: &[&str], line_num: &mut usize) -> Option<Task> {
    let trimmed = line.trim();

    // Parse status marker
    let status = if trimmed.contains("[x]") {
        TrackStatus::Completed
    } else if trimmed.contains("[~]") {
        TrackStatus::InProgress
    } else {
        TrackStatus::Pending
    };

    // Extract task ID from the task header format
    // Format: "### [ ] Task 1.1: Title"
    // or: "- [ ] Task 1.1: Title"
    let title = trimmed
        .split(']')
        .last()?
        .trim()
        .trim_start_matches("Task ")
        .trim_start_matches("task ")
        .to_string();

    // Generate ID from title using the same normalization as dependencies
    // This ensures task IDs match dependency references
    let normalized = title
        .split(':')
        .next()
        .unwrap_or(&title)
        .trim()
        .trim_start_matches("Task ")
        .trim_start_matches("task ")
        .to_lowercase();
    let id = normalized
        .replace(' ', "-")
        .replace('.', "-")
        .replace(':', "-")
        .replace('_', "-")
        .replace('/', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Look ahead for subtasks and dependencies
    let mut subtasks = Vec::new();
    let mut dependencies = Vec::new();
    let mut description = String::new();
    let original_line = *line_num;

    // Look ahead a few lines for subtasks and details
    let lookahead_start = *line_num + 1;
    let mut i = lookahead_start;
    let indent_level = line.len() - line.trim_start().len();
    let max_lookahead = all_lines.len().min(lookahead_start + 50);

    while i < max_lookahead {
        let next_line = all_lines[i];
        let next_trimmed = next_line.trim();

        // Stop if we hit another task at same or lower indent
        if (next_trimmed.starts_with("### [") || next_trimmed.starts_with("- ["))
            && next_line.len() - next_line.trim_start().len() <= indent_level
        {
            break;
        }

        // Check for subtasks (more indented)
        if next_trimmed.starts_with("- [")
            && next_line.len() - next_line.trim_start().len() > indent_level
        {
            // Parse subtask recursively
            *line_num = i;
            if let Some(subtask) = parse_task_line(next_line, all_lines, line_num) {
                subtasks.push(subtask);
            }
        }

        // Check for dependencies
        if next_trimmed.starts_with("**Dependencies**:") {
            // Parse dependencies from following lines
            i += 1;
            while i < all_lines.len() {
                let dep_line = all_lines[i].trim();
                if dep_line.is_empty() || !dep_line.starts_with('-') {
                    break;
                }
                // Extract task ID from dependency and normalize to match task ID format
                // Task IDs are normalized to: "task-" + lowercase alphanumeric + hyphens only
                let dep_id = dep_line
                    .trim_start_matches('-')
                    .trim()
                    .trim_start_matches("Task ")
                    .trim_start_matches("task ")
                    .to_lowercase()
                    .replace([' ', '.', ':', ':', '_'], "-")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .collect::<String>();
                // Add "task-" prefix to match task ID format
                let normalized_id = if dep_id.starts_with("task-") {
                    dep_id
                } else {
                    format!("task-{}", dep_id)
                };
                dependencies.push(TaskDependency {
                    task_id: normalized_id,
                    dependency_type: TaskDependencyType::Hard,
                });
                i += 1;
            }
            continue;
        }

        // Collect description lines
        if !next_trimmed.starts_with("**") && !next_trimmed.is_empty() {
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(next_trimmed);
        }

        i += 1;
    }

    // Update line_num to the last line we processed (or -1 if we found no subtasks/dependencies)
    // This ensures the main parser knows where to continue
    if i > lookahead_start {
        *line_num = i - 1;
    }

    Some(Task {
        id: format!("task-{}", id),
        title,
        status,
        dependencies,
        description,
        subtasks,
        notes: None,
        line_number: original_line + 1,
    })
}

/// Write updated plan.md preserving formatting
pub fn write_plan_md<P: AsRef<Path>>(plan: &TrackPlan, path: P) -> Result<()> {
    let original = fs::read_to_string(path.as_ref())?;
    let mut lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();

    // Update task status markers in place
    update_task_status_in_lines(&mut lines, &plan.tasks, 0)?;

    let updated = lines.join("\n");
    fs::write(path.as_ref(), updated)?;

    Ok(())
}

fn update_task_status_in_lines(
    lines: &mut [String],
    tasks: &[Task],
    base_indent: usize,
) -> Result<()> {
    for task in tasks {
        // Find the task line using stricter matching:
        // - Must match the task line number (stored in task.line_number)
        // - Must contain the status marker pattern
        if task.line_number > 0 && task.line_number <= lines.len() {
            let line_idx = task.line_number - 1;
            let line = &mut lines[line_idx];
            // Verify this is the correct task line
            if line.contains('[') && line.contains(']') {
                let new_marker = task.status.to_marker();
                // Replace existing marker with new one
                *line = line
                    .replacen("[ ]", new_marker, 1)
                    .replacen("[~]", new_marker, 1)
                    .replacen("[x]", new_marker, 1);
            }
        }

        // Recursively update subtasks
        if !task.subtasks.is_empty() {
            update_task_status_in_lines(lines, &task.subtasks, base_indent + 2)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_marker() {
        assert_eq!(TrackStatus::from_marker("[ ]"), Some(TrackStatus::Pending));
        assert_eq!(
            TrackStatus::from_marker("[~]"),
            Some(TrackStatus::InProgress)
        );
        assert_eq!(
            TrackStatus::from_marker("[x]"),
            Some(TrackStatus::Completed)
        );
        assert_eq!(TrackStatus::from_marker("[?]"), None);
    }

    #[test]
    fn test_status_to_marker() {
        assert_eq!(TrackStatus::Pending.to_marker(), "[ ]");
        assert_eq!(TrackStatus::InProgress.to_marker(), "[~]");
        assert_eq!(TrackStatus::Completed.to_marker(), "[x]");
    }
}
