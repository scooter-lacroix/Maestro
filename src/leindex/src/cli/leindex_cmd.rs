//! LeIndex command implementation
//!
//! Project-level operations: index, search, analyze phases

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::five_phase;
use crate::five_phase::PhaseOptions;
use crate::token_format::FormatMode;

/// LeIndex project-level operations
#[derive(Parser, Clone)]
pub struct LeIndexCommand {
    #[command(subcommand)]
    pub command: LeIndexSubcommand,
}

#[derive(Subcommand, Clone)]
pub enum LeIndexSubcommand {
    /// Initialize and index a project
    Init {
        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Force re-index even if index exists
        #[arg(long)]
        force: bool,
    },

    /// Show index status
    Status,

    /// Search code with hybrid full-text + semantic search
    Search {
        /// Search query
        query: String,

        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Search within specific file
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Run 5-phase project analysis
    Analyze {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Run specific phase (1-5), or 'all'
        #[arg(short, long)]
        phase: Option<String>,

        /// Mode: ultra, balanced, verbose
        #[arg(short, long, default_value = "balanced")]
        mode: String,

        /// Max files to analyze
        #[arg(long, default_value = "25")]
        max_files: usize,

        /// Max output chars
        #[arg(short = 'C', long, default_value = "12000")]
        max_chars: usize,
    },

    /// Direct phase access (phase1)
    Phase1 {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Mode: ultra, balanced, verbose
        #[arg(short, long, default_value = "balanced")]
        mode: String,

        /// Max files to analyze
        #[arg(long, default_value = "25")]
        max_files: usize,

        /// Max output chars
        #[arg(short = 'C', long, default_value = "12000")]
        max_chars: usize,
    },

    /// Direct phase access (phase2)
    Phase2 {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Mode: ultra, balanced, verbose
        #[arg(short, long, default_value = "balanced")]
        mode: String,

        /// Max files to analyze
        #[arg(long, default_value = "25")]
        max_files: usize,

        /// Max output chars
        #[arg(short = 'C', long, default_value = "12000")]
        max_chars: usize,
    },

    /// Direct phase access (phase3)
    Phase3 {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Mode: ultra, balanced, verbose
        #[arg(short, long, default_value = "balanced")]
        mode: String,

        /// Max files to analyze
        #[arg(long, default_value = "25")]
        max_files: usize,

        /// Max output chars
        #[arg(short = 'C', long, default_value = "12000")]
        max_chars: usize,
    },

    /// Direct phase access (phase4)
    Phase4 {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Mode: ultra, balanced, verbose
        #[arg(short, long, default_value = "balanced")]
        mode: String,

        /// Max files to analyze
        #[arg(long, default_value = "25")]
        max_files: usize,

        /// Max output chars
        #[arg(short = 'C', long, default_value = "12000")]
        max_chars: usize,
    },

    /// Direct phase access (phase5)
    Phase5 {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Mode: ultra, balanced, verbose
        #[arg(short, long, default_value = "balanced")]
        mode: String,

        /// Max files to analyze
        #[arg(long, default_value = "25")]
        max_files: usize,

        /// Max output chars
        #[arg(short = 'C', long, default_value = "12000")]
        max_chars: usize,
    },

    /// Generate context bundle for a file or function
    Context {
        /// Target file or function
        target: String,

        /// Project root
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: json, llm
        #[arg(short, long, default_value = "llm")]
        format: String,

        /// Include caller analysis
        #[arg(long)]
        include_callers: bool,

        /// Include callee analysis
        #[arg(long)]
        include_callees: bool,
    },
}

pub async fn run(cmd: LeIndexCommand) -> Result<()> {
    match cmd.command {
        LeIndexSubcommand::Init { path, force } => run_init(path, force).await,
        LeIndexSubcommand::Status => run_status().await,
        LeIndexSubcommand::Search {
            query,
            limit,
            file,
            format,
        } => run_search(query, limit, file, format).await,
        LeIndexSubcommand::Analyze {
            path,
            phase,
            mode,
            max_files,
            max_chars,
        } => run_analyze(path, phase, mode, max_files, max_chars).await,
        LeIndexSubcommand::Phase1 {
            path,
            mode,
            max_files,
            max_chars,
        } => run_phase(1, path, mode, max_files, max_chars).await,
        LeIndexSubcommand::Phase2 {
            path,
            mode,
            max_files,
            max_chars,
        } => run_phase(2, path, mode, max_files, max_chars).await,
        LeIndexSubcommand::Phase3 {
            path,
            mode,
            max_files,
            max_chars,
        } => run_phase(3, path, mode, max_files, max_chars).await,
        LeIndexSubcommand::Phase4 {
            path,
            mode,
            max_files,
            max_chars,
        } => run_phase(4, path, mode, max_files, max_chars).await,
        LeIndexSubcommand::Phase5 {
            path,
            mode,
            max_files,
            max_chars,
        } => run_phase(5, path, mode, max_files, max_chars).await,
        LeIndexSubcommand::Context {
            target,
            path,
            format,
            include_callers,
            include_callees,
        } => run_context(target, path, format, include_callers, include_callees).await,
    }
}

async fn run_init(path: PathBuf, _force: bool) -> Result<()> {
    println!("Initializing LeIndex for: {}", path.display());
    println!(
        "Note: Full indexing not yet implemented - use `leindex analyze` for file-level analysis"
    );
    Ok(())
}

async fn run_status() -> Result<()> {
    println!("LeIndex Status");
    println!("=============");
    println!("Index location: ~/.config/maestro/leindex/");
    println!("Supported languages: Python, TypeScript, JavaScript, Rust, Go, Java, C, C++");
    println!("\nUse `leindex init .` to initialize a project index");
    Ok(())
}

async fn run_search(
    query: String,
    _limit: usize,
    _file: Option<PathBuf>,
    _format: String,
) -> Result<()> {
    println!("LeIndex Search: '{}'", query);
    println!(
        "Note: Full-text search not yet implemented - use `leindex analyze` for file-level analysis"
    );
    Ok(())
}

async fn run_analyze(
    path: PathBuf,
    phase: Option<String>,
    mode: String,
    max_files: usize,
    max_chars: usize,
) -> Result<()> {
    let format_mode = parse_mode(&mode);
    let opts = PhaseOptions {
        root: path,
        mode: format_mode,
        max_files,
        max_focus_files: (max_files / 8).max(1),
        top_n: 15,
        max_output_chars: max_chars,
    };

    if let Some(p) = phase {
        let phase_num = p.parse::<usize>().unwrap_or(0);
        if !(1..=5).contains(&phase_num) {
            eprintln!("Invalid phase: {}. Use 1-5 or 'all'", p);
            return Ok(());
        }
        run_phase(phase_num, opts.root, mode, max_files, max_chars).await
    } else {
        // Run all phases
        for p in 1..=5 {
            run_phase(p, opts.root.clone(), mode.clone(), max_files, max_chars).await?;
            println!();
        }
        Ok(())
    }
}

async fn run_phase(
    phase: usize,
    path: PathBuf,
    mode: String,
    max_files: usize,
    max_chars: usize,
) -> Result<()> {
    let format_mode = parse_mode(&mode);
    let opts = PhaseOptions {
        root: path,
        mode: format_mode,
        max_files,
        max_focus_files: (max_files / 8).max(1),
        top_n: 15,
        max_output_chars: max_chars,
    };

    let result = match phase {
        1 => five_phase::phase1_structural_scan(&opts),
        2 => five_phase::phase2_dependency_map(&opts),
        3 => five_phase::phase3_logic_flow(&opts),
        4 => five_phase::phase4_critical_path(&opts),
        5 => five_phase::phase5_optimization_report(&opts),
        _ => return Err(anyhow::anyhow!("Invalid phase: {}", phase)),
    };

    match result {
        Ok(output) => print!("{}", output),
        Err(e) => eprintln!("Error running phase {}: {}", phase, e),
    }

    Ok(())
}

async fn run_context(
    target: String,
    path: PathBuf,
    format: String,
    _include_callers: bool,
    _include_callees: bool,
) -> Result<()> {
    println!("LeIndex Context: {}", target);
    println!("Project: {}", path.display());
    println!("Format: {}", format);
    println!("Note: Full context extraction not yet implemented");
    Ok(())
}

fn parse_mode(mode: &str) -> FormatMode {
    match mode.to_lowercase().as_str() {
        "ultra" => FormatMode::Ultra,
        "verbose" => FormatMode::Verbose,
        _ => FormatMode::Balanced,
    }
}
