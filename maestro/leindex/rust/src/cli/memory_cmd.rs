//! Memory command implementation
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone, Debug)]
pub enum MemoryCommands {
    Serve { #[clap(short, long, default_value = "8080")] port: u16, #[clap(short, long, default_value = "127.0.0.1")] host: String },
    Status,
}

pub async fn run(cmd: MemoryCommands) -> Result<()> {
    match cmd {
        MemoryCommands::Serve { port, host } => super::memory_impl::serve(port, host, None, false).await,
        MemoryCommands::Status => super::memory_impl::status(None).await,
    }
}
