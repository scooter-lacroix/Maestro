//! Memory command implementation
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Clone, Debug)]
pub enum MemoryCommands {
    /// Start the memory dashboard server
    Serve {
        #[clap(short, long, default_value = "8080")]
        port: u16,
        #[clap(short, long, default_value = "127.0.0.1")]
        host: String,
        #[clap(short, long)]
        db: Option<PathBuf>,
        #[clap(long)]
        debug: bool,
    },
    /// Show memory system status
    Status {
        #[clap(short, long)]
        db: Option<PathBuf>,
    },
    /// Scan directories for Maestro projects
    Scan {
        #[clap(short, long, default_value = "5")]
        depth: usize,
        paths: Vec<PathBuf>,
    },
    /// Store a memory in the Maestro Memory System
    Store {
        #[clap(short, long)]
        content: String,
        #[clap(short, long, default_value = "observation")]
        category: String,
        #[clap(short, long, default_value = "normal")]
        importance: String,
        #[clap(short, long)]
        db: Option<PathBuf>,
    },
}

pub async fn run(cmd: MemoryCommands) -> Result<()> {
    match cmd {
        MemoryCommands::Serve { port, host, db, debug } => {
            super::memory_impl::serve(port, host, db, debug).await
        }
        MemoryCommands::Status { db } => super::memory_impl::status(db).await,
        MemoryCommands::Scan { depth, paths } => super::memory_impl::scan(paths, depth).await,
        MemoryCommands::Store { content, category, importance, db } => {
            super::memory_impl::store(content, category, importance, db).await
        }
    }
}

/// Store a memory in the Maestro Memory System
#[cfg(feature = "rusqlite")]
pub async fn store(
    content: String,
    category: String,
    _importance: String,
    db: Option<PathBuf>,
) -> Result<()> {
    use crate::memory::{MemoryCategory, MemoryService};

    let db_path = db.unwrap_or_else(|| {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".maestro");
        path.push("maestro.db");
        path
    });

    // Parse category
    let memory_category = match category.to_lowercase().as_str() {
        "context" => MemoryCategory::Context,
        "knowledge" => MemoryCategory::Knowledge,
        "preference" => MemoryCategory::Preference,
        "specification" => MemoryCategory::Specification,
        "fact" => MemoryCategory::Fact,
        "pattern" => MemoryCategory::Pattern,
        "decision" => MemoryCategory::Decision,
        "observation" => MemoryCategory::Observation,
        "temporary" => MemoryCategory::Temporary,
        _ => {
            eprintln!(
                "Warning: Unknown category '{}', using 'observation'",
                category
            );
            MemoryCategory::Observation
        }
    };

    let service = MemoryService::new(Some(db_path))?;
    service.initialize()?;

    // Store the memory (project is auto-created from current directory)
    let memory_id = service.store_memory(&content, memory_category)?;

    println!("✓ Memory stored successfully");
    println!("  ID: {}", memory_id);
    println!("  Category: {}", category);
    println!("  Content: {}", content);

    Ok(())
}
