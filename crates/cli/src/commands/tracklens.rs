//! TrackLens Command - Review and walkthrough for Maestro tracks
//!
//! This module provides CLI commands for TrackLens review functionality:
//! - Review documents (specs, plans, walkthroughs)
//! - Generate and review track walkthroughs
//! - Code review mode (git diff)

use anyhow::{anyhow, Result};
use leindex_core::tracklens::{
    DecisionBehavior, ReviewContent, ReviewMetadata, ReviewMode, ServerConfig, TrackLensDecision,
    TrackLensServer, WalkthroughConfig, WalkthroughGenerator,
};
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

/// TrackLens subcommands
#[derive(clap::Subcommand, Debug, Clone)]
pub enum TrackLensCommands {
    /// Review a document (spec, plan, or walkthrough)
    Review {
        /// File to review
        #[arg(short, long)]
        file: PathBuf,

        /// Review mode: review, code-review, annotate
        #[arg(short, long, default_value = "review")]
        mode: String,

        /// Do not open browser automatically
        #[arg(long)]
        no_browser: bool,
    },

    /// Generate and review walkthrough for completed track
    Walkthrough {
        /// Track ID
        track_id: String,

        /// Include full diffs
        #[arg(long)]
        full_diffs: bool,

        /// Do not open browser automatically
        #[arg(long)]
        no_browser: bool,
    },

    /// Code review mode (git diff)
    CodeReview {
        /// Git commit, range, or ref
        #[arg(default_value = "HEAD")]
        commit: String,

        /// Do not open browser automatically
        #[arg(long)]
        no_browser: bool,
    },
}

/// Run TrackLens command
pub async fn run(command: TrackLensCommands) -> Result<()> {
    match command {
        TrackLensCommands::Review {
            file,
            mode,
            no_browser,
        } => run_review(file, mode, !no_browser).await,
        TrackLensCommands::Walkthrough {
            track_id,
            full_diffs,
            no_browser,
        } => run_walkthrough(track_id, full_diffs, !no_browser).await,
        TrackLensCommands::CodeReview { commit, no_browser } => {
            run_code_review(commit, !no_browser).await
        }
    }
}

/// Validate a track ID to prevent path traversal attacks
/// Only allows alphanumeric characters, dashes, underscores
fn validate_track_id(track_id: &str) -> Result<()> {
    // Reject empty track IDs
    if track_id.is_empty() {
        anyhow::bail!("Track ID cannot be empty");
    }

    // Reject track IDs containing path separators
    if track_id.contains('/') || track_id.contains('\\') {
        anyhow::bail!("Track ID cannot contain path separators");
    }

    // Reject path traversal attempts
    if track_id.contains("..") {
        anyhow::bail!("Track ID cannot contain '..' (path traversal not allowed)");
    }

    // Reject absolute paths
    if track_id.starts_with('/') || track_id.starts_with('\\') {
        anyhow::bail!("Track ID cannot be an absolute path");
    }

    // Only allow safe characters: alphanumeric, dash, underscore
    let safe_chars = track_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_');

    if !safe_chars {
        anyhow::bail!(
            "Track ID contains invalid characters. Only alphanumeric, '-', and '_' are allowed"
        );
    }

    Ok(())
}

/// Review a document file
async fn run_review(file: PathBuf, mode: String, browser: bool) -> Result<()> {
    info!("═══════════════════════════════════════════════════════════════");
    info!("  TrackLens Review");
    info!("═══════════════════════════════════════════════════════════════");
    info!("File: {}", file.display());
    info!("Mode: {}", mode);
    println!();

    // Parse review mode
    let review_mode = match mode.as_str() {
        "code-review" => ReviewMode::CodeReview,
        "annotate" => ReviewMode::Annotate,
        _ => ReviewMode::Review,
    };

    let read_file_content = || async {
        tokio::fs::read_to_string(&file)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file.display(), e))
    };

    // Get content based on mode
    let content = if review_mode == ReviewMode::CodeReview {
        // For code review mode, generate a git diff for the file
        let file_str = file.to_str().ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
        let output = tokio::process::Command::new("git")
            .args(["diff", "--", file_str])
            .output()
            .await?;

        if output.status.success() {
            let diff = String::from_utf8_lossy(&output.stdout).to_string();
            if diff.trim().is_empty() {
                // If no diff, try staged changes
                let output = tokio::process::Command::new("git")
                    .args(["diff", "--staged", "--", file_str])
                    .output()
                    .await?;
                let staged_diff = String::from_utf8_lossy(&output.stdout).to_string();
                if staged_diff.trim().is_empty() {
                    read_file_content().await?
                } else {
                    staged_diff
                }
            } else {
                diff
            }
        } else {
            // If git diff fails, read the file content directly
            read_file_content().await?
        }
    } else {
        // For other modes, read the file content directly
        read_file_content().await?
    };

    // Start TrackLens server
    let config = ServerConfig {
        port: 0, // Random port
        host: "127.0.0.1".to_string(),
        open_browser: browser,
    };
    let server = TrackLensServer::new(config);
    let url = server.start().await?;

    info!("Review server running at: {}", url);
    info!("Open this URL in your browser to review the document.");
    println!();

    // Set review content
    let review_content = ReviewContent {
        mode: review_mode,
        content,
        metadata: ReviewMetadata {
            track_id: None,
            document_type: "markdown".to_string(),
            origin: "cli".to_string(),
        },
    };
    server.set_content(review_content)?;

    wait_for_tracklens_ready(&server).await?;

    // Wait for decision
    info!("Waiting for review decision...");
    let decision = server.wait_for_decision().await?;

    // Output decision
    print_decision(&decision);

    // Return error on deny so agents can detect rejection via exit code
    match decision.behavior {
        DecisionBehavior::Allow => Ok(()),
        DecisionBehavior::Deny => Err(anyhow!(
            "Review denied. See annotations above for required changes."
        )),
    }
}

/// Generate and review a track walkthrough
async fn run_walkthrough(track_id: String, full_diffs: bool, browser: bool) -> Result<()> {
    // Validate track ID to prevent path traversal
    validate_track_id(&track_id)?;

    info!("═══════════════════════════════════════════════════════════════");
    info!("  TrackLens Walkthrough");
    info!("═══════════════════════════════════════════════════════════════");
    info!("Track: {}", track_id);
    println!();

    // Load track spec and plan
    let tracks_dir = std::path::PathBuf::from("./maestro/tracks");
    let track_path = tracks_dir.join(&track_id);

    let spec_path = track_path.join("spec.md");
    let plan_path = track_path.join("plan.md");

    if !spec_path.exists() {
        anyhow::bail!("Spec file not found: {}", spec_path.display());
    }
    if !plan_path.exists() {
        anyhow::bail!("Plan file not found: {}", plan_path.display());
    }

    let spec = tokio::fs::read_to_string(&spec_path).await?;
    let plan = tokio::fs::read_to_string(&plan_path).await?;

    // Generate walkthrough
    let generator = WalkthroughGenerator::new(
        std::path::Path::new("."),
        WalkthroughConfig {
            include_snippets: true,
            include_diffs: full_diffs,
            max_snippet_lines: 30,
        },
    );

    let walkthrough = generator.generate(&track_id, &spec, &plan)?;
    let markdown = generator.to_markdown(&walkthrough);

    info!(
        "Walkthrough generated: {} tasks, {} files changed",
        walkthrough.completed_tasks.len(),
        walkthrough.files.len()
    );
    println!();

    // Start server and present
    let config = ServerConfig {
        port: 0,
        host: "127.0.0.1".to_string(),
        open_browser: browser,
    };
    let server = TrackLensServer::new(config);
    let url = server.start().await?;

    info!("Walkthrough review server running at: {}", url);
    info!("Open this URL in your browser to review the walkthrough.");
    println!();

    let review_content = ReviewContent {
        mode: ReviewMode::Review,
        content: markdown,
        metadata: ReviewMetadata {
            track_id: Some(track_id.clone()),
            document_type: "walkthrough".to_string(),
            origin: "cli".to_string(),
        },
    };
    server.set_content(review_content)?;

    wait_for_tracklens_ready(&server).await?;

    // Wait for decision
    info!("Waiting for walkthrough review decision...");
    let decision = server.wait_for_decision().await?;

    // Output decision
    print_decision(&decision);

    // Handle decision
    match decision.behavior {
        DecisionBehavior::Allow => {
            // Save walkthrough-final.md (within validated track_path)
            let output_path = track_path.join("walkthrough-final.md");

            // Security check: canonicalize parent directory (must exist)
            let parent = output_path.parent().unwrap();
            std::fs::create_dir_all(parent)?;
            let canonical_parent = parent.canonicalize()?;
            let canonical_tracks = tracks_dir.canonicalize()?;
            if !canonical_parent.starts_with(&canonical_tracks) {
                anyhow::bail!("Security error: Output path escapes tracks directory");
            }

            let content = server
                .state
                .content
                .read()
                .map_err(|e| anyhow!("Failed to read content: {}", e))?;
            if let Some(ref c) = *content {
                tokio::fs::write(&output_path, &c.content).await?;
                info!("Walkthrough saved to: {}", output_path.display());
            }
        }
        DecisionBehavior::Deny => {
            // Create remediation tasks from annotations
            if let Some(annotations) = decision.annotations {
                if !annotations.is_empty() {
                    info!("");
                    info!("Remediation tasks required:");
                    for (i, annotation) in annotations.iter().enumerate() {
                        info!("  {}. {}", i + 1, annotation.content.comment);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Code review mode using git diff
async fn run_code_review(commit: String, browser: bool) -> Result<()> {
    info!("═══════════════════════════════════════════════════════════════");
    info!("  TrackLens Code Review");
    info!("═══════════════════════════════════════════════════════════════");
    info!("Commit/Ref: {}", commit);
    println!();

    // Get git diff
    let output = tokio::process::Command::new("git")
        .args(["diff", &commit])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git diff failed: {}", stderr);
    }

    let diff = String::from_utf8_lossy(&output.stdout).to_string();

    if diff.trim().is_empty() {
        info!("No changes found in diff.");
        return Ok(());
    }

    // Start server
    let config = ServerConfig {
        port: 0,
        host: "127.0.0.1".to_string(),
        open_browser: browser,
    };
    let server = TrackLensServer::new(config);
    let url = server.start().await?;

    info!("Code review server running at: {}", url);
    info!("Open this URL in your browser to review the changes.");
    println!();

    let review_content = ReviewContent {
        mode: ReviewMode::CodeReview,
        content: diff,
        metadata: ReviewMetadata {
            track_id: None,
            document_type: "git-diff".to_string(),
            origin: "cli".to_string(),
        },
    };
    server.set_content(review_content)?;

    wait_for_tracklens_ready(&server).await?;

    // Wait for decision
    info!("Waiting for code review decision...");
    let decision = server.wait_for_decision().await?;

    // Output decision
    print_decision(&decision);

    // Return error on deny so agents can detect rejection via exit code
    match decision.behavior {
        DecisionBehavior::Allow => Ok(()),
        DecisionBehavior::Deny => Err(anyhow!(
            "Code review denied. See annotations above for required changes."
        )),
    }
}

/// Print review decision to console
fn print_decision(decision: &TrackLensDecision) {
    info!("");
    info!("═══════════════════════════════════════════════════════════════");
    info!("  Review Decision");
    info!("═══════════════════════════════════════════════════════════════");
    info!("");

    match decision.behavior {
        DecisionBehavior::Allow => {
            info!("Decision: ✓ APPROVED");
        }
        DecisionBehavior::Deny => {
            info!("Decision: ✗ DENIED");
        }
    }

    if let Some(ref annotations) = decision.annotations {
        if !annotations.is_empty() {
            info!("");
            info!("Annotations ({})", annotations.len());
            for (i, annotation) in annotations.iter().enumerate() {
                let severity = match annotation.content.severity {
                    leindex_core::tracklens::AnnotationSeverity::Info => "INFO",
                    leindex_core::tracklens::AnnotationSeverity::Warning => "WARN",
                    leindex_core::tracklens::AnnotationSeverity::Error => "ERROR",
                };
                info!("  {}. [{}] {}", i + 1, severity, annotation.content.comment);
            }
        }
    }

    info!("");
}

async fn wait_for_tracklens_ready(server: &TrackLensServer) -> Result<()> {
    let timeout_ms = std::env::var("TRACKLENS_CLIENT_READY_TIMEOUT_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|timeout| *timeout > 0)
        .unwrap_or(20_000);
    info!(
        "Waiting for TrackLens UI readiness (timeout: {}ms)...",
        timeout_ms
    );
    server
        .wait_for_client_ready(Duration::from_millis(timeout_ms))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracklens_commands_debug() {
        let cmd = TrackLensCommands::Review {
            file: PathBuf::from("test.md"),
            mode: "review".to_string(),
            no_browser: false,
        };
        assert!(format!("{:?}", cmd).contains("Review"));
    }

    #[test]
    fn test_tracklents_walkthrough_command() {
        let cmd = TrackLensCommands::Walkthrough {
            track_id: "test-track".to_string(),
            full_diffs: true,
            no_browser: true,
        };
        assert!(format!("{:?}", cmd).contains("Walkthrough"));
    }

    #[test]
    fn test_tracklens_code_review_command() {
        let cmd = TrackLensCommands::CodeReview {
            commit: "HEAD~1".to_string(),
            no_browser: false,
        };
        assert!(format!("{:?}", cmd).contains("CodeReview"));
    }
}
