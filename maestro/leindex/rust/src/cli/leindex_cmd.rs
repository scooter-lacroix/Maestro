//! LeIndex command implementation
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone, Debug)]
pub enum LeIndexCommands {
    Init { path: Option<String> },
    Search { query: String },
    Status,
    Analyze { file: String },
}

// Type alias for compatibility with main.rs
pub use LeIndexCommands as LeIndexSubcommand;

pub async fn run(cmd: LeIndexCommands) -> Result<()> {
    match cmd {
        LeIndexCommands::Init { path } => { eprintln!("Init: {:?}", path); Ok(()) }
        LeIndexCommands::Search { query } => { eprintln!("Search: {}", query); Ok(()) }
        LeIndexCommands::Status => { eprintln!("LeIndex Status: TODO"); Ok(()) }
        LeIndexCommands::Analyze { file } => { eprintln!("Analyze: {}", file); Ok(()) }
    }
}
