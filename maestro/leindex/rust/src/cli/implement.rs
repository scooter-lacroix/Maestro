//! Track implementation initiation helpers.

use anyhow::{Context, Result};
use clap::ValueEnum;
use std::path::PathBuf;
use std::process::Command;

#[cfg(feature = "rusqlite")]
use crate::memory::{MemoryService, SessionManager};
use crate::multiplexer::TmuxMultiplexer;
use crate::token_format::TokenFormatter;

use super::prompt;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImplementSessionTarget {
    Ask,
    Current,
    New,
}

pub async fn run(
    command: String,
    description: Vec<String>,
    target: ImplementSessionTarget,
    tool: String,
    path: Option<PathBuf>,
    title: Option<String>,
) -> Result<()> {
    let description = description.join(" ").trim().to_string();
    let command = command.trim().to_string();
    if command.is_empty() {
        anyhow::bail!("Implement command cannot be empty");
    }

    let in_tmux = std::env::var("TMUX").is_ok();
    let target = match target {
        ImplementSessionTarget::Ask => {
            if !in_tmux {
                ImplementSessionTarget::New
            } else {
                let choice = prompt::ask_choice(
                    "Implement track in which session?",
                    &[
                        "Current Session (preserve context)",
                        "New Session (clean context)",
                    ],
                )?;
                match choice {
                    Some(0) => ImplementSessionTarget::Current,
                    Some(1) => ImplementSessionTarget::New,
                    _ => return Ok(()),
                }
            }
        }
        other => other,
    };

    match target {
        ImplementSessionTarget::Current => {
            if !in_tmux {
                anyhow::bail!("Not in tmux; cannot target Current Session");
            }
            let target = tmux_current_pane_target()
                .or_else(|| tmux_current_session_target())
                .context("Failed to determine current tmux target")?;

            let mux = TmuxMultiplexer::default();
            mux.send_keys(&target, &command)?;
            mux.send_enter(&target)?;
            if !description.is_empty() {
                mux.send_keys(&target, &description)?;
                mux.send_enter(&target)?;
            }

            println!("Sent track command to current tmux target: {}", target);
        }
        ImplementSessionTarget::New => {
            let cwd = std::env::current_dir().context("Failed to read current directory")?;
            let project_path = path.unwrap_or(cwd);
            let project_path_str = project_path.to_string_lossy().to_string();

            let title = title.unwrap_or_else(|| {
                let formatter = TokenFormatter::new();
                if description.is_empty() {
                    "implement".to_string()
                } else {
                    format!("impl {}", formatter.truncate(&description, 40))
                }
            });

            #[cfg(feature = "rusqlite")]
            {
                let service = MemoryService::new(None).context("Failed to create memory service")?;
                let _ = service.initialize();
                let manager = SessionManager::new(service)?;
                let session = manager.create_session(&title, &project_path_str, &tool, None, None)?;

                // Give the tool a moment to start before sending commands.
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                let mux = TmuxMultiplexer::default();
                mux.send_keys(&session.session_id, &command)?;
                mux.send_enter(&session.session_id)?;
                if !description.is_empty() {
                    mux.send_keys(&session.session_id, &description)?;
                    mux.send_enter(&session.session_id)?;
                }

                println!(
                    "Started new session {} (tool={}) and sent track command.",
                    session.session_id, tool
                );
            }

            #[cfg(not(feature = "rusqlite"))]
            {
                anyhow::bail!("Implement::New requires the 'rusqlite' feature to be enabled. Please rebuild with: --features rusqlite");
            }
        }
        ImplementSessionTarget::Ask => unreachable!("Ask resolved earlier"),
    }

    Ok(())
}

fn tmux_current_pane_target() -> Option<String> {
    tmux_display("#{pane_id}").ok().filter(|s| !s.is_empty())
}

fn tmux_current_session_target() -> Option<String> {
    tmux_display("#{session_name}")
        .ok()
        .filter(|s| !s.is_empty())
}

fn tmux_display(format: &str) -> Result<String> {
    let out = Command::new("tmux")
        .args(["display-message", "-p", format])
        .output()
        .context("Failed to run tmux display-message")?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
