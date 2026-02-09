//! Maestro CLI - Main entry point
//!
//! Pure Rust implementation of the Maestro command-line interface.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Import the CLI module from the leindex_analyzers library
// This avoids the module being compiled twice (once in lib, once in binary)
use leindex_analyzers::cli::{analyze, implement, mcp, memory_impl as memory, tui};
use leindex_analyzers::cli::implement::ImplementSessionTarget;

/// Maestro - AI-Powered Project Orchestrator
#[derive(Parser)]
#[command(name = "maestro")]
#[command(author = "Maestro Team")]
#[command(version = "2.0.0")]
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
            } => memory::serve(port, host, db, debug).await,
            MemoryCommands::Status { db } => memory::status(db).await,
            MemoryCommands::Scan { paths, depth } => memory::scan(paths, depth).await,
        },
        Commands::Tui => tui::run().await,
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
    }
}
