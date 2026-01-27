//!
//! # Maestro LeIndex Migration Framework
//!
//! A comprehensive database migration framework for SQLite/DuckDB/Tantivy to Turso migrations.
//!
//! ## Modules
//!
//! - `migrations`: Core migration management with async support
//! - `vector`: HNSW-based vector index for similarity search
//! - `turso`: Hybrid storage configuration combining local SQLite and remote Turso
//! - `vector_migration`: Vector migration bridge for transferring embeddings
//!
//! ## Example
//!
//! ```rust
//! use maestro_leindex::migrations::{Migration, MigrationManager};
//!
//! #[derive(Debug)]
//! struct CreateUsersTable;
//!
//! #[async_trait::async_trait]
//! impl Migration for CreateUsersTable {
//!     fn version(&self) -> &str {
//!         "2024_01_01_001_create_users_table"
//!     }
//!
//!     async fn up(&self, db: &libsql::Database) -> anyhow::Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn down(&self, db: &libsql::Database) -> anyhow::Result<()> {
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Vector Migration (Task 8.3)
//!
//! ```rust
//! use maestro_leindex::vector_migration::VectorMigrationBridge;
//! use maestro_leindex::turso::{TursoConfig, HybridStorage};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = TursoConfig::hybrid(
//!         "libsql://token@db.turso.io".to_string(),
//!         "auth_token".to_string()
//!     );
//!     let storage = HybridStorage::new(config).await?;
//!
//!     let bridge = VectorMigrationBridge::new(storage);
//!     let progress = bridge.migrate_embeddings().await?;
//!
//!     println!("Migrated {} embeddings", progress.success_count);
//!     Ok(())
//! }
//! ```

pub mod migrations;
pub mod vector;
pub mod turso;
pub mod vector_migration;
