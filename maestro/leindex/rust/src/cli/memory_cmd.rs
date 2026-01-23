//! Memory command implementation
//!
//! Manages the Maestro Memory System including database, scanner, and API server.

use anyhow::Result;
use std::path::PathBuf;

/// Start the memory dashboard server
pub async fn serve(port: u16, host: String, db: Option<PathBuf>, debug: bool) -> Result<()> {
    println!("🚀 Starting Maestro Memory Dashboard");
    println!("   Host: {}:{}", host, port);

    let db_path = db.unwrap_or_else(|| {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".maestro");
        path.push("maestro.db");
        path
    });

    println!("   Database: {}", db_path.display());

    if debug {
        println!("   Debug: enabled");
    }

    // Use the api server module
    use crate::api::{run_server, ServerConfig};

    let config = ServerConfig {
        host,
        port,
        db_path: Some(db_path),
        enable_cors: true,
    };

    println!(
        "\n✨ Dashboard ready at http://{}:{}",
        config.host, config.port
    );
    println!("   Press Ctrl+C to stop\n");

    run_server(config).await
}

/// Show memory system status
pub async fn status(db: Option<PathBuf>) -> Result<()> {
    use crate::memory::service::MemoryService;

    let db_path = db.clone().unwrap_or_else(|| {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".maestro");
        path.push("maestro.db");
        path
    });

    println!("📊 Maestro Memory System Status");
    println!();

    let service = MemoryService::new(db)?;
    service.initialize()?;

    let db_stats = service.stats()?;

    println!("   Database: {} ✓", db_path.display());
    println!("   Projects: {}", db_stats.project_count);
    println!("   Memories: {}", db_stats.memory_count);
    println!("   Sessions: {}", db_stats.session_count);

    Ok(())
}

/// Scan directories for Maestro projects
pub async fn scan(paths: Vec<PathBuf>, depth: usize) -> Result<()> {
    println!("🔍 Scanning for Maestro projects");
    println!();

    let dirs: Vec<_> = if paths.is_empty() {
        // Default to common directories
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        vec![home.join("Prod"), home.join("Projects"), home.join("src")]
    } else {
        paths
    };

    for dir in &dirs {
        println!("   Scanning: {} (depth {})", dir.display(), depth);
    }
    println!();

    let mut found = 0;

    for base_dir in dirs {
        if !base_dir.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(&base_dir)
            .max_depth(depth)
            .into_iter()
            .filter_map(|e| e.ok())
        // Intentionally skip dirs with permission errors
        {
            let path = entry.path();

            // Check for Maestro markers
            let maestro_dir = path.join("maestro");
            let dot_maestro = path.join(".maestro");

            if maestro_dir.is_dir() || dot_maestro.is_dir() {
                found += 1;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                println!("   ✓ {} ({})", name, path.display());
            }
        }
    }

    println!();
    println!("Found {} Maestro project(s)", found);

    Ok(())
}
