//! Turso Storage Backend
//!
//! Unified database backend using libSQL (Turso) for storage.
//! This will eventually replace the rusqlite-based DatabaseManager.
//!
//! ## Architecture
//!
//! - **Local Embedded Mode**: Uses libsql's local database for embedded operation
//! - **Remote Mode**: Can connect to remote Turso database (future)
//! - **LSP State Tracking**: Stores LSP server state for session management
//!
//! ## Migration Status
//!
//! This is a skeleton implementation. Full migration from rusqlite will happen in Phase 2.

use anyhow::{Context, Result};
use libsql::{Builder, Database};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

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

/// Turso Storage Backend
///
/// Provides unified storage using libSQL (Turso) as the database backend.
/// This will replace the existing rusqlite-based DatabaseManager.
///
/// ## Current Implementation
///
/// - Local embedded mode only
/// - Basic connection handling
/// - LSP state table schema
///
/// ## Future Implementation (Phase 2)
///
/// - Full OLTP operations migration
/// - OLAP operations migration
/// - FTS5 migration from Tantivy
/// - Database migration scripts
#[derive(Clone)]
pub struct TursoStorageBackend {
    db_path: PathBuf,
    #[allow(dead_code)]
    database: Arc<Database>,
    read_only: bool,
}

impl TursoStorageBackend {
    /// Create new Turso storage backend with local embedded mode
    ///
    /// ## Arguments
    ///
    /// - `db_path`: Optional path to database file. If None, uses `~/.maestro/maestro_turso.db`
    ///
    /// ## Returns
    ///
    /// Returns `Result<TursoStorageBackend>` with the initialized backend
    pub async fn new(db_path: Option<PathBuf>) -> Result<Self> {
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

        info!("Opening Turso database (local embedded): {}", path.display());

        // Use libsql's local embedded mode
        let db = Builder::new_local(path.clone())
            .build()
            .await
            .context("Failed to open libsql database")?;

        info!("Turso database opened successfully");

        Ok(Self {
            db_path: path,
            database: Arc::new(db),
            read_only: false,
        })
    }

    /// Create in-memory database for testing
    ///
    /// ## Returns
    ///
    /// Returns `Result<TursoStorageBackend>` with an in-memory database
    pub async fn in_memory() -> Result<Self> {
        info!("Opening in-memory Turso database");

        let db = Builder::new_local(":memory:")
            .build()
            .await
            .context("Failed to open in-memory libsql database")?;

        info!("In-memory Turso database opened successfully");

        Ok(Self {
            db_path: PathBuf::from(":memory:"),
            database: Arc::new(db),
            read_only: false,
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
        self.read_only
    }

    /// Execute a query with a connection
    pub async fn with_connection<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&libsql::Connection) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self
            .database
            .connect()
            .context("Failed to get connection")?;
        f(&conn)
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
        let backend = TursoStorageBackend::in_memory()
            .await
            .expect("Failed to create in-memory backend");
        backend.initialize().await.expect("Failed to initialize");
        assert_eq!(backend.path(), Path::new(":memory:"));
        assert!(!backend.is_read_only());
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
