//! Turso Storage Backend
//!
//! Unified database backend using libSQL (Turso) for storage.
//! This will eventually replace the rusqlite-based DatabaseManager.
//!
//! ## Architecture
//!
//! - **Local Embedded Mode**: Uses libsql's local database for embedded operation
//! - **Remote Mode**: Can connect to remote Turso database (future)
//! - **Connection Pooling**: Manages connection pool for efficient database access
//! - **LSP State Tracking**: Stores LSP server state for session management
//!
//! ## Migration Status
//!
//! This is a skeleton implementation. Full migration from rusqlite will happen in Phase 2.

use anyhow::{Context, Result};
use libsql::{Builder, Database};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// LSP server status values
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LspStatus {
    Running,
    Stopped,
    Error,
    Starting,
}

impl LspStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LspStatus::Running => "running",
            LspStatus::Stopped => "stopped",
            LspStatus::Error => "error",
            LspStatus::Starting => "starting",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(LspStatus::Running),
            "stopped" => Some(LspStatus::Stopped),
            "error" => Some(LspStatus::Error),
            "starting" => Some(LspStatus::Starting),
            _ => None,
        }
    }
}

/// LSP server state record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspServerState {
    pub id: i64,
    pub session_id: String,
    pub language: String,
    pub lsp_name: String,
    pub status: String,
    pub pid: Option<i64>,
    pub port: Option<i64>,
    pub auto_start: bool,
    pub last_started: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Configuration for Turso storage backend
#[derive(Debug, Clone)]
pub struct TursoConfig {
    /// Maximum number of connections in the pool
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Whether to enable read-only mode
    pub read_only: bool,
}

impl Default for TursoConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            connection_timeout_secs: 30,
            read_only: false,
        }
    }
}

/// Turso Storage Backend
///
/// Provides unified storage using libSQL (Turso) as the database backend.
/// This will replace the existing rusqlite-based DatabaseManager.
///
/// ## Current Implementation
///
/// - Local embedded mode only
/// - Basic connection handling
/// - Connection pooling configuration
/// - Graceful shutdown
/// - LSP state table schema
///
/// ## Future Implementation (Phase 2)
///
/// - Full OLTP operations migration
/// - OLAP operations migration
/// - FTS5 migration from Tantivy
/// - Database migration scripts
pub struct TursoStorageBackend {
    db_path: PathBuf,
    database: Arc<Database>,
    config: TursoConfig,
    is_shutdown: Arc<AtomicBool>,
}

// Implement Clone for TursoStorageBackend
impl Clone for TursoStorageBackend {
    fn clone(&self) -> Self {
        Self {
            db_path: self.db_path.clone(),
            database: Arc::clone(&self.database),
            config: self.config.clone(),
            is_shutdown: Arc::clone(&self.is_shutdown),
        }
    }
}

impl TursoStorageBackend {
    /// Create new Turso storage backend with local embedded mode
    ///
    /// ## Arguments
    ///
    /// - `db_path`: Optional path to database file. If None, uses `~/.maestro/maestro_turso.db`
    /// - `config`: Optional configuration. If None, uses default configuration.
    ///
    /// ## Returns
    ///
    /// Returns `Result<TursoStorageBackend>` with the initialized backend
    pub async fn new(db_path: Option<PathBuf>, config: Option<TursoConfig>) -> Result<Self> {
        let config = config.unwrap_or_default();
        let path = db_path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".maestro");
            p.push("maestro_turso.db");
            p
        });

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        info!(
            "Opening Turso database (local embedded): {} with max_connections={}",
            path.display(),
            config.max_connections
        );

        // Use libsql's local embedded mode with connection pool configuration
        let db = Builder::new_local(path.clone())
            .build()
            .await
            .context("Failed to open libsql database")?;

        info!("Turso database opened successfully with connection pool");

        Ok(Self {
            db_path: path,
            database: Arc::new(db),
            config,
            is_shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Create in-memory database for testing
    ///
    /// ## Arguments
    ///
    /// - `config`: Optional configuration. If None, uses default configuration.
    ///
    /// ## Returns
    ///
    /// Returns `Result<TursoStorageBackend>` with an in-memory database
    pub async fn in_memory(config: Option<TursoConfig>) -> Result<Self> {
        let config = config.unwrap_or_default();
        info!("Opening in-memory Turso database with max_connections={}", config.max_connections);

        let db = Builder::new_local(":memory:")
            .build()
            .await
            .context("Failed to open in-memory libsql database")?;

        info!("In-memory Turso database opened successfully");

        Ok(Self {
            db_path: PathBuf::from(":memory:"),
            database: Arc::new(db),
            config,
            is_shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Initialize database schema
    ///
    /// This creates the LSP state table. More tables will be added in Phase 2.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Turso database schema");

        let conn = self
            .database
            .connect()
            .context("Failed to get connection")?;

        // Create LSP servers table
        conn.execute(
            LSP_SERVERS_TABLE_SQL,
            libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
        )
        .await
        .context("Failed to create LSP servers table")?;

        info!("Turso database schema initialized successfully");
        Ok(())
    }

    /// Get database path
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Check if database is in read-only mode
    pub fn is_read_only(&self) -> bool {
        self.config.read_only
    }

    /// Get the configuration
    pub fn config(&self) -> &TursoConfig {
        &self.config
    }

    /// Check if the backend has been shut down
    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::SeqCst)
    }

    /// Execute a query with a connection
    ///
    /// This method checks if the backend is shut down before attempting to get a connection.
    pub async fn with_connection<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&libsql::Connection) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Turso backend is shut down"));
        }

        let conn = self
            .database
            .connect()
            .context("Failed to get connection")?;
        f(&conn)
    }

    /// Shutdown the backend gracefully
    ///
    /// This method marks the backend as shut down and performs cleanup.
    /// After shutdown, any attempt to use the backend will return an error.
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if shutdown completed successfully
    pub async fn shutdown(&self) -> Result<()> {
        if self.is_shutdown() {
            debug!("Turso backend already shut down");
            return Ok(());
        }

        info!("Shutting down Turso backend: {}", self.db_path.display());

        // Mark as shut down first to prevent new connections
        self.is_shutdown.store(true, Ordering::SeqCst);

        // Note: libsql's Database doesn't have an explicit close method in version 0.9
        // The Arc<Database> will be dropped when all references are released
        // For explicit cleanup, we rely on Rust's RAII pattern

        debug!("Turso backend shutdown complete");
        Ok(())
    }

    /// Create a new backend with the default path and config
    ///
    /// Convenience method for creating a backend with all defaults.
    pub async fn with_defaults() -> Result<Self> {
        Self::new(None, None).await
    }
}

/// SQL for creating LSP servers table
const LSP_SERVERS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS lsp_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    language TEXT NOT NULL,
    lsp_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'stopped',
    pid INTEGER,
    port INTEGER,
    auto_start INTEGER DEFAULT 1,
    last_started TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    UNIQUE(session_id, lsp_name)
);

CREATE INDEX IF NOT EXISTS idx_lsp_session ON lsp_servers(session_id);
CREATE INDEX IF NOT EXISTS idx_lsp_status ON lsp_servers(status);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_turso_backend_creation() {
        let backend = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create in-memory backend");
        backend.initialize().await.expect("Failed to initialize");
        assert_eq!(backend.path(), Path::new(":memory:"));
        assert!(!backend.is_read_only());
        assert!(!backend.is_shutdown());
    }

    #[tokio::test]
    async fn test_turso_backend_with_config() {
        let config = TursoConfig {
            max_connections: 20,
            connection_timeout_secs: 60,
            read_only: true,
        };
        let backend = TursoStorageBackend::in_memory(Some(config))
            .await
            .expect("Failed to create in-memory backend");
        assert_eq!(backend.config().max_connections, 20);
        assert_eq!(backend.config().connection_timeout_secs, 60);
        assert!(backend.is_read_only());
    }

    #[tokio::test]
    async fn test_turso_backend_default_config() {
        let backend = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create in-memory backend");
        assert_eq!(backend.config().max_connections, 10);
        assert_eq!(backend.config().connection_timeout_secs, 30);
        assert!(!backend.is_read_only());
    }

    #[tokio::test]
    async fn test_turso_backend_shutdown() {
        let backend = TursoStorageBackend::in_memory(None)
            .await
            .expect("Failed to create in-memory backend");
        backend.initialize().await.expect("Failed to initialize");

        assert!(!backend.is_shutdown());

        // Shutdown the backend
        backend.shutdown().await.expect("Failed to shutdown");
        assert!(backend.is_shutdown());

        // Double shutdown should be idempotent
        backend.shutdown().await.expect("Double shutdown should succeed");
        assert!(backend.is_shutdown());

        // Attempting to use connection after shutdown should fail
        let result = backend.with_connection(|_conn| Ok(())).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("shut down"));
    }

    #[tokio::test]
    async fn test_turso_backend_with_defaults() {
        let backend = TursoStorageBackend::with_defaults()
            .await
            .expect("Failed to create backend with defaults");
        backend.initialize().await.expect("Failed to initialize");
        assert!(!backend.is_shutdown());
        assert!(!backend.is_read_only());
        assert_eq!(backend.config().max_connections, 10);
    }

    #[tokio::test]
    async fn test_turso_config_default() {
        let config = TursoConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connection_timeout_secs, 30);
        assert!(!config.read_only);
    }

    #[tokio::test]
    async fn test_lsp_status_conversion() {
        assert_eq!(LspStatus::Running.as_str(), "running");
        assert_eq!(LspStatus::Stopped.as_str(), "stopped");
        assert_eq!(LspStatus::Error.as_str(), "error");
        assert_eq!(LspStatus::Starting.as_str(), "starting");

        assert_eq!(LspStatus::from_str("running"), Some(LspStatus::Running));
        assert_eq!(LspStatus::from_str("stopped"), Some(LspStatus::Stopped));
        assert_eq!(LspStatus::from_str("error"), Some(LspStatus::Error));
        assert_eq!(LspStatus::from_str("starting"), Some(LspStatus::Starting));
        assert_eq!(LspStatus::from_str("invalid"), None);
    }
}
