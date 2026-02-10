//! Orchestrate CLI command
//!
//! Ralph-style autonomous task execution loop.

use crate::orchestrate::{
    model::{AgentConfig, LoopMode, OrchestrateConfig},
    parser::{parse_plan_md, parse_tracks_md},
    OrchestrateEngine,
};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum OrchestrateSubcommand {
    /// Start orchestrate loop for a track
    Start {
        /// Track ID to orchestrate
        track_id: String,

        /// Mode: planning (analyze only) or building (implement)
        #[arg(long, default_value = "building")]
        mode: String,

        /// Agent tool to use
        #[arg(long, default_value = "claude")]
        tool: String,

        /// Agent model (optional, tool-dependent)
        #[arg(long)]
        model: Option<String>,

        /// Enable dangerous mode (auto-approve, no prompts)
        #[arg(long)]
        dangerous: bool,

        /// Enable sandbox mode (bubblewrap isolation)
        #[arg(long)]
        sandbox: bool,

        /// Tracks directory (defaults to ./maestro/tracks)
        #[arg(long)]
        tracks_dir: Option<PathBuf>,

        /// Max retries before failure
        #[arg(long, default_value = "3")]
        max_retries: u32,

        /// Error strategy: retry, skip, abort
        #[arg(long, default_value = "retry")]
        error_strategy: String,
    },

    /// Pause orchestrate loop for a track
    Pause {
        /// Track ID to pause
        track_id: String,

        /// Tracks directory
        #[arg(long)]
        tracks_dir: Option<PathBuf>,
    },

    /// Resume orchestrate loop for a track
    Resume {
        /// Track ID to resume
        track_id: String,

        /// Tracks directory
        #[arg(long)]
        tracks_dir: Option<PathBuf>,
    },

    /// Abort orchestrate loop for a track
    Abort {
        /// Track ID to abort
        track_id: String,

        /// Tracks directory
        #[arg(long)]
        tracks_dir: Option<PathBuf>,
    },

    /// Show status of orchestrate sessions
    Status {
        /// Track ID to check (optional, shows all if not specified)
        track_id: Option<String>,

        /// Tracks directory
        #[arg(long)]
        tracks_dir: Option<PathBuf>,
    },

    /// List all available tracks
    List {
        /// Tracks directory (defaults to ./maestro/tracks)
        #[arg(long)]
        tracks_dir: Option<PathBuf>,
    },
}

pub struct OrchestrateCommand {
    pub command: OrchestrateSubcommand,
}

pub async fn run(cmd: OrchestrateCommand) -> Result<()> {
    match cmd.command {
        OrchestrateSubcommand::Start {
            track_id,
            mode,
            tool,
            model,
            dangerous,
            sandbox,
            tracks_dir,
            max_retries,
            error_strategy,
        } => {
            let tracks_dir = tracks_dir.unwrap_or_else(|| PathBuf::from("./maestro/tracks"));

            let loop_mode = match mode.as_str() {
                "planning" => LoopMode::Planning,
                "building" => LoopMode::Building,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid mode: {}. Use 'planning' or 'building'",
                        mode
                    ))
                }
            };

            let error_strategy = match error_strategy.as_str() {
                "retry" => crate::orchestrate::model::ErrorStrategy::Retry,
                "skip" => crate::orchestrate::model::ErrorStrategy::Skip,
                "abort" => crate::orchestrate::model::ErrorStrategy::Abort,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid error strategy: {}. Use 'retry', 'skip', or 'abort'",
                        error_strategy
                    ))
                }
            };

            let config = OrchestrateConfig {
                max_retries,
                error_strategy,
                ..Default::default()
            };

            let agent_config = AgentConfig {
                tool,
                model,
                dangerous_mode: dangerous,
                sandbox,
            };

            let mut engine = OrchestrateEngine::new(config, tracks_dir)?;
            engine.start(&track_id, loop_mode, agent_config).await?;
            Ok(())
        }

        OrchestrateSubcommand::Pause {
            track_id,
            tracks_dir,
        } => {
            let tracks_dir = tracks_dir.unwrap_or_else(|| PathBuf::from("./maestro/tracks"));

            let config = OrchestrateConfig::default();
            let engine = OrchestrateEngine::new(config, tracks_dir)?;
            engine.pause(&track_id)?;
            Ok(())
        }

        OrchestrateSubcommand::Resume {
            track_id,
            tracks_dir,
        } => {
            let tracks_dir = tracks_dir.unwrap_or_else(|| PathBuf::from("./maestro/tracks"));

            let config = OrchestrateConfig::default();
            let engine = OrchestrateEngine::new(config, tracks_dir)?;
            engine.resume(&track_id)?;
            Ok(())
        }

        OrchestrateSubcommand::Abort {
            track_id,
            tracks_dir,
        } => {
            let tracks_dir = tracks_dir.unwrap_or_else(|| PathBuf::from("./maestro/tracks"));

            let config = OrchestrateConfig::default();
            let engine = OrchestrateEngine::new(config, tracks_dir)?;
            engine.abort(&track_id)?;
            Ok(())
        }

        OrchestrateSubcommand::Status {
            track_id,
            tracks_dir,
        } => {
            let tracks_dir = tracks_dir.unwrap_or_else(|| PathBuf::from("./maestro/tracks"));

            show_status(tracks_dir, track_id.as_deref()).await?;
            Ok(())
        }

        OrchestrateSubcommand::List { tracks_dir } => {
            let tracks_dir = tracks_dir.unwrap_or_else(|| PathBuf::from("./maestro/tracks"));

            list_tracks(tracks_dir).await?;
            Ok(())
        }
    }
}

async fn show_status(tracks_dir: PathBuf, track_id: Option<&str>) -> Result<()> {
    let config = OrchestrateConfig::default();
    let state_manager = crate::orchestrate::state::StateManager::new(config.data_dir.clone())?;

    if let Some(track_id) = track_id {
        // Show status for specific track
        if let Some(session) = state_manager.load_session(track_id)? {
            println!("Track: {}", session.track_id);
            println!("Mode: {:?}", session.mode);
            println!("Status: {:?}", session.status);
            println!("Iteration: {}", session.current_iteration);
            println!("Current Task: {:?}", session.current_task_id);
            println!("Started: {}", session.started_at);
            println!("Updated: {}", session.updated_at);
            println!("\nAgent Config:");
            println!("  Tool: {}", session.agent_config.tool);
            println!("  Model: {:?}", session.agent_config.model);
            println!("  Dangerous Mode: {}", session.agent_config.dangerous_mode);
            println!("  Sandbox: {}", session.agent_config.sandbox);

            // Show recent iterations
            let recent = state_manager.recent_iterations(track_id, 5)?;
            if !recent.is_empty() {
                println!("\nRecent Iterations:");
                for log in recent {
                    println!(
                        "  [{}] {}: {} - {}",
                        log.iteration,
                        log.completed_at.unwrap_or_else(|| log.started_at.clone()),
                        log.task_id,
                        format!("{:?}", log.status).to_lowercase()
                    );
                }
            }
        } else {
            println!("No active session found for track: {}", track_id);
        }
    } else {
        // Show all tracks with sessions
        println!("Scanning for orchestrate sessions...\n");

        let tracks_md = tracks_dir.join("tracks.md");
        if !tracks_md.exists() {
            println!("No tracks.md found at: {:?}", tracks_dir);
            return Ok(());
        }

        let tracks = parse_tracks_md(&tracks_md)?;
        let mut found_any = false;

        for track in tracks {
            if let Some(session) = state_manager.load_session(&track.id)? {
                found_any = true;
                println!("Track: {} ({})", track.id, track.description);
                println!("  Status: {:?}", session.status);
                println!("  Mode: {:?}", session.mode);
                println!("  Iteration: {}", session.current_iteration);
                if let Some(task) = session.current_task_id {
                    println!("  Current Task: {}", task);
                }
                println!();
            }
        }

        if !found_any {
            println!("No active orchestrate sessions found.");
        }
    }

    Ok(())
}

async fn list_tracks(tracks_dir: PathBuf) -> Result<()> {
    let tracks_md = tracks_dir.join("tracks.md");
    if !tracks_md.exists() {
        println!("No tracks.md found at: {:?}", tracks_dir);
        return Ok(());
    }

    let tracks = parse_tracks_md(&tracks_md)?;

    println!("Available Tracks:\n");

    for track in tracks {
        let status_symbol = match track.status {
            crate::orchestrate::model::TrackStatus::Pending => "[ ]",
            crate::orchestrate::model::TrackStatus::InProgress => "[~]",
            crate::orchestrate::model::TrackStatus::Completed => "[x]",
        };

        println!("{} {} - {}", status_symbol, track.id, track.description);
        println!("   Path: {:?}", track.link_path);

        // Check if plan.md exists and show task summary
        let plan_path = track.link_path.join("plan.md");
        if plan_path.exists() {
            if let Ok(plan) = parse_plan_md(&plan_path) {
                let all_tasks: Vec<&crate::orchestrate::model::Task> = plan.all_tasks();
                let completed = all_tasks
                    .iter()
                    .filter(|t| t.status == crate::orchestrate::model::TrackStatus::Completed)
                    .count();
                let total = all_tasks.len();

                if total > 0 {
                    let percent = (completed * 100) / total;
                    println!("   Progress: {}/{} tasks ({}%)", completed, total, percent);
                }
            }
        }

        println!();
    }

    Ok(())
}
