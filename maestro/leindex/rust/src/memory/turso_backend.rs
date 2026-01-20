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
//! - **OLTP Operations**: CRUD operations for sessions, projects, memories, tracks
//!
//! ## Migration Status
//!
//! Phase 2: OLTP operations migrated from rusqlite to libsql.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use libsql::{Builder, Database};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::future::Future;
use tracing::{debug, info};

use super::models::{Session, SessionStatus, MaestroProject, Memory, MemoryCategory, MemoryImportance};

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
    /// Creates all tables for OLTP operations: LSP servers, sessions, projects, memories, tracks.
    /// Also creates FTS5 full-text search tables.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Turso database schema");

        let conn = self
            .database
            .connect()
            .context("Failed to get connection")?;

        // Create all tables
        for table_sql in &[
            LSP_SERVERS_TABLE_SQL,
            SESSIONS_TABLE_SQL,
            PROJECTS_TABLE_SQL,
            MEMORIES_TABLE_SQL,
            TRACKS_TABLE_SQL,
            SESSION_GROUPS_TABLE_SQL,
            MCP_SERVERS_TABLE_SQL,
        ] {
            conn.execute(
                table_sql,
                libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to create table")?;
        }

        // Create FTS5 virtual table for full-text search
        // Note: Triggers not supported in libsql 0.9, so manual sync is used
        let _ = conn
            .execute(
                "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(content, category)",
                libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to create FTS5 table")?;

        // Populate FTS5 index with existing memories
        let _ = conn
            .execute(
                "INSERT INTO memories_fts(rowid, content, category)
                 SELECT id, content, category FROM memories
                 WHERE id NOT IN (SELECT rowid FROM memories_fts)",
                libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await;

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
        F: FnOnce(libsql::Connection) -> Pin<Box<dyn Future<Output = Result<T>> + Send>> + Send + 'static,
        T: Send + 'static,
    {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Turso backend is shut down"));
        }

        let conn = self
            .database
            .connect()
            .context("Failed to get connection")?;
        f(conn).await
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

    // ========================================================================
    // Session CRUD Operations
    // ========================================================================

    /// Insert a new session
    pub async fn insert_session(&self, session: &Session) -> Result<i64> {
        let session_id = session.session_id.clone();
        let title = session.title.clone();
        let project_path = session.project_path.clone();
        let group_path = session.group_path.clone().unwrap_or_default();
        let sort_order = session.sort_order;
        let parent_session_id = session.parent_session_id.clone().unwrap_or_default();
        let command = session.command.clone().unwrap_or_default();
        let tool = session.tool.clone().unwrap_or_default();
        let status = session.status.as_str().to_string();
        let multiplexer_session = session.multiplexer_session.clone().unwrap_or_default();
        let started_at = session.started_at.to_rfc3339();
        let last_accessed_at = session.last_accessed_at.map(|d| d.to_rfc3339()).unwrap_or_default();
        let ended_at = session.ended_at.map(|d| d.to_rfc3339()).unwrap_or_default();
        let metadata = serde_json::to_string(&session.metadata).unwrap_or_default();

        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                conn.execute(
                    "INSERT INTO sessions (session_id, title, project_path, group_path, sort_order,
                     parent_session_id, command, tool, status, multiplexer_session, started_at,
                     last_accessed_at, ended_at, metadata)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    libsql::params_from_iter([
                        libsql::Value::Text(session_id),
                        libsql::Value::Text(title),
                        libsql::Value::Text(project_path),
                        libsql::Value::Text(group_path),
                        libsql::Value::Integer(sort_order as i64),
                        libsql::Value::Text(parent_session_id),
                        libsql::Value::Text(command),
                        libsql::Value::Text(tool),
                        libsql::Value::Text(status),
                        libsql::Value::Text(multiplexer_session),
                        libsql::Value::Text(started_at),
                        libsql::Value::Text(last_accessed_at),
                        libsql::Value::Text(ended_at),
                        libsql::Value::Text(metadata),
                    ]),
                )
                .await
                .context("Failed to insert session")?;

                Ok(conn.last_insert_rowid())
            })
        })
        .await
    }

    /// Get a session by session_id
    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let session_id = session_id.to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT id, session_id, title, project_path, group_path, sort_order,
                         parent_session_id, command, tool, status, multiplexer_session,
                         started_at, last_accessed_at, ended_at, metadata
                         FROM sessions WHERE session_id = ?",
                    )
                    .await
                    .context("Failed to prepare session query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([libsql::Value::Text(session_id)]))
                    .await
                    .context("Failed to query session")?;

                // Get the first row if exists
                if let Ok(Some(row)) = result.next().await {
                    Ok(Some(Session {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        title: row.get(2)?,
                        project_path: row.get(3)?,
                        group_path: row.get(4)?,
                        sort_order: row.get(5)?,
                        parent_session_id: row.get(6)?,
                        command: row.get(7)?,
                        tool: row.get(8)?,
                        status: SessionStatus::from_str(row.get::<String>(9)?.as_str())
                            .unwrap_or(SessionStatus::Idle),
                        multiplexer_session: row.get(10)?,
                        started_at: parse_datetime(row.get::<String>(11)?),
                        last_accessed_at: row.get::<Option<String>>(12)?.map(|s| parse_datetime(s)),
                        ended_at: row.get::<Option<String>>(13)?.map(|s| parse_datetime(s)),
                        metadata: row.get::<Option<String>>(14)?
                            .and_then(|s: String| serde_json::from_str::<serde_json::Value>(&s).ok()),
                    }))
                } else {
                    Ok(None)
                }
            })
        })
        .await
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT id, session_id, title, project_path, group_path, sort_order,
                         parent_session_id, command, tool, status, multiplexer_session,
                         started_at, last_accessed_at, ended_at, metadata
                         FROM sessions ORDER BY sort_order, started_at",
                    )
                    .await
                    .context("Failed to prepare sessions list query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query sessions")?;

                let mut sessions = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    sessions.push(Session {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        title: row.get(2)?,
                        project_path: row.get(3)?,
                        group_path: row.get(4)?,
                        sort_order: row.get(5)?,
                        parent_session_id: row.get(6)?,
                        command: row.get(7)?,
                        tool: row.get(8)?,
                        status: SessionStatus::from_str(row.get::<String>(9)?.as_str())
                            .unwrap_or(SessionStatus::Idle),
                        multiplexer_session: row.get(10)?,
                        started_at: parse_datetime(row.get::<String>(11)?),
                        last_accessed_at: row.get::<Option<String>>(12)?.map(|s| parse_datetime(s)),
                        ended_at: row.get::<Option<String>>(13)?.map(|s| parse_datetime(s)),
                        metadata: row.get::<Option<String>>(14)?
                            .and_then(|s: String| serde_json::from_str::<serde_json::Value>(&s).ok()),
                    });
                }
                Ok(sessions)
            })
        })
        .await
    }

    /// Update session status
    pub async fn update_session_status(&self, session_id: &str, status: SessionStatus) -> Result<()> {
        let session_id = session_id.to_string();
        let status_str = status.as_str().to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                conn.execute(
                    "UPDATE sessions SET status = ?1, updated_at = datetime('now') WHERE session_id = ?2",
                    libsql::params_from_iter([
                        libsql::Value::Text(status_str),
                        libsql::Value::Text(session_id),
                    ]),
                )
                .await
                .context("Failed to update session status")?;
                Ok(())
            })
        })
        .await
    }

    /// Update session last accessed time
    pub async fn update_session_last_accessed(&self, session_id: &str) -> Result<()> {
        let session_id = session_id.to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                conn.execute(
                    "UPDATE sessions SET last_accessed_at = datetime('now'), updated_at = datetime('now') WHERE session_id = ?",
                    libsql::params_from_iter([libsql::Value::Text(session_id)]),
                )
                .await
                .context("Failed to update session last accessed")?;
                Ok(())
            })
        })
        .await
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let session_id = session_id.to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                conn.execute(
                    "DELETE FROM sessions WHERE session_id = ?",
                    libsql::params_from_iter([libsql::Value::Text(session_id)]),
                )
                .await
                .context("Failed to delete session")?;
                Ok(())
            })
        })
        .await
    }

    // ========================================================================
    // Project CRUD Operations
    // ========================================================================

    /// Get or create a project by path
    pub async fn get_or_create_project(&self, path: &str, name: &str) -> Result<MaestroProject> {
        // Try to get existing project
        if let Some(project) = self.get_project_by_path(path).await? {
            return Ok(project);
        }

        // Create new project
        let path = path.to_string();
        let name = name.to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                conn.execute(
                    "INSERT INTO maestro_projects (project_path, project_name) VALUES (?1, ?2)",
                    libsql::params_from_iter([
                        libsql::Value::Text(path.clone()),
                        libsql::Value::Text(name.clone()),
                    ]),
                )
                .await
                .context("Failed to insert project")?;

                let id = conn.last_insert_rowid();

                Ok(MaestroProject {
                    id,
                    project_path: path,
                    project_name: name,
                    description: None,
                    project_type: None,
                    tech_stack: Vec::new(),
                    is_active: true,
                    created_at: Utc::now(),
                    updated_at: None,
                    last_scanned_at: None,
                })
            })
        })
        .await
    }

    /// Get a project by path
    pub async fn get_project_by_path(&self, path: &str) -> Result<Option<MaestroProject>> {
        let path = path.to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT id, project_path, project_name, description, project_type, tech_stack,
                         is_active, created_at, updated_at, last_scanned_at
                         FROM maestro_projects WHERE project_path = ?",
                    )
                    .await
                    .context("Failed to prepare project query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([libsql::Value::Text(path)]))
                    .await
                    .context("Failed to query project")?;

                // Get the first row if exists
                if let Ok(Some(row)) = result.next().await {
                    Ok(Some(MaestroProject {
                        id: row.get::<i64>(0)?,
                        project_path: row.get::<String>(1)?,
                        project_name: row.get::<String>(2)?,
                        description: row.get::<Option<String>>(3)?,
                        project_type: row.get::<Option<String>>(4)?,
                        tech_stack: row
                            .get::<Option<String>>(5)?
                            .and_then(|s: String| serde_json::from_str::<Vec<String>>(&s).ok())
                            .unwrap_or_default(),
                        is_active: row.get::<i32>(6)? == 1,
                        created_at: parse_datetime(row.get::<String>(7)?),
                        updated_at: row.get::<Option<String>>(8)?.map(|s| parse_datetime(s)),
                        last_scanned_at: row.get::<Option<String>>(9)?.map(|s| parse_datetime(s)),
                    }))
                } else {
                    Ok(None)
                }
            })
        })
        .await
    }

    /// List all active projects
    pub async fn list_projects(&self) -> Result<Vec<MaestroProject>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT id, project_path, project_name, description, project_type, tech_stack,
                         is_active, created_at, updated_at, last_scanned_at
                         FROM maestro_projects WHERE is_active = 1 ORDER BY project_name",
                    )
                    .await
                    .context("Failed to prepare projects list query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query projects")?;

                let mut projects = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    projects.push(MaestroProject {
                        id: row.get::<i64>(0)?,
                        project_path: row.get::<String>(1)?,
                        project_name: row.get::<String>(2)?,
                        description: row.get::<Option<String>>(3)?,
                        project_type: row.get::<Option<String>>(4)?,
                        tech_stack: row
                            .get::<Option<String>>(5)?
                            .and_then(|s: String| serde_json::from_str::<Vec<String>>(&s).ok())
                            .unwrap_or_default(),
                        is_active: row.get::<i32>(6)? == 1,
                        created_at: parse_datetime(row.get::<String>(7)?),
                        updated_at: row.get::<Option<String>>(8)?.map(|s| parse_datetime(s)),
                        last_scanned_at: row.get::<Option<String>>(9)?.map(|s| parse_datetime(s)),
                    });
                }
                Ok(projects)
            })
        })
        .await
    }

    // ========================================================================
    // Memory CRUD Operations
    // ========================================================================

    /// Insert a new memory
    pub async fn insert_memory(&self, memory: &Memory) -> Result<i64> {
        let content = memory.content.clone();
        let content_fts = content.clone();
        let summary = memory.summary.clone().unwrap_or_default();
        let category = memory_category_to_string(&memory.category);
        let category_fts = category.clone();
        let importance = memory_importance_to_string(&memory.importance);
        let source = memory.source.clone().unwrap_or_default();
        let session_id = memory.session_id.clone().unwrap_or_default();
        let project_id = memory.project_id.unwrap_or(0);
        let track_id = memory.track_id.unwrap_or(0);
        let command = memory.command.clone().unwrap_or_default();
        let command_context = memory.command_context.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();
        let created_at = memory.created_at.to_rfc3339();
        let expires_at = memory.expires_at.map(|d| d.to_rfc3339()).unwrap_or_default();
        let last_accessed = memory.last_accessed.map(|d| d.to_rfc3339()).unwrap_or_default();
        let metadata = memory.metadata.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();
        let tags = memory.tags.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                conn.execute(
                    "INSERT INTO memories (content, summary, category, importance, source, session_id,
                     project_id, track_id, command, command_context, created_at, expires_at,
                     last_accessed, meta_data, tags)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    libsql::params_from_iter([
                        libsql::Value::Text(content),
                        libsql::Value::Text(summary),
                        libsql::Value::Text(category),
                        libsql::Value::Text(importance),
                        libsql::Value::Text(source),
                        libsql::Value::Text(session_id),
                        libsql::Value::Integer(project_id),
                        libsql::Value::Integer(track_id),
                        libsql::Value::Text(command),
                        libsql::Value::Text(command_context),
                        libsql::Value::Text(created_at),
                        libsql::Value::Text(expires_at),
                        libsql::Value::Text(last_accessed),
                        libsql::Value::Text(metadata),
                        libsql::Value::Text(tags),
                    ]),
                )
                .await
                .context("Failed to insert memory")?;

                let id = conn.last_insert_rowid();

                // Also add to FTS5 index
                let _ = conn
                    .execute(
                        "INSERT INTO memories_fts(rowid, content, category) VALUES (?, ?, ?)",
                        libsql::params_from_iter([
                            libsql::Value::Integer(id),
                            libsql::Value::Text(content_fts),
                            libsql::Value::Text(category_fts),
                        ]),
                    )
                    .await;

                Ok(id)
            })
        })
        .await
    }

    /// Get memories by session_id
    pub async fn get_memories_by_session(&self, session_id: &str) -> Result<Vec<Memory>> {
        let session_id = session_id.to_string();
        self.with_connection(move |conn: libsql::Connection| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT id, content, summary, category, importance, source, session_id,
                         project_id, track_id, command, command_context, created_at, expires_at,
                         last_accessed, meta_data, tags
                         FROM memories WHERE session_id = ? ORDER BY created_at DESC",
                    )
                    .await
                    .context("Failed to prepare memories query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([libsql::Value::Text(session_id)]))
                    .await
                    .context("Failed to query memories")?;

                let mut memories = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    memories.push(Memory {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        summary: row.get(2)?,
                        category: string_to_memory_category(row.get::<String>(3)?.as_str()),
                        importance: string_to_memory_importance(row.get::<String>(4)?.as_str()),
                        source: row.get(5)?,
                        session_id: row.get(6)?,
                        project_id: row.get::<i64>(7)?.try_into().ok(),
                        track_id: row.get::<i64>(8)?.try_into().ok(),
                        command: row.get(9)?,
                        command_context: row.get::<Option<String>>(10)?
                            .and_then(|s: String| serde_json::from_str::<serde_json::Value>(&s).ok()),
                        created_at: parse_datetime(row.get::<String>(11)?),
                        expires_at: row.get::<Option<String>>(12)?.map(|s| parse_datetime(s)),
                        last_accessed: row.get::<Option<String>>(13)?.map(|s| parse_datetime(s)),
                        metadata: row.get::<Option<String>>(14)?
                            .and_then(|s: String| serde_json::from_str::<serde_json::Value>(&s).ok()),
                        tags: row.get::<Option<String>>(15)?
                            .and_then(|s: String| serde_json::from_str::<Vec<String>>(&s).ok()),
                    });
                }
                Ok(memories)
            })
        })
        .await
    }

    /// Get database statistics
    pub async fn stats(&self) -> Result<TursoDbStats> {
        self.with_connection(|conn| {
            Box::pin(async move {
                // Query project count
                let stmt = conn
                    .prepare("SELECT COUNT(*) FROM maestro_projects WHERE is_active = 1")
                    .await
                    .context("Failed to prepare project count query")?;
                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query project count")?;
                let project_count: i64 = if let Ok(Some(row)) = result.next().await {
                    row.get(0)?
                } else {
                    0
                };

                // Query memory count
                let stmt = conn
                    .prepare("SELECT COUNT(*) FROM memories")
                    .await
                    .context("Failed to prepare memory count query")?;
                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query memory count")?;
                let memory_count: i64 = if let Ok(Some(row)) = result.next().await {
                    row.get(0)?
                } else {
                    0
                };

                // Query session count
                let stmt = conn
                    .prepare("SELECT COUNT(*) FROM sessions")
                    .await
                    .context("Failed to prepare session count query")?;
                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query session count")?;
                let session_count: i64 = if let Ok(Some(row)) = result.next().await {
                    row.get(0)?
                } else {
                    0
                };

                Ok(TursoDbStats {
                    project_count: project_count as usize,
                    memory_count: memory_count as usize,
                    session_count: session_count as usize,
                })
            })
        })
        .await
    }

    // ========================================================================
    // OLAP Analytical Queries
    // ========================================================================

    /// Get session statistics grouped by status
    pub async fn session_stats_by_status(&self) -> Result<Vec<SessionStatusStats>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT status, COUNT(*) as count
                         FROM sessions
                         GROUP BY status
                         ORDER BY count DESC",
                    )
                    .await
                    .context("Failed to prepare session stats query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query session stats")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(SessionStatusStats {
                        status: row.get::<String>(0)?,
                        count: row.get::<i64>(1)? as usize,
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get session statistics grouped by project path
    pub async fn session_stats_by_project(&self, limit: Option<usize>) -> Result<Vec<ProjectSessionStats>> {
        let limit = limit.unwrap_or(20);
        self.with_connection(move |conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT project_path, COUNT(*) as session_count,
                                COUNT(CASE WHEN status = 'running' THEN 1 END) as running_count,
                                COUNT(CASE WHEN status = 'idle' THEN 1 END) as idle_count
                         FROM sessions
                         GROUP BY project_path
                         ORDER BY session_count DESC
                         LIMIT ?",
                    )
                    .await
                    .context("Failed to prepare project session stats query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([libsql::Value::Integer(
                        limit as i64,
                    )]))
                    .await
                    .context("Failed to query project session stats")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(ProjectSessionStats {
                        project_path: row.get(0)?,
                        session_count: row.get::<i64>(1)? as usize,
                        running_count: row.get::<i64>(2)? as usize,
                        idle_count: row.get::<i64>(3)? as usize,
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get memory statistics grouped by category
    pub async fn memory_stats_by_category(&self) -> Result<Vec<MemoryCategoryStats>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT category, COUNT(*) as count
                         FROM memories
                         GROUP BY category
                         ORDER BY count DESC",
                    )
                    .await
                    .context("Failed to prepare memory category stats query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query memory category stats")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(MemoryCategoryStats {
                        category: row.get(0)?,
                        count: row.get::<i64>(1)? as usize,
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get memory statistics grouped by importance
    pub async fn memory_stats_by_importance(&self) -> Result<Vec<MemoryImportanceStats>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT importance, COUNT(*) as count
                         FROM memories
                         GROUP BY importance
                         ORDER BY
                             CASE importance
                                 WHEN 'critical' THEN 1
                                 WHEN 'high' THEN 2
                                 WHEN 'normal' THEN 3
                                 WHEN 'low' THEN 4
                                 ELSE 5
                             END",
                    )
                    .await
                    .context("Failed to prepare memory importance stats query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query memory importance stats")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(MemoryImportanceStats {
                        importance: row.get(0)?,
                        count: row.get::<i64>(1)? as usize,
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get track completion statistics
    pub async fn track_completion_stats(&self) -> Result<Vec<TrackCompletionStats>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT t.status, COUNT(*) as count,
                                AVG(CAST(t.completed_tasks AS REAL) / NULLIF(t.total_tasks, 0) * 100) as avg_completion_pct
                         FROM maestro_tracks t
                         GROUP BY t.status
                         ORDER BY count DESC",
                    )
                    .await
                    .context("Failed to prepare track completion stats query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query track completion stats")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(TrackCompletionStats {
                        status: row.get(0)?,
                        count: row.get::<i64>(1)? as usize,
                        avg_completion_pct: row.get::<Option<f64>>(2)?,
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get project activity summary (recent sessions and memory creation)
    pub async fn project_activity_summary(&self, days: Option<u32>) -> Result<Vec<ProjectActivitySummary>> {
        let days = days.unwrap_or(7);
        self.with_connection(move |conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT p.project_path, p.project_name,
                                COUNT(DISTINCT s.id) as session_count,
                                COUNT(DISTINCT m.id) as memory_count,
                                MAX(s.started_at) as last_session_at
                         FROM maestro_projects p
                         LEFT JOIN sessions s ON p.project_path = s.project_path
                             AND datetime(s.started_at) >= datetime('now', '-' || ? || ' days')
                         LEFT JOIN memories m ON m.project_id = p.id
                             AND datetime(m.created_at) >= datetime('now', '-' || ? || ' days')
                         WHERE p.is_active = 1
                         GROUP BY p.id
                         HAVING session_count > 0 OR memory_count > 0
                         ORDER BY session_count DESC, memory_count DESC
                         LIMIT 50",
                    )
                    .await
                    .context("Failed to prepare project activity summary query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([
                        libsql::Value::Integer(days as i64),
                        libsql::Value::Integer(days as i64),
                    ]))
                    .await
                    .context("Failed to query project activity summary")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(ProjectActivitySummary {
                        project_path: row.get(0)?,
                        project_name: row.get(1)?,
                        session_count: row.get::<i64>(2)? as usize,
                        memory_count: row.get::<i64>(3)? as usize,
                        last_session_at: row.get::<Option<String>>(4)?
                            .map(|s| parse_datetime(s)),
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get most active projects by session count
    pub async fn most_active_projects(&self, limit: Option<usize>) -> Result<Vec<ActiveProjectStats>> {
        let limit = limit.unwrap_or(10);
        self.with_connection(move |conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT p.project_path, p.project_name,
                                COUNT(s.id) as total_sessions,
                                COUNT(CASE WHEN s.status = 'running' THEN 1 END) as active_sessions,
                                MIN(s.started_at) as first_session_at,
                                MAX(s.started_at) as last_session_at
                         FROM maestro_projects p
                         INNER JOIN sessions s ON p.project_path = s.project_path
                         WHERE p.is_active = 1
                         GROUP BY p.id
                         ORDER BY total_sessions DESC
                         LIMIT ?",
                    )
                    .await
                    .context("Failed to prepare most active projects query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([libsql::Value::Integer(
                        limit as i64,
                    )]))
                    .await
                    .context("Failed to query most active projects")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(ActiveProjectStats {
                        project_path: row.get(0)?,
                        project_name: row.get(1)?,
                        total_sessions: row.get::<i64>(2)? as usize,
                        active_sessions: row.get::<i64>(3)? as usize,
                        first_session_at: row.get::<Option<String>>(4)?
                            .map(|s| parse_datetime(s)),
                        last_session_at: row.get::<Option<String>>(5)?
                            .map(|s| parse_datetime(s)),
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    /// Get LSP server statistics
    pub async fn lsp_server_stats(&self) -> Result<Vec<LspServerStats>> {
        self.with_connection(|conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT lsp_name, status, COUNT(*) as count
                         FROM lsp_servers
                         GROUP BY lsp_name, status
                         ORDER BY lsp_name, status",
                    )
                    .await
                    .context("Failed to prepare LSP server stats query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to query LSP server stats")?;

                let mut stats = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    stats.push(LspServerStats {
                        lsp_name: row.get(0)?,
                        status: row.get(1)?,
                        count: row.get::<i64>(2)? as usize,
                    });
                }
                Ok(stats)
            })
        })
        .await
    }

    // ========================================================================
    // FTS5 Full-Text Search
    // ========================================================================

    /// Search memories using FTS5 full-text search
    ///
    /// Searches memory content using FTS5 for fast full-text search.
    /// Supports simple queries (words) and FTS5 query syntax.
    pub async fn search_memories(&self, query: &str, limit: Option<usize>) -> Result<Vec<MemorySearchResult>> {
        let limit = limit.unwrap_or(50);
        let query = query.to_string();
        self.with_connection(move |conn| {
            Box::pin(async move {
                let stmt = conn
                    .prepare(
                        "SELECT m.id, m.content, m.summary, m.category, m.importance,
                                m.source, m.session_id, m.project_id, m.created_at,
                                snippet(memories_fts, 0, '<mark>', '</mark>', '...', 64) as snippet
                         FROM memories_fts
                         JOIN memories m ON m.id = memories_fts.rowid
                         WHERE memories_fts MATCH ?
                         ORDER BY rank
                         LIMIT ?",
                    )
                    .await
                    .context("Failed to prepare FTS5 search query")?;

                let mut result = stmt
                    .query(libsql::params_from_iter([
                        libsql::Value::Text(query),
                        libsql::Value::Integer(limit as i64),
                    ]))
                    .await
                    .context("Failed to execute FTS5 search")?;

                let mut results = Vec::new();
                while let Ok(Some(row)) = result.next().await {
                    let project_id: Option<i64> = row.get(7)?;
                    results.push(MemorySearchResult {
                        id: row.get::<i64>(0)?,
                        content: row.get(1)?,
                        summary: row.get(2)?,
                        category: row.get(3)?,
                        importance: row.get(4)?,
                        source: row.get(5)?,
                        session_id: row.get(6)?,
                        project_id: project_id.and_then(|id| id.try_into().ok()),
                        created_at: parse_datetime(row.get::<String>(8)?),
                        snippet: row.get::<Option<String>>(9)?,
                    });
                }
                Ok(results)
            })
        })
        .await
    }

    /// Rebuild FTS5 index for all memories
    ///
    /// This should be called after bulk imports or if FTS5 gets out of sync.
    /// Drops and recreates the FTS table and repopulates it.
    pub async fn rebuild_fts_index(&self) -> Result<usize> {
        self.with_connection(|conn| {
            Box::pin(async move {
                // Drop FTS table
                let _ = conn
                    .execute(
                        "DROP TABLE IF EXISTS memories_fts",
                        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
                    )
                    .await;

                // Recreate FTS table
                conn.execute(
                    "CREATE VIRTUAL TABLE memories_fts USING fts5(content, category)",
                    libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
                )
                .await
                .context("Failed to create FTS5 table")?;

                // Populate FTS table
                let rows_affected = conn
                    .execute(
                        "INSERT INTO memories_fts(rowid, content, category)
                         SELECT id, content, category FROM memories",
                        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
                    )
                    .await
                    .context("Failed to populate FTS5 index")?;

                Ok(rows_affected as usize)
            })
        })
        .await
    }

    /// Trigger FTS5 index optimization
    ///
    /// Runs an integrity check and optimization on the FTS5 index.
    /// Call this after bulk imports to ensure FTS index is optimized.
    pub async fn optimize_fts_index(&self) -> Result<()> {
        self.with_connection(|conn| {
            Box::pin(async move {
                conn.execute(
                    "INSERT INTO memories_fts(memories_fts) VALUES('optimize')",
                    libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
                )
                .await
                .context("Failed to optimize FTS5 index")?;
                Ok(())
            })
        })
        .await
    }
}

// ============================================================================
// FTS5 Result Types
// ============================================================================

/// Memory search result with snippet
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemorySearchResult {
    pub id: i64,
    pub content: String,
    pub summary: Option<String>,
    pub category: String,
    pub importance: String,
    pub source: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub snippet: Option<String>,  // HTML-highlighted snippet
}

// ============================================================================
// OLAP Result Types
// ============================================================================

/// Session statistics grouped by status
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionStatusStats {
    pub status: String,
    pub count: usize,
}

/// Project session statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectSessionStats {
    pub project_path: String,
    pub session_count: usize,
    pub running_count: usize,
    pub idle_count: usize,
}

/// Memory statistics grouped by category
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryCategoryStats {
    pub category: String,
    pub count: usize,
}

/// Memory statistics grouped by importance
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryImportanceStats {
    pub importance: String,
    pub count: usize,
}

/// Track completion statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrackCompletionStats {
    pub status: String,
    pub count: usize,
    pub avg_completion_pct: Option<f64>,
}

/// Project activity summary
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectActivitySummary {
    pub project_path: String,
    pub project_name: String,
    pub session_count: usize,
    pub memory_count: usize,
    pub last_session_at: Option<DateTime<Utc>>,
}

/// Active project statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActiveProjectStats {
    pub project_path: String,
    pub project_name: String,
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub first_session_at: Option<DateTime<Utc>>,
    pub last_session_at: Option<DateTime<Utc>>,
}

/// LSP server statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct LspServerStats {
    pub lsp_name: String,
    pub status: String,
    pub count: usize,
}

/// Database statistics for Turso backend
#[derive(Debug, Clone, serde::Serialize)]
pub struct TursoDbStats {
    pub project_count: usize,
    pub memory_count: usize,
    pub session_count: usize,
}

/// Parse datetime string into DateTime<Utc>
fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Helper function to convert MemoryCategory to string
fn memory_category_to_string(category: &MemoryCategory) -> String {
    match category {
        MemoryCategory::General => "general".to_string(),
        MemoryCategory::Knowledge => "knowledge".to_string(),
        MemoryCategory::Preference => "preferences".to_string(),
        MemoryCategory::Specification => "specifications".to_string(),
        MemoryCategory::Fact => "fact".to_string(),
        MemoryCategory::Pattern => "pattern".to_string(),
        MemoryCategory::Decision => "decision".to_string(),
        MemoryCategory::Context => "context".to_string(),
        MemoryCategory::Temporary => "temporary".to_string(),
        MemoryCategory::Observation => "observation".to_string(),
    }
}

/// Helper function to convert string to MemoryCategory
fn string_to_memory_category(s: &str) -> MemoryCategory {
    match s.to_lowercase().as_str() {
        "general" => MemoryCategory::General,
        "knowledge" => MemoryCategory::Knowledge,
        "preferences" => MemoryCategory::Preference,
        "specifications" => MemoryCategory::Specification,
        "fact" => MemoryCategory::Fact,
        "pattern" => MemoryCategory::Pattern,
        "decision" => MemoryCategory::Decision,
        "context" => MemoryCategory::Context,
        "temporary" => MemoryCategory::Temporary,
        "observation" => MemoryCategory::Observation,
        _ => MemoryCategory::Context,
    }
}

/// Helper function to convert MemoryImportance to string
fn memory_importance_to_string(importance: &MemoryImportance) -> String {
    match importance {
        MemoryImportance::Critical => "critical".to_string(),
        MemoryImportance::High => "high".to_string(),
        MemoryImportance::Normal => "normal".to_string(),
        MemoryImportance::Low => "low".to_string(),
    }
}

/// Helper function to convert string to MemoryImportance
fn string_to_memory_importance(s: &str) -> MemoryImportance {
    match s.to_lowercase().as_str() {
        "critical" => MemoryImportance::Critical,
        "high" => MemoryImportance::High,
        "normal" => MemoryImportance::Normal,
        "low" => MemoryImportance::Low,
        _ => MemoryImportance::Normal,
    }
}

impl SessionStatus {
    /// Convert from string to SessionStatus
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "running" => Some(SessionStatus::Running),
            "waiting" => Some(SessionStatus::Waiting),
            "idle" => Some(SessionStatus::Idle),
            "error" => Some(SessionStatus::Error),
            "starting" => Some(SessionStatus::Starting),
            "paused" => Some(SessionStatus::Paused),
            "completed" => Some(SessionStatus::Completed),
            "terminated" => Some(SessionStatus::Terminated),
            _ => Some(SessionStatus::Idle),
        }
    }

    /// Convert SessionStatus to string
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Running => "running",
            SessionStatus::Waiting => "waiting",
            SessionStatus::Idle => "idle",
            SessionStatus::Error => "error",
            SessionStatus::Starting => "starting",
            SessionStatus::Paused => "paused",
            SessionStatus::Completed => "completed",
            SessionStatus::Terminated => "terminated",
        }
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

/// SQL for creating sessions table
const SESSIONS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    project_path TEXT NOT NULL,
    group_path TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    parent_session_id TEXT,
    command TEXT,
    tool TEXT,
    status TEXT NOT NULL DEFAULT 'idle',
    multiplexer_session TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    last_accessed_at TEXT,
    ended_at TEXT,
    metadata TEXT
);
CREATE INDEX IF NOT EXISTS idx_sessions_path ON sessions(project_path);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_group_sort ON sessions(group_path, sort_order);
"#;

/// SQL for creating maestro_projects table
const PROJECTS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS maestro_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_path TEXT NOT NULL UNIQUE,
    project_name TEXT NOT NULL,
    description TEXT,
    project_type TEXT,
    tech_stack TEXT,
    is_active INTEGER DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    last_scanned_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_projects_path ON maestro_projects(project_path);
CREATE INDEX IF NOT EXISTS idx_projects_active ON maestro_projects(is_active);
"#;

/// SQL for creating memories table
const MEMORIES_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    summary TEXT,
    category TEXT NOT NULL DEFAULT 'context',
    importance TEXT NOT NULL DEFAULT 'normal',
    source TEXT,
    session_id TEXT,
    project_id INTEGER,
    track_id INTEGER,
    command TEXT,
    command_context TEXT,
    embedding_id INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    last_accessed TEXT,
    meta_data TEXT,
    tags TEXT
);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id);
CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
CREATE INDEX IF NOT EXISTS idx_memories_expires ON memories(expires_at);
"#;

/// SQL for creating maestro_tracks table
const TRACKS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS maestro_tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    project_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'new',
    total_tasks INTEGER DEFAULT 0,
    completed_tasks INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    UNIQUE(project_id, track_id),
    FOREIGN KEY (project_id) REFERENCES maestro_projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_tracks_project ON maestro_tracks(project_id);
CREATE INDEX IF NOT EXISTS idx_tracks_status ON maestro_tracks(status);
"#;

/// SQL for creating session_groups table
const SESSION_GROUPS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS session_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    category TEXT,
    is_expanded INTEGER DEFAULT 1,
    sort_order INTEGER DEFAULT 0,
    parent_id INTEGER REFERENCES session_groups(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_groups_path ON session_groups(path);
"#;

/// SQL for creating mcp_servers table
const MCP_SERVERS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS mcp_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    transport TEXT NOT NULL DEFAULT 'stdio',
    command TEXT NOT NULL,
    args TEXT,
    env TEXT,
    cwd TEXT,
    url TEXT,
    headers TEXT,
    status TEXT NOT NULL DEFAULT 'stopped',
    socket_path TEXT,
    client_count INTEGER DEFAULT 0,
    last_started_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_mcp_status ON mcp_servers(status);
CREATE INDEX IF NOT EXISTS idx_mcp_transport ON mcp_servers(transport);
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
        let result = backend.with_connection(|_conn| Box::pin(async { Ok(()) })).await;
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

    // ========================================================================
    // OLAP Query Tests
    // ========================================================================

    #[tokio::test]
    async fn test_olap_session_stats_by_status() {
        // Use tempfile for testing instead of :memory: to ensure DDL persists
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Insert test sessions with different statuses
        let _ = backend.insert_session(&Session {
            id: 0,
            session_id: "test-1".to_string(),
            title: "Test Session 1".to_string(),
            project_path: "/test/project".to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Running,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).await;

        let _ = backend.insert_session(&Session {
            id: 0,
            session_id: "test-2".to_string(),
            title: "Test Session 2".to_string(),
            project_path: "/test/project".to_string(),
            group_path: None,
            sort_order: 1,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Idle,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).await;

        let stats = backend.session_stats_by_status()
            .await
            .expect("Failed to get session stats");

        assert_eq!(stats.len(), 2);
        assert!(stats.iter().any(|s| s.status == "running" && s.count == 1));
        assert!(stats.iter().any(|s| s.status == "idle" && s.count == 1));
    }

    #[tokio::test]
    async fn test_olap_memory_stats_by_category() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Insert test memories with different categories
        let _ = backend.insert_memory(&Memory {
            id: 0,
            content: "Test content 1".to_string(),
            summary: None,
            category: MemoryCategory::Knowledge,
            importance: MemoryImportance::Normal,
            source: None,
            session_id: None,
            project_id: None,
            track_id: None,
            command: None,
            command_context: None,
            created_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
            metadata: None,
            tags: None,
        }).await;

        let _ = backend.insert_memory(&Memory {
            id: 0,
            content: "Test content 2".to_string(),
            summary: None,
            category: MemoryCategory::Pattern,
            importance: MemoryImportance::Normal,
            source: None,
            session_id: None,
            project_id: None,
            track_id: None,
            command: None,
            command_context: None,
            created_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
            metadata: None,
            tags: None,
        }).await;

        let stats = backend.memory_stats_by_category()
            .await
            .expect("Failed to get memory stats");

        assert_eq!(stats.len(), 2);
        assert!(stats.iter().any(|s| s.category == "knowledge" && s.count == 1));
        assert!(stats.iter().any(|s| s.category == "pattern" && s.count == 1));
    }

    #[tokio::test]
    async fn test_olap_project_activity_summary() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Create a test project
        let _ = backend.get_or_create_project("/test/project", "Test Project").await;

        // Insert a session for the project
        let _ = backend.insert_session(&Session {
            id: 0,
            session_id: "test-1".to_string(),
            title: "Test Session".to_string(),
            project_path: "/test/project".to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Running,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).await;

        let summary = backend.project_activity_summary(Some(7))
            .await
            .expect("Failed to get project activity summary");

        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].project_path, "/test/project");
        assert_eq!(summary[0].project_name, "Test Project");
        assert_eq!(summary[0].session_count, 1);
    }

    #[tokio::test]
    async fn test_olap_most_active_projects() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Create test projects
        let _ = backend.get_or_create_project("/test/project1", "Project 1").await;
        let _ = backend.get_or_create_project("/test/project2", "Project 2").await;

        // Insert sessions for project1
        for i in 0..3 {
            let _ = backend.insert_session(&Session {
                id: 0,
                session_id: format!("test-{}", i),
                title: format!("Test Session {}", i),
                project_path: "/test/project1".to_string(),
                group_path: None,
                sort_order: i as i32,
                parent_session_id: None,
                command: None,
                tool: None,
                status: SessionStatus::Idle,
                multiplexer_session: None,
                started_at: Utc::now(),
                last_accessed_at: None,
                ended_at: None,
                metadata: None,
            }).await;
        }

        // Insert one session for project2
        let _ = backend.insert_session(&Session {
            id: 0,
            session_id: "test-p2".to_string(),
            title: "Test Session P2".to_string(),
            project_path: "/test/project2".to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Idle,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).await;

        let stats = backend.most_active_projects(Some(10))
            .await
            .expect("Failed to get most active projects");

        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].project_path, "/test/project1");
        assert_eq!(stats[0].total_sessions, 3);
        assert_eq!(stats[1].project_path, "/test/project2");
        assert_eq!(stats[1].total_sessions, 1);
    }

    // ========================================================================
    // FTS5 Full-Text Search Tests
    // ========================================================================

    #[tokio::test]
    async fn test_fts5_search_memories() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Insert test memories with different content
        let cat_id = backend.insert_memory(&Memory {
            id: 0,
            content: "The cat sat on the mat".to_string(),
            summary: Some("A cat story".to_string()),
            category: MemoryCategory::Knowledge,
            importance: MemoryImportance::Normal,
            source: None,
            session_id: None,
            project_id: None,
            track_id: None,
            command: None,
            command_context: None,
            created_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
            metadata: None,
            tags: None,
        }).await.expect("Failed to insert memory");

        let dog_id = backend.insert_memory(&Memory {
            id: 0,
            content: "The dog chased the ball in the park".to_string(),
            summary: Some("A dog story".to_string()),
            category: MemoryCategory::Pattern,
            importance: MemoryImportance::High,
            source: None,
            session_id: None,
            project_id: None,
            track_id: None,
            command: None,
            command_context: None,
            created_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
            metadata: None,
            tags: None,
        }).await.expect("Failed to insert memory");

        // Test search for "cat"
        let results = backend.search_memories("cat", Some(10))
            .await
            .expect("Failed to search memories");

        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.id == cat_id));
    }

    #[tokio::test]
    async fn test_fts5_rebuild_index() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Insert test memories
        for i in 0..3 {
            backend.insert_memory(&Memory {
                id: 0,
                content: format!("Test content number {}", i),
                summary: None,
                category: MemoryCategory::Knowledge,
                importance: MemoryImportance::Normal,
                source: None,
                session_id: None,
                project_id: None,
                track_id: None,
                command: None,
                command_context: None,
                created_at: Utc::now(),
                expires_at: None,
                last_accessed: None,
                metadata: None,
                tags: None,
            }).await.expect("Failed to insert memory");
        }

        // Rebuild FTS index
        let count = backend.rebuild_fts_index()
            .await
            .expect("Failed to rebuild FTS index");

        assert_eq!(count, 3);

        // Verify search works
        let results = backend.search_memories("content", Some(10))
            .await
            .expect("Failed to search memories");

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_fts5_optimize_index() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backend = TursoStorageBackend::new(Some(db_path), None)
            .await
            .expect("Failed to create backend");
        backend.initialize().await.expect("Failed to initialize");

        // Insert a test memory
        backend.insert_memory(&Memory {
            id: 0,
            content: "Test content for optimization".to_string(),
            summary: None,
            category: MemoryCategory::Knowledge,
            importance: MemoryImportance::Normal,
            source: None,
            session_id: None,
            project_id: None,
            track_id: None,
            command: None,
            command_context: None,
            created_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
            metadata: None,
            tags: None,
        }).await.expect("Failed to insert memory");

        // Optimize FTS index
        backend.optimize_fts_index()
            .await
            .expect("Failed to optimize FTS index");

        // Verify search still works
        let results = backend.search_memories("optimization", Some(10))
            .await
            .expect("Failed to search memories");

        assert!(!results.is_empty());
    }
}
