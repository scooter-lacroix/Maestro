//! Maestro Cockpit - Terminal UI for Maestro orchestration
//!
//! Run with: maestro-cockpit

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    maestro_cockpit::run().await
}
