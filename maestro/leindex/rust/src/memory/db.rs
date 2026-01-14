//! Database Connection Manager
//!
//! Manages SQLite connections with connection pooling via DashMap.
//! Optimized for concurrent access with thread-safe operations.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use dashmap::DashMap;
use tracing::info;

use super::schema::CREATE_TABLES_SQL;

#[derive(Clone)]
pub struct DatabaseManager {
    db_path: PathBuf,
    connection: Arc<Mutex<Connection>>,
    /// Cache for prepared statement results
    cache: DashMap<String, serde_json::Value>,
}

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
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        info!("Opening database: {}", path.display());
        
        let conn = Connection::open(&path)
            .context("Failed to open database")?;

        // Enable WAL mode and Foreign Keys for better concurrent performance and integrity
        conn.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=10000;
            PRAGMA foreign_keys=ON;
            PRAGMA busy_timeout=5000;
        ")
            .context("Failed to set pragmas")?;

        Ok(Self {
            db_path: path,
            connection: Arc::new(Mutex::new(conn)),
            cache: DashMap::new(),
        })
    }

    /// Initialize database schema
    pub fn initialize(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute_batch(CREATE_TABLES_SQL)
            .context("Failed to create tables")?;
        info!("Database initialized successfully");
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
        let mut conn = self.connection.lock().unwrap();
        f(&mut conn)
    }

    /// Get cached value or compute it
    pub fn cached_or_compute<F>(
        &self,
        key: &str,
        compute: F,
    ) -> Result<serde_json::Value>
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
                .query_row("SELECT COUNT(*) FROM maestro_projects", [], |row| row.get(0))
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
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbStats {
    pub project_count: usize,
    pub track_count: usize,
    pub memory_count: usize,
    pub session_count: usize,
    pub db_size_bytes: u64,
}

#[cfg(test)]
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
