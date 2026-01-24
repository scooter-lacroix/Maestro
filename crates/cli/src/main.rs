//! Maestro CLI - Main entry point
//!
//! Pure Rust implementation of the Maestro command-line interface.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Re-export CLI modules from leindex-core
use leindex_core::cli::analyze;
use leindex_core::cli::implement::ImplementSessionTarget;
use leindex_core::cli::implement;
use leindex_core::cli::leindex_cmd;
use leindex_core::cli::memory_cmd as memory;
use leindex_core::cli::mcp;
use leindex_core::cli::orchestrate;

// Local CLI commands
mod commands;
use commands::{configure, pi_status, pi_test, pi_agents};

/// Maestro - AI-Powered Project Orchestrator
#[derive(Parser)]
#[command(name = "maestro")]
#[command(author = "Maestro Team")]
#[command(version = "2.5.0")]
#[command(about = "Spec-driven development framework for AI-assisted software engineering")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze source code files
    Analyze {
        /// File or directory to analyze
        path: PathBuf,

        /// Output format: json, llm, ultra
        #[arg(short, long, default_value = "llm")]
        format: String,

        /// Language (auto-detected if not specified)
        #[arg(short, long)]
        language: Option<String>,

        /// Analysis type: ast, callgraph, cfg, dfg, slicing, all
        #[arg(short, long, default_value = "all")]
        analysis: String,
    },

    /// Configure Maestro integrations
    Configure {
        /// Enable pi-mono configuration wizard
        #[arg(long)]
        pi_mono: bool,
    },

    /// LeIndex project-level operations (index, search, 5-phase analysis)
    LeIndex {
        #[command(subcommand)]
        command: leindex_cmd::LeIndexSubcommand,
    },

    /// Maestro Memory System operations
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },

    /// Launch Maestro Terminal UI (Cockpit)
    Tui,

    /// Initiate a track implementation in tmux
    Implement {
        /// Track command to run (e.g. /maestro:implement)
        command: String,

        /// Track description (freeform)
        description: Vec<String>,

        /// Where to run: ask | current | new
        #[arg(long, value_enum, default_value_t = ImplementSessionTarget::Ask)]
        session: ImplementSessionTarget,

        /// Tool for new sessions (claude, gemini, opencode, amp, shell, ...)
        #[arg(long, default_value = "claude")]
        tool: String,

        /// Working directory / project path for new sessions (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,

        /// Title for the new tmux session (defaults to derived from description)
        #[arg(long)]
        title: Option<String>,

        /// Pi-Mono: Execute with single agent (scout, planner, reviewer, worker)
        #[arg(long, conflicts_with_all = ["pi_chain", "pi_parallel"])]
        pi_agent: Option<String>,

        /// Pi-Mono: Execute with chain of agents (comma-separated: scout,planner,worker)
        #[arg(long, value_delimiter = ',', conflicts_with_all = ["pi_agent", "pi_parallel"])]
        pi_chain: Option<Vec<String>>,

        /// Pi-Mono: Execute with parallel agents (comma-separated: worker,worker,worker)
        #[arg(long, value_delimiter = ',', conflicts_with_all = ["pi_agent", "pi_chain"])]
        pi_parallel: Option<Vec<String>>,
    },

    /// MCP pooling, proxying, and tool search
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Orchestrate autonomous track execution (Ralph-style loops)
    Orchestrate {
        #[command(subcommand)]
        command: OrchestrateCommands,
    },

    /// Show Pi-Mono integration status
    PiStatus {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Test Pi-Mono subagent execution
    PiTest {
        /// Task description to execute
        task: String,

        /// Agent type to use (scout, planner, reviewer, worker)
        #[arg(short, long)]
        agent: Option<String>,

        /// Timeout in seconds
        #[arg(short, long)]
        timeout: Option<u64>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List Pi-Mono agent mappings
    PiAgents {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Start pooled stdio MCP servers on UNIX sockets
    Serve,
    /// Bridge stdio to a pooled UNIX socket server
    Proxy {
        /// MCP server name
        name: String,
    },
    /// Meta MCP server: tool search + cross-server tool call
    ToolSearch,
}

#[derive(Subcommand)]
enum OrchestrateCommands {
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

#[derive(Subcommand)]
enum MemoryCommands {
    /// Start the Maestro Memory Dashboard web server
    Serve {
        /// Port to run the dashboard on
        #[arg(short, long, default_value = "18765")]
        port: u16,

        /// Host to bind the dashboard to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Path to database file
        #[arg(short, long)]
        db: Option<PathBuf>,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,
    },

    /// Show Maestro memory system status
    Status {
        /// Path to database file
        #[arg(short, long)]
        db: Option<PathBuf>,
    },

    /// Scan directories for Maestro projects
    Scan {
        /// Directories to scan
        paths: Vec<PathBuf>,

        /// Maximum depth to scan
        #[arg(short, long, default_value = "5")]
        depth: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("maestro=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            path,
            format,
            language,
            analysis,
        } => analyze::run(path, format, language, analysis).await,
        Commands::Configure { pi_mono } => configure::run(pi_mono).await,
        Commands::LeIndex { command } => leindex_cmd::run(leindex_cmd::LeIndexCommand { command }).await,
        Commands::Memory { command } => match command {
            MemoryCommands::Serve {
                port,
                host,
                db,
                debug,
            } => memory::serve(port, host, db, debug).await,
            MemoryCommands::Status { db } => memory::status(db).await,
            MemoryCommands::Scan { paths, depth } => memory::scan(paths, depth).await,
        },
        Commands::Tui => maestro_cockpit::run().await,
        Commands::Implement {
            command,
            description,
            session,
            tool,
            path,
            title,
            pi_agent,
            pi_chain,
            pi_parallel,
        } => commands::implement::run(command, description, session, tool, path, title, pi_agent, pi_chain, pi_parallel).await,
        Commands::Mcp { command } => match command {
            McpCommands::Serve => mcp::serve().await,
            McpCommands::Proxy { name } => mcp::proxy(name).await,
            McpCommands::ToolSearch => mcp::tool_search().await,
        },
        Commands::Orchestrate { command } => match command {
            OrchestrateCommands::Start {
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
                orchestrate::run(orchestrate::OrchestrateCommand {
                    command: orchestrate::OrchestrateSubcommand::Start {
                        track_id,
                        mode,
                        tool,
                        model,
                        dangerous,
                        sandbox,
                        tracks_dir,
                        max_retries,
                        error_strategy,
                    }
                }).await
            }
            OrchestrateCommands::Pause { track_id, tracks_dir } => {
                orchestrate::run(orchestrate::OrchestrateCommand {
                    command: orchestrate::OrchestrateSubcommand::Pause { track_id, tracks_dir },
                }).await
            }
            OrchestrateCommands::Resume { track_id, tracks_dir } => {
                orchestrate::run(orchestrate::OrchestrateCommand {
                    command: orchestrate::OrchestrateSubcommand::Resume { track_id, tracks_dir },
                }).await
            }
            OrchestrateCommands::Abort { track_id, tracks_dir } => {
                orchestrate::run(orchestrate::OrchestrateCommand {
                    command: orchestrate::OrchestrateSubcommand::Abort { track_id, tracks_dir },
                }).await
            }
            OrchestrateCommands::Status { track_id, tracks_dir } => {
                orchestrate::run(orchestrate::OrchestrateCommand {
                    command: orchestrate::OrchestrateSubcommand::Status { track_id, tracks_dir },
                }).await
            }
            OrchestrateCommands::List { tracks_dir } => {
                orchestrate::run(orchestrate::OrchestrateCommand {
                    command: orchestrate::OrchestrateSubcommand::List { tracks_dir },
                }).await
            }
        },
        Commands::PiStatus { config, verbose, json } => {
            pi_status::run(config, verbose, json).await
        }
        Commands::PiTest { task, agent, timeout, verbose } => {
            pi_test::run(task, agent, timeout, verbose).await
        }
        Commands::PiAgents { config, verbose, json } => {
            pi_agents::run(config, verbose, json).await
        }
    }
}
