//! Maestro CLI - Main entry point
//!
//! Pure Rust implementation of the Maestro command-line interface.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Import the library to make all modules available
// The CLI modules are part of the lib, so we use lib's module structure
pub use leindex_core::*;

// Don't re-declare mod cli - it's already available from the lib
use leindex_core::cli::implement::ImplementSessionTarget;
use leindex_core::cli::integrate::{IntegrateAction, IntegrationTool};
use leindex_core::cli::mcp;
use leindex_core::cli::{analyze, implement, integrate, memory_impl as memory};

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

    /// Maestro Memory System operations
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },

    /// Launch Maestro Terminal UI
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
    },

    /// MCP pooling, proxying, and tool search
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Integrate Maestro with external CLI tools (OpenCode, Codex, Gemini, Qwen, Amp, Droid)
    Integrate {
        /// Integration action to perform
        #[command(subcommand)]
        action: IntegrateAction,

        /// Tool to integrate (claude, opencode, codex, gemini, qwen, amp, droid)
        #[arg(short, long)]
        tool: Option<IntegrationTool>,

        /// Install all integrations
        #[arg(long, conflicts_with = "tool")]
        all: bool,

        /// Dry run (show changes without applying)
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
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
        Commands::Memory { command } => match command {
            MemoryCommands::Serve {
                port,
                host,
                db,
                debug,
            } => mem_cmd::serve(port, host, db, debug).await,
            MemoryCommands::Status { db } => mem_cmd::status(db).await,
            MemoryCommands::Scan { paths, depth } => mem_cmd::scan(paths, depth).await,
        },
        Commands::Tui => {
            // TODO: The Cockpit TUI has been extracted to the maestro-cockpit crate
            // Use the separate maestro CLI (crates/cli) to access the TUI: `maestro tui`
            eprintln!("Error: The TUI has been moved to the maestro-cockpit crate.");
            eprintln!("Please use the maestro CLI binary from crates/cli instead.");
            eprintln!("Run: cargo run --manifest-path=crates/cli/Cargo.toml --bin maestro -- tui");
            Err(anyhow::anyhow!("TUI not available from this binary"))
        }
        Commands::Implement {
            command,
            description,
            session,
            tool,
            path,
            title,
        } => implement::run(command, description, session, tool, path, title).await,
        Commands::Mcp { command } => match command {
            McpCommands::Serve => mcp::serve().await,
            McpCommands::Proxy { name } => mcp::proxy(name).await,
            McpCommands::ToolSearch => mcp::tool_search().await,
        },
        Commands::Integrate {
            action,
            mut tool,
            all,
            dry_run,
            verbose,
        } => {
            // If --all is specified, set tool to None to indicate install all
            if all {
                tool = None;
            }
            integrate::run(action, tool, dry_run, verbose).await
        }
    }
}
