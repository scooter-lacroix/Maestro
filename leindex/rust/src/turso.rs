//!
//! # Turso Hybrid Storage Module
//!
//! Provides hybrid storage configuration combining local SQLite with remote Turso.
//! Enables local-first development with optional remote Turso storage for production scale.
//!
//! ## Features
//!
//! - **Local-First**: Fast local SQLite for development
//! - **Remote Turso**: Optional remote storage for production
//! - **Vector Extension**: Support for Turso's vec0 extension
//! - **Migration Bridge**: Data migration from local to remote
//!
//! ## Example
//!
//! ```rust
//! use turso::{TursoConfig, HybridStorage};
//!
//! let config = TursoConfig::local_only();
//! let storage = HybridStorage::new(config)?;
//! ```

use anyhow::{Context, Result};
use libsql::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Retry configuration for Turso operations
#[derive(Debug, Clone)]
struct RetryConfig {
    /// Maximum number of retry attempts
    max_attempts: usize,
    /// Initial delay between retries (in milliseconds)
    initial_delay_ms: u64,
    /// Backoff multiplier for exponential backoff
    backoff_multiplier: f64,
    /// Maximum delay between retries (in milliseconds)
    max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_delay_ms: 5000,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom values
    pub fn new(max_attempts: usize, initial_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            ..Default::default()
        }
    }
}

/// Retry an async operation with exponential backoff
async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = Duration::from_millis(config.initial_delay_ms);
    let mut last_error = None;

    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    tracing::info!(
                        "{} succeeded after {} retries",
                        operation_name,
                        attempt
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_attempts - 1 {
                    tracing::warn!(
                        "{} attempt {} failed, retrying in {:?}: {:?}",
                        operation_name,
                        attempt + 1,
                        delay,
                        last_error
                    );
                    sleep(delay).await;
                    delay = Duration::from_millis(
                        (delay.as_millis() as f64 * config.backoff_multiplier)
                            .min(config.max_delay_ms as f64) as u64
                    );
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("Unknown error after retries")))
        .context(format!("{} failed after {} attempts", operation_name, config.max_attempts))
}

/// Turso configuration for hybrid storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TursoConfig {
    /// Database URL (e.g., libsql://token@db.turso.io)
    /// For local-only mode, use "file:local.db"
    pub database_url: String,
    /// Auth token for Turso
    /// Empty string for local-only mode
    pub auth_token: String,
    /// Enable vector extension in Turso
    pub enable_vectors: bool,
    /// Remote-only mode (no local SQLite)
    pub remote_only: bool,
}

impl Default for TursoConfig {
    fn default() -> Self {
        Self {
            database_url: "file:local.db".to_string(),
            auth_token: String::new(),
            enable_vectors: false,
            remote_only: false,
        }
    }
}

impl TursoConfig {
    /// Create a new Turso config
    pub fn new(database_url: String, auth_token: String) -> Self {
        Self {
            database_url,
            auth_token,
            enable_vectors: false,
            remote_only: false,
        }
    }

    /// Create local-only config
    pub fn local_only() -> Self {
        Self {
            database_url: "file:local.db".to_string(),
            auth_token: String::new(),
            enable_vectors: false,
            remote_only: false,
        }
    }

    /// Create remote-only config
    pub fn remote_only(database_url: String, auth_token: String) -> Self {
        Self {
            database_url,
            auth_token,
            enable_vectors: false,
            remote_only: true,
        }
    }

    /// Create hybrid config (local + remote)
    pub fn hybrid(database_url: String, auth_token: String) -> Self {
        Self {
            database_url,
            auth_token,
            enable_vectors: false,
            remote_only: false,
        }
    }

    /// Enable vector extension
    pub fn with_vectors(mut self, enable: bool) -> Self {
        self.enable_vectors = enable;
        self
    }

    /// Check if this is a local-only configuration
    pub fn is_local_only(&self) -> bool {
        self.database_url.starts_with("file:") || self.auth_token.is_empty()
    }

    /// Check if this is a remote configuration
    pub fn is_remote(&self) -> bool {
        !self.is_local_only()
    }
}

/// Migration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStats {
    /// Number of nodes migrated
    pub nodes_migrated: usize,
    /// Number of edges migrated
    pub edges_migrated: usize,
    /// Number of embeddings migrated
    pub embeddings_migrated: usize,
    /// Time taken for migration (milliseconds)
    pub migration_time_ms: u64,
}

impl Default for MigrationStats {
    fn default() -> Self {
        Self {
            nodes_migrated: 0,
            edges_migrated: 0,
            embeddings_migrated: 0,
            migration_time_ms: 0,
        }
    }
}

/// Storage mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    /// No storage configured
    None,
    /// Local-only storage
    LocalOnly,
    /// Remote-only storage
    RemoteOnly,
    /// Hybrid storage (local + remote)
    Hybrid,
}

/// Hybrid storage: local SQLite + remote Turso
pub struct HybridStorage {
    /// Local SQLite connection (rusqlite)
    pub local: Option<Arc<Mutex<rusqlite::Connection>>>,
    /// Remote Turso connection
    pub remote: Option<Arc<Connection>>,
    /// Configuration
    pub config: TursoConfig,
}

impl HybridStorage {
    /// Create hybrid storage from configuration
    pub async fn new(config: TursoConfig) -> Result<Self> {
        // Initialize local storage if not remote-only
        let local = if !config.remote_only {
            let conn = rusqlite::Connection::open(&config.database_url.replace("file:", ""))
                .context("Failed to open local SQLite database")?;
            Some(Arc::new(Mutex::new(conn)))
        } else {
            None
        };

        // Initialize remote storage if configured
        let remote = if config.is_remote() {
            tracing::info!("Establishing Turso connection with retry logic...");

            // Use retry logic for establishing Turso connection
            let retry_config = RetryConfig::default();
            let db_url = config.database_url.clone();
            let auth_token = config.auth_token.clone();

            let db = retry_with_backoff(
                || {
                    let url = db_url.clone();
                    let token = auth_token.clone();
                    async move {
                        libsql::Database::open_remote(&url, &token)
                            .context("Failed to open Turso database")
                    }
                },
                &retry_config,
                "Turso database open",
            ).await?;

            // Connect without retry for now (connection establishment is fast)
            // The Database object handles connection pooling internally
            let conn = db.connect()
                .context("Failed to connect to Turso")?;

            tracing::info!("Turso connection established successfully");
            Some(Arc::new(conn))
        } else {
            None
        };

        Ok(Self { local, remote, config })
    }

    /// Initialize vector extension in Turso
    pub async fn init_vectors(&self) -> Result<()> {
        if !self.config.enable_vectors {
            return Ok(());
        }

        if let Some(remote) = &self.remote {
            // Load vector extension
            remote
                .execute("SELECT load_extension('vec0')", ())
                .await
                .context("Failed to load vec0 extension")?;

            tracing::info!("Vector extension (vec0) initialized");
        }

        Ok(())
    }

    /// Get local storage
    pub fn local(&self) -> Option<&Arc<Mutex<rusqlite::Connection>>> {
        self.local.as_ref()
    }

    /// Get remote storage
    pub fn remote(&self) -> Option<&Arc<Connection>> {
        self.remote.as_ref()
    }

    /// Migrate data from local to remote
    pub async fn migrate_to_remote(&self) -> Result<MigrationStats> {
        let start = std::time::Instant::now();

        let Some(local_conn) = &self.local else {
            anyhow::bail!("No local storage to migrate from");
        };

        let Some(remote_conn) = &self.remote else {
            anyhow::bail!("No remote storage configured");
        };

        let mut stats = MigrationStats::default();

        // Get local connection
        let local = local_conn.lock().await;

        // Migrate nodes
        {
            let mut node_stmt = local.prepare("SELECT id, data FROM nodes")?;
            let mut node_rows = node_stmt.query([])?;

            let mut nodes = Vec::new();
            while let Some(row) = node_rows.next()? {
                let id: String = row.get(0)?;
                let data: String = row.get(1)?;
                nodes.push((id, data));
                stats.nodes_migrated += 1;
            }
            drop(node_rows);
            drop(node_stmt);

            // Insert nodes to remote
            for (id, data) in &nodes {
                remote_conn
                    .execute(
                        "INSERT OR REPLACE INTO nodes (id, data) VALUES (?1, ?2)",
                        [id.as_str(), data.as_str()],
                    )
                    .await?;
            }
        }

        // Migrate embeddings
        {
            let mut embed_stmt = local.prepare("SELECT id, embedding FROM embeddings")?;
            let mut embed_rows = embed_stmt.query([])?;

            while let Some(row) = embed_rows.next()? {
                let id: String = row.get(0)?;
                let embedding_str: String = row.get(1)?;
                let embedding: Vec<f32> = serde_json::de::from_str(&embedding_str)?;

                // Convert embedding to JSON array for storage
                let embedding_json = serde_json::to_string(&embedding)?;

                remote_conn
                    .execute(
                        "INSERT OR REPLACE INTO embeddings (id, embedding) VALUES (?1, ?2)",
                        [id.as_str(), embedding_json.as_str()],
                    )
                    .await?;

                stats.embeddings_migrated += 1;
            }
            drop(embed_rows);
            drop(embed_stmt);
        }

        drop(local); // Release lock before finalizing

        stats.migration_time_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            "Migration complete: {} nodes, {} embeddings in {}ms",
            stats.nodes_migrated,
            stats.embeddings_migrated,
            stats.migration_time_ms
        );

        Ok(stats)
    }

    /// Check if local storage is available
    pub fn has_local(&self) -> bool {
        self.local.is_some()
    }

    /// Check if remote storage is available
    pub fn has_remote(&self) -> bool {
        self.remote.is_some()
    }

    /// Get storage mode
    pub fn mode(&self) -> StorageMode {
        match (self.local.is_some(), self.remote.is_some()) {
            (true, false) => StorageMode::LocalOnly,
            (false, true) => StorageMode::RemoteOnly,
            (true, true) => StorageMode::Hybrid,
            (false, false) => StorageMode::None,
        }
    }

    /// Execute a query on local storage
    pub async fn execute_local(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        let Some(local) = &self.local else {
            anyhow::bail!("No local storage available");
        };

        let conn = local.lock().await;
        Ok(conn.execute(sql, params)?)
    }

    /// Execute a query on remote storage
    pub async fn execute_remote(&self, sql: &str, params: impl libsql::params::IntoParams) -> Result<()> {
        let Some(remote) = &self.remote else {
            anyhow::bail!("No remote storage available");
        };

        remote.execute(sql, params).await?;
        Ok(())
    }

    /// Query local storage
    pub async fn query_local(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<serde_json::Value>> {
        let Some(local) = &self.local else {
            anyhow::bail!("No local storage available");
        };

        let conn = local.lock().await;
        let mut stmt = conn.prepare(sql)?;

        // Get column names first to avoid borrow issues
        let column_names: Vec<String> = (0..stmt.column_count())
            .map(|i| stmt.column_name(i).map(|n| n.to_string()))
            .collect::<Result<_, _>>()?;

        let mut results = Vec::new();

        let mut rows = stmt.query(params)?;
        while let Some(row) = rows.next()? {
            let mut obj = serde_json::Map::new();
            for (i, name) in column_names.iter().enumerate() {
                let value: serde_json::Value = serde_json::to_value(row.get::<_, String>(i)?)?;
                obj.insert(name.clone(), value);
            }
            results.push(serde_json::Value::Object(obj));
        }

        Ok(results)
    }

    /// Query remote storage
    pub async fn query_remote(
        &self,
        sql: &str,
        params: impl libsql::params::IntoParams,
    ) -> Result<Vec<serde_json::Value>> {
        let Some(remote) = &self.remote else {
            anyhow::bail!("No remote storage available");
        };

        let mut results = Vec::new();
        let mut rows = remote.query(sql, params).await?;

        // Collect all rows first since we can't get column count from Row
        let mut row_data = Vec::new();
        while let Some(row) = rows.next().await? {
            let mut values = Vec::new();
            // Try to get values from columns 0-N until we get an error
            let mut i = 0;
            loop {
                match row.get::<String>(i) {
                    Ok(val) => values.push(val),
                    Err(_) => break,
                }
                i += 1;
            }
            row_data.push(values);
        }

        for values in row_data {
            let mut obj = serde_json::Map::new();
            for (i, value) in values.iter().enumerate() {
                obj.insert(format!("col_{}", i), serde_json::json!(value));
            }
            results.push(serde_json::Value::Object(obj));
        }

        Ok(results)
    }
}

/// Storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Vector extension not available")]
    VectorExtensionNotAvailable,

    #[error("Local storage error: {0}")]
    LocalStorageError(String),

    #[error("Remote query failed: {0}")]
    RemoteQueryFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turso_config_default() {
        let config = TursoConfig::default();
        assert_eq!(config.database_url, "file:local.db");
        assert!(config.auth_token.is_empty());
        assert!(!config.enable_vectors);
        assert!(!config.remote_only);
    }

    #[test]
    fn test_turso_config_local_only() {
        let config = TursoConfig::local_only();
        assert!(config.is_local_only());
        assert!(!config.is_remote());
    }

    #[test]
    fn test_turso_config_remote_only() {
        let config = TursoConfig::remote_only(
            "libsql://token@db.turso.io".to_string(),
            "auth_token".to_string(),
        );
        assert!(config.is_remote());
        assert!(!config.is_local_only());
        assert!(config.remote_only);
    }

    #[test]
    fn test_turso_config_hybrid() {
        let config = TursoConfig::hybrid(
            "libsql://token@db.turso.io".to_string(),
            "auth_token".to_string(),
        );
        assert!(config.is_remote());
        assert!(!config.is_local_only());
        assert!(!config.remote_only);
    }

    #[test]
    fn test_turso_config_with_vectors() {
        let config = TursoConfig::local_only().with_vectors(true);
        assert!(config.enable_vectors);
    }

    #[test]
    fn test_migration_stats_default() {
        let stats = MigrationStats::default();
        assert_eq!(stats.nodes_migrated, 0);
        assert_eq!(stats.edges_migrated, 0);
        assert_eq!(stats.embeddings_migrated, 0);
        assert_eq!(stats.migration_time_ms, 0);
    }

    #[tokio::test]
    async fn test_hybrid_storage_local_only() -> Result<()> {
        let config = TursoConfig::local_only();
        let storage = HybridStorage::new(config).await?;
        assert!(storage.has_local());
        assert!(!storage.has_remote());
        assert_eq!(storage.mode(), StorageMode::LocalOnly);
        Ok(())
    }
}
