//!
//! # Maestro LeIndex Migration CLI
//!
//! CLI tool for running database migrations and vector embeddings migration.
//!

use maestro_leindex::migrations::{Migration, MigrationManager};
use maestro_leindex::turso::{TursoConfig, HybridStorage};
use maestro_leindex::vector_migration::VectorMigrationBridge;
use libsql::Connection;

#[derive(Debug)]
struct SampleMigration;

#[async_trait::async_trait]
impl Migration for SampleMigration {
    fn version(&self) -> &str {
        "2024_01_01_001_initial"
    }

    async fn up(&self, _conn: &Connection) -> anyhow::Result<()> {
        println!("Running sample migration up...");
        Ok(())
    }

    async fn down(&self, _conn: &Connection) -> anyhow::Result<()> {
        println!("Running sample migration down...");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Maestro LeIndex Migration Tool");
    println!("=============================");
    println!();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "migrate" => run_migrations().await?,
        "vector-migrate" => run_vector_migration().await?,
        "help" => print_usage(),
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: maestro-leindex <command>");
    println!();
    println!("Commands:");
    println!("  migrate         Run database migrations");
    println!("  vector-migrate  Migrate vector embeddings to Turso (Task 8.3)");
    println!("  help            Show this help message");
    println!();
    println!("Environment Variables:");
    println!("  TURSO_URL       Turso database URL (e.g., libsql://token@db.turso.io)");
    println!("  TURSO_AUTH      Turso auth token");
    println!("  LOCAL_DB        Path to local SQLite database (default: local.db)");
}

async fn run_migrations() -> anyhow::Result<()> {
    println!("Running migrations...");
    println!();

    // For local migrations, use a local database
    let db_url = std::env::var("LOCAL_DB").unwrap_or_else(|_| "local.db".to_string());
    let db = libsql::Database::open(&db_url)?;
    let conn = db.connect()?;

    let manager = MigrationManager::new(conn);

    // Initialize migrations table
    manager.initialize().await?;

    // Run sample migration
    let applied = manager.run_migrations(&[&SampleMigration], false).await?;

    println!("Applied {} migration(s)", applied);

    // Get status
    let status = manager.get_status().await?;
    println!();
    println!("Migration Status:");
    println!("  Applied: {}", status.applied_count);
    println!("  Pending: {}", status.pending_count);
    println!("  Rolled Back: {}", status.rolled_back_count);

    Ok(())
}

async fn run_vector_migration() -> anyhow::Result<()> {
    println!("Vector Embedding Migration (Task 8.3)");
    println!("===================================");
    println!();

    // Get configuration from environment
    let turso_url = std::env::var("TURSO_URL")
        .map_err(|_| anyhow::anyhow!("TURSO_URL environment variable not set"))?;

    let turso_auth = std::env::var("TURSO_AUTH")
        .map_err(|_| anyhow::anyhow!("TURSO_AUTH environment variable not set"))?;

    let local_db = std::env::var("LOCAL_DB").unwrap_or_else(|_| "local.db".to_string());

    println!("Configuration:");
    println!("  Local DB: {}", local_db);
    println!("  Turso URL: {}", turso_url);
    println!("  Turso Auth: ***");
    println!();

    // Create hybrid storage configuration
    let config = TursoConfig::hybrid(
        format!("file:{}", local_db),
        turso_auth.clone(),
    );

    // Override remote URL
    let config = TursoConfig {
        database_url: turso_url.clone(),
        auth_token: turso_auth,
        enable_vectors: true,
        remote_only: false,
    };

    // Create hybrid storage
    let storage = HybridStorage::new(config).await?;

    println!("Storage mode: {:?}", storage.mode());
    println!();

    // Initialize vector extension if enabled
    if storage.config.enable_vectors {
        println!("Initializing vector extension...");
        storage.init_vectors().await?;
        println!("Vector extension initialized");
        println!();
    }

    // Create migration bridge
    let bridge = VectorMigrationBridge::new(storage)
        .with_max_concurrency(10)
        .with_batch_size(100);

    // Run migration
    println!("Starting vector migration...");
    println!();

    let progress = bridge.migrate_embeddings().await?;

    // Print results
    println!();
    println!("Migration Results:");
    println!("  Total: {}", progress.total);
    println!("  Succeeded: {}", progress.success_count);
    println!("  Failed: {}", progress.error_count);
    println!("  Progress: {:.1}%", progress.progress_percent());

    if let Some(duration) = progress.duration_ms {
        println!("  Duration: {}ms", duration);
    }

    if !progress.errors.is_empty() {
        println!();
        println!("Errors:");
        for error in progress.errors.iter().take(10) {
            println!("  [{}] {}", error.id, error.error);
        }
        if progress.errors.len() > 10 {
            println!("  ... and {} more", progress.errors.len() - 10);
        }
    }

    if progress.is_successful() {
        println!();
        println!("Migration completed successfully!");
    } else {
        println!();
        println!("Migration completed with errors.");
    }

    Ok(())
}
