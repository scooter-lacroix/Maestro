//! Database Connection Manager
//!
//! Manages SQLite connections with connection pooling via DashMap.
//! Optimized for concurrent access with thread-safe operations.
//!
//! **NOTE:** This module is only available when the "rusqlite" feature is enabled.
//! The new TursoStorageBackend should be preferred for new code.

#[cfg(feature = "rusqlite")]
use anyhow::{Context, Result};
#[cfg(feature = "rusqlite")]
use dashmap::DashMap;
#[cfg(feature = "rusqlite")]
use rusqlite::Connection;
#[cfg(feature = "rusqlite")]
use std::path::{Path, PathBuf};
#[cfg(feature = "rusqlite")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "rusqlite")]
use tracing::info;

#[cfg(feature = "rusqlite")]
use super::schema::CREATE_TABLES_SQL;

#[cfg(feature = "rusqlite")]
#[derive(Clone)]
pub struct DatabaseManager {
    db_path: PathBuf,
    connection: Arc<Mutex<Connection>>,
    read_only: bool,
    /// Cache for prepared statement results
    cache: DashMap<String, serde_json::Value>,
}

#[cfg(feature = "rusqlite")]
impl DatabaseManager {
    /// Create new database manager
    pub fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".maestro");
            p.push("maestro.db");
            p
        });

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        info!("Opening database: {}", path.display());

        let mut conn = match Connection::open(&path) {
            Ok(c) => c,
            Err(e) => {
                // Graceful degradation: allow read-only mode when the DB is not writable.
                // This keeps the TUI usable (at least for reads) instead of failing hard.
                info!(
                    "Database open failed ({}). Falling back to read-only mode: {}",
                    e,
                    path.display()
                );
                Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .context("Failed to open database (read-only fallback)")?
            }
        };

        let mut read_only = false;

        // Enable WAL mode and Foreign Keys for better concurrent performance and integrity.
        // If the sandbox/filesystem prevents writes, fall back to read-only mode.
        let pragmas = "
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=10000;
            PRAGMA foreign_keys=ON;
            PRAGMA busy_timeout=5000;
        ";

        if let Err(e) = conn.execute_batch(pragmas) {
            let msg = e.to_string().to_lowercase();
            if msg.contains("readonly")
                || msg.contains("read-only")
                || msg.contains("sqlite_readonly")
            {
                info!(
                    "Database pragmas require write access; using read-only mode: {} ({})",
                    path.display(),
                    e
                );
                conn =
                    Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                        .context("Failed to reopen database (read-only after pragma failure)")?;
                read_only = true;
                let _ = conn.execute_batch(
                    "
                    PRAGMA query_only=ON;
                    PRAGMA foreign_keys=ON;
                    PRAGMA busy_timeout=5000;
                ",
                );
            } else {
                return Err(e).context("Failed to set pragmas");
            }
        }

        Ok(Self {
            db_path: path,
            connection: Arc::new(Mutex::new(conn)),
            read_only,
            cache: DashMap::new(),
        })
    }

    /// Initialize database schema
    pub fn initialize(&self) -> Result<()> {
        if self.read_only {
            info!(
                "Database is read-only; skipping schema initialization: {}",
                self.db_path.display()
            );
            return Ok(());
        }

        let mut conn = self.connection.lock().unwrap();
        // Use lenient execution for the initial schema as well, so that indices
        // referencing missing columns (to be added by migrations) don't block startup.
        Self::execute_batch_lenient(&conn, CREATE_TABLES_SQL)
            .context("Failed to initialize base tables")?;

        // Run migrations
        self.run_migrations(&mut conn)?;

        info!("Database initialized successfully");
        Ok(())
    }

    fn run_migrations(&self, conn: &mut Connection) -> Result<()> {
        use super::schema::MIGRATIONS;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY)",
            [],
        )?;

        for (version, sql) in MIGRATIONS {
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?)",
                [version],
                |row| row.get(0),
            )?;

            if !exists {
                info!("Running migration: {}", version);
                match *version {
                    // These migrations include ALTER TABLE statements that can fail on fresh DBs
                    // (because CREATE_TABLES_SQL already has the latest schema). Run them
                    // statement-by-statement and ignore duplicate-column errors so migrations are
                    // idempotent across schema versions.
                    "002_add_indexes"
                    | "003_tui_consolidation"
                    | "004_group_categorization"
                    | "005_mcp_transport"
                    | "006_mcp_cwd"
                    | "007_sessions_sort_order" => Self::execute_batch_lenient(conn, sql)
                        .context(format!("Failed to run migration {}", version))?,
                    _ => conn
                        .execute_batch(sql)
                        .context(format!("Failed to run migration {}", version))?,
                }
                conn.execute(
                    "INSERT INTO schema_migrations (version) VALUES (?)",
                    [version],
                )?;
            }
        }
        Ok(())
    }

    fn is_duplicate_column_error(err: &rusqlite::Error) -> bool {
        let msg = err.to_string().to_lowercase();
        msg.contains("duplicate column name") || msg.contains("no such column")
    }

    fn execute_batch_lenient(conn: &Connection, sql: &str) -> Result<()> {
        for stmt in sql.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            match conn.execute_batch(stmt) {
                Ok(()) => {}
                Err(e) if Self::is_duplicate_column_error(&e) => {}
                Err(e) => return Err(e).context("SQL statement failed"),
            }
        }
        Ok(())
    }

    /// Get database path
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Execute a query with the connection
    pub fn with_connection<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.connection.lock().unwrap();
        f(&conn)
    }

    /// Execute a mutable query with the connection
    pub fn with_connection_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Connection) -> Result<T>,
    {
        if self.read_only {
            anyhow::bail!("Database is read-only: {}", self.db_path.display());
        }
        let mut conn = self.connection.lock().unwrap();
        f(&mut conn)
    }

    /// Get cached value or compute it
    pub fn cached_or_compute<F>(&self, key: &str, compute: F) -> Result<serde_json::Value>
    where
        F: FnOnce() -> Result<serde_json::Value>,
    {
        if let Some(cached) = self.cache.get(key) {
            return Ok(cached.clone());
        }

        let value = compute()?;
        self.cache.insert(key.to_string(), value.clone());
        Ok(value)
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<DbStats> {
        self.with_connection(|conn| {
            let project_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM maestro_projects", [], |row| {
                    row.get(0)
                })
                .unwrap_or(0);

            let track_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM maestro_tracks", [], |row| row.get(0))
                .unwrap_or(0);

            let memory_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
                .unwrap_or(0);

            let session_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
                .unwrap_or(0);

            Ok(DbStats {
                project_count: project_count as usize,
                track_count: track_count as usize,
                memory_count: memory_count as usize,
                session_count: session_count as usize,
                db_size_bytes: std::fs::metadata(&self.db_path)
                    .map(|m| m.len())
                    .unwrap_or(0),
            })
        })
    }
}

/// Database statistics
#[cfg(feature = "rusqlite")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbStats {
    pub project_count: usize,
    pub track_count: usize,
    pub memory_count: usize,
    pub session_count: usize,
    pub db_size_bytes: u64,
}

#[cfg(all(test, feature = "rusqlite"))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db = DatabaseManager::new(Some(db_path.clone())).unwrap();
        db.initialize().unwrap();

        assert!(db_path.exists());
    }

    #[test]
    fn test_database_stats() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db = DatabaseManager::new(Some(db_path)).unwrap();
        db.initialize().unwrap();

        let stats = db.stats().unwrap();
        assert_eq!(stats.project_count, 0);
    }
}
