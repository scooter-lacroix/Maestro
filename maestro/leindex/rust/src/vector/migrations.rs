//! Database Migrations for Vector Store
//!
//! Provides schema migration functionality to handle database schema evolution
//! and fix security issues like SQL injection vulnerabilities.

use anyhow::{Context, Result};
use libsql::Database;
use tracing::{info, warn};

use super::metadata::ChunkType;

/// Current migration version
pub const CURRENT_MIGRATION_VERSION: i32 = 2;

/// Run all pending migrations
pub async fn run_migrations(database: &libsql::Database) -> Result<bool> {
    let conn = database
        .connect()
        .context("Failed to get connection for migration check")?;

    // Check if this is an in-memory database - if so, skip migrations
    // In-memory databases are created fresh each time with the correct schema
    let is_in_memory = {
        let mut db_check = conn
            .prepare("PRAGMA database_list")
            .await
            .context("Failed to prepare database list check")?
            .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
            .await
            .context("Failed to execute database list check")?;

        let mut in_memory = false;
        while let Some(row) = db_check.next().await? {
            let name: String = row.get(1).unwrap_or_default();
            let file: String = row.get(2).unwrap_or_default();
            // Check for in-memory indicators: empty file, contains :memory:, or starts with file::
            if name == "main"
                && (file.is_empty() || file.contains(":memory:") || file.starts_with("file::"))
            {
                in_memory = true;
                info!(
                    "Detected in-memory database (file={:?}), skipping migrations",
                    file
                );
                break;
            }
        }
        // db_check cursor is dropped here, releasing lock
        in_memory
    };

    if is_in_memory {
        return Ok(false);
    }

    let mut applied = false;

    // Migration v1: chunk_type TEXT -> INTEGER
    if migrate_chunk_type_to_integer(database).await? {
        applied = true;
    }

    // Migration v2: Add embedding_vector column and DiskANN index
    if migrate_add_embedding_vector_column(database).await? {
        applied = true;
    }

    Ok(applied)
}

/// Migrate chunk_type column from TEXT to INTEGER (Task 7.6.10)
///
/// **SECURITY CRITICAL:** Existing databases may have chunk_type as TEXT,
/// which is vulnerable to SQL injection via debug string formatting.
/// New code uses INTEGER with to_i32()/from_i32() for safe conversion.
/// This migration updates old databases to the safe INTEGER schema.
///
/// Migration steps:
/// 1. Check migration version - skip if already at current version
/// 2. Check if chunk_type is TEXT - skip if already INTEGER
/// 3. Create new vectors_new table with INTEGER schema
/// 4. Migrate data, converting TEXT enum values to INTEGER discriminants
/// 5. Drop old table and rename new table
/// 6. Recreate indexes
/// 7. Update migration version
pub async fn migrate_chunk_type_to_integer(database: &Database) -> Result<bool> {
    let conn = database
        .connect()
        .context("Failed to get connection for migration")?;

    // Step 1: Check migration version
    let migration_version = check_migration_version(&conn).await?;
    if migration_version >= CURRENT_MIGRATION_VERSION {
        info!(
            "Schema migration already applied (version {})",
            migration_version
        );
        return Ok(false);
    }

    // Step 2: Check if chunk_type is TEXT (needs migration)
    let chunk_type_is_text = check_chunk_type_is_text(&conn).await?;
    if !chunk_type_is_text {
        info!("chunk_type column is already INTEGER, skipping migration");
        // Still update migration version for consistency
        update_migration_version(&conn, CURRENT_MIGRATION_VERSION).await?;
        return Ok(false);
    }

    info!("Starting migration: chunk_type TEXT -> INTEGER");

    // Step 3: Create new table with INTEGER schema
    conn.execute(
        r#"
        CREATE TABLE vectors_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vector_id TEXT NOT NULL UNIQUE,
            file_path TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            start_line INTEGER,
            end_line INTEGER,
            chunk_type INTEGER NOT NULL,
            parent_context TEXT,
            content TEXT,
            embedding TEXT NOT NULL,
            embedding_model TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT
        )
        "#,
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await
    .context("Failed to create new vectors_new table")?;

    // Step 4: Migrate data from old table to new table
    let stmt = conn
        .prepare(
            r#"
            SELECT
                id, vector_id, file_path, chunk_index, start_line, end_line,
                chunk_type, parent_context, content, embedding,
                embedding_model, created_at, updated_at
            FROM vectors
            "#,
        )
        .await
        .context("Failed to prepare migration SELECT query")?;

    let mut rows = stmt
        .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
        .await
        .context("Failed to execute migration SELECT query")?;

    let mut migrated_count = 0;
    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let vector_id: String = row.get(1)?;
        let file_path: String = row.get(2)?;
        let chunk_index: i64 = row.get(3)?;
        let start_line: Option<i64> = row.get(4)?; // May be NULL
        let end_line: Option<i64> = row.get(5)?; // May be NULL
        let chunk_type_text: String = row.get(6)?;
        let parent_context: Option<String> = row.get(7)?;
        let content: Option<String> = row.get(8)?;
        let embedding: String = row.get(9)?;
        let embedding_model: String = row.get(10)?;
        let created_at: String = row.get(11)?;
        let updated_at: Option<String> = row.get(12)?;

        // Convert TEXT enum to INTEGER discriminant
        let chunk_type_int = chunk_type_from_text(&chunk_type_text).to_i32() as i64;

        // Insert into new table
        conn.execute(
            r#"
            INSERT INTO vectors_new
            (id, vector_id, file_path, chunk_index, start_line, end_line,
             chunk_type, parent_context, content, embedding,
             embedding_model, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            libsql::params_from_iter(
                [
                    libsql::Value::Integer(id),
                    libsql::Value::Text(vector_id),
                    libsql::Value::Text(file_path),
                    libsql::Value::Integer(chunk_index),
                    start_line
                        .map(|v| libsql::Value::Integer(v))
                        .unwrap_or(libsql::Value::Null),
                    end_line
                        .map(|v| libsql::Value::Integer(v))
                        .unwrap_or(libsql::Value::Null),
                    libsql::Value::Integer(chunk_type_int),
                    libsql::Value::Text(parent_context.unwrap_or_default()),
                    libsql::Value::Text(content.unwrap_or_default()),
                    libsql::Value::Text(embedding),
                    libsql::Value::Text(embedding_model),
                    libsql::Value::Text(created_at),
                    updated_at
                        .map(|v| libsql::Value::Text(v))
                        .unwrap_or(libsql::Value::Null),
                ]
                .into_iter(),
            ),
        )
        .await
        .context("Failed to insert migrated row")?;

        migrated_count += 1;
    }

    info!(
        "Migrated {} vectors from TEXT to INTEGER schema",
        migrated_count
    );

    // Step 5: Drop old table and rename new table
    conn.execute(
        "DROP TABLE vectors",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await
    .context("Failed to drop old vectors table")?;

    conn.execute(
        "ALTER TABLE vectors_new RENAME TO vectors",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await
    .context("Failed to rename vectors_new to vectors")?;

    // Step 6: Recreate indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_vectors_file_path ON vectors(file_path)",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await
    .context("Failed to create file_path index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_vectors_chunk_type ON vectors(chunk_type)",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await
    .context("Failed to create chunk_type index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_vectors_created_at ON vectors(created_at)",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await
    .context("Failed to create created_at index")?;

    // Step 7: Update migration version
    update_migration_version(&conn, CURRENT_MIGRATION_VERSION).await?;

    info!(
        "Schema migration completed successfully: {} vectors migrated",
        migrated_count
    );
    Ok(true)
}

/// Migration v2: Add embedding_vector column and DiskANN index
pub async fn migrate_add_embedding_vector_column(database: &libsql::Database) -> Result<bool> {
    let conn = database
        .connect()
        .context("Failed to get connection for migration v2")?;

    // Step 1: Check migration version
    let migration_version = check_migration_version(&conn).await?;
    if migration_version >= 2 {
        return Ok(false);
    }

    info!("Starting migration v2: Add embedding_vector column and DiskANN index");

    // Step 2: Check if column already exists
    // CRITICAL: Wrap in block to ensure rows cursor is dropped before backfill
    // to prevent SQLite lock contention
    let column_exists = {
        let mut rows = conn
            .prepare("PRAGMA table_info(vectors)")
            .await
            .context("Failed to prepare table_info query")?
            .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
            .await
            .context("Failed to execute table_info query")?;

        let mut exists = false;
        while let Some(row) = rows.next().await? {
            let name: String = row.get(1)?;
            if name == "embedding_vector" {
                exists = true;
                break;
            }
        }
        // rows is dropped here, releasing any database lock
        exists
    };

    if !column_exists {
        // Add column
        conn.execute(
            "ALTER TABLE vectors ADD COLUMN embedding_vector BLOB",
            libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
        )
        .await
        .context("Failed to add embedding_vector column")?;
        info!("Added embedding_vector BLOB column to vectors table");
    }

    // Step 3: Backfill data
    let backfilled = backfill_embedding_vectors(database).await?;
    info!("Backfilled {} vectors with base64 embeddings", backfilled);

    // Step 4: Create DiskANN index
    // Note: We use execute instead of a prepared statement because 'USING' might
    // be rejected by some parsers if not supported, but we want to try it.
    let result = conn.execute(
        "CREATE INDEX IF NOT EXISTS vectors_diskann_idx ON vectors USING libsql_vector_idx(embedding_vector)",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    ).await;

    match result {
        Ok(_) => info!("Successfully created DiskANN index: vectors_diskann_idx"),
        Err(e) => {
            warn!("Could not create DiskANN index (this is expected if vector extension is not loaded): {}", e);
            // We don't fail the migration because we want search to still work with fallback
        }
    }

    // Step 5: Update migration version
    update_migration_version(&conn, 2).await?;

    info!("Migration v2 completed successfully");
    Ok(true)
}

/// Migration: backfill embedding_vector column for existing rows
pub async fn backfill_embedding_vectors(database: &libsql::Database) -> Result<usize> {
    let conn = database
        .connect()
        .context("Failed to get connection for backfill")?;

    // Get all vectors without embedding_vector
    let stmt = conn
        .prepare("SELECT id, embedding FROM vectors WHERE embedding_vector IS NULL")
        .await
        .context("Failed to prepare backfill query")?;

    let mut rows = stmt
        .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
        .await?;
    let mut migrated = 0;

    // Use a transaction for efficiency if there are many rows
    conn.execute(
        "BEGIN TRANSACTION",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await?;

    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let embedding_json: String = row.get(1)?;

        let embedding: Vec<f32> = serde_json::from_str(&embedding_json)
            .context("Failed to parse embedding JSON for backfill")?;

        // PERF: Store raw bytes in BLOB (Task 8.7)
        let mut embedding_bytes = Vec::with_capacity(embedding.len() * 4);
        for &f in &embedding {
            embedding_bytes.extend_from_slice(&f.to_le_bytes());
        }

        conn.execute(
            "UPDATE vectors SET embedding_vector = ?1 WHERE id = ?2",
            libsql::params_from_iter(
                [
                    libsql::Value::Blob(embedding_bytes),
                    libsql::Value::Integer(id),
                ]
                .into_iter(),
            ),
        )
        .await
        .context("Failed to update row during backfill")?;

        migrated += 1;
    }

    conn.execute(
        "COMMIT",
        libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
    )
    .await?;

    Ok(migrated)
}

/// Convert TEXT representation of ChunkType to enum variant
///
/// Handles various string formats (lowercase, uppercase, with underscores, etc.)
/// for robustness when reading from old TEXT columns
fn chunk_type_from_text(text: &str) -> ChunkType {
    match text.to_lowercase().trim() {
        "function" => ChunkType::Function,
        "class" => ChunkType::Class,
        "module" => ChunkType::Module,
        "import" => ChunkType::Import,
        "comment" => ChunkType::Comment,
        "text" => ChunkType::Text,
        "other" => ChunkType::Other,
        // Fallback for unknown values
        _ => {
            warn!(
                "Unknown chunk_type value in database: {}, using Other",
                text
            );
            ChunkType::Other
        }
    }
}

/// Check if chunk_type column is TEXT type
async fn check_chunk_type_is_text(conn: &libsql::Connection) -> Result<bool> {
    // CRITICAL: Ensure cursor is fully consumed to avoid lock contention
    let mut rows = conn
        .prepare("PRAGMA table_info(vectors)")
        .await
        .context("Failed to prepare table_info query")?
        .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
        .await
        .context("Failed to execute table_info query")?;

    let mut is_text = true; // Default to TEXT if column not found
    while let Some(row) = rows.next().await? {
        let name: String = row.get(1)?;
        if name == "chunk_type" {
            let typ: String = row.get(2)?;
            // In SQLite, TEXT type may be reported as "TEXT" or contain "TEXT"
            is_text = typ == "TEXT" || typ.contains("TEXT");
            break;
        }
    }
    // rows is fully consumed here, releasing the lock
    Ok(is_text)
}

/// Check current migration version
async fn check_migration_version(conn: &libsql::Connection) -> Result<i32> {
    // Check if migration_info table exists
    let mut rows = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='migration_info'")
        .await
        .context("Failed to prepare migration_info check")?
        .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
        .await
        .context("Failed to execute migration_info check")?;

    if rows.next().await?.is_none() {
        // Table doesn't exist, create it with version 0
        conn.execute(
            "CREATE TABLE migration_info (version INTEGER NOT NULL DEFAULT 0)",
            libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
        )
        .await
        .context("Failed to create migration_info table")?;

        conn.execute(
            "INSERT INTO migration_info (version) VALUES (0)",
            libsql::params_from_iter([libsql::Value::Integer(0)].into_iter()),
        )
        .await
        .context("Failed to initialize migration version")?;

        return Ok(0);
    }

    // Read current version
    let mut rows = conn
        .prepare("SELECT version FROM migration_info")
        .await
        .context("Failed to prepare version SELECT")?
        .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
        .await
        .context("Failed to execute version SELECT")?;

    if let Some(row) = rows.next().await? {
        Ok(row.get::<i64>(0)? as i32)
    } else {
        // No rows, should not happen after table creation above
        warn!("migration_info table exists but has no rows, creating version 0");
        conn.execute(
            "INSERT INTO migration_info (version) VALUES (0)",
            libsql::params_from_iter([libsql::Value::Integer(0)].into_iter()),
        )
        .await
        .context("Failed to initialize migration version")?;
        Ok(0)
    }
}

/// Update migration version
async fn update_migration_version(conn: &libsql::Connection, version: i32) -> Result<()> {
    conn.execute(
        "UPDATE migration_info SET version = ?1",
        libsql::params_from_iter([libsql::Value::Integer(version as i64)].into_iter()),
    )
    .await
    .context("Failed to update migration version")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_type_from_text() {
        // Test various formats
        assert_eq!(chunk_type_from_text("function"), ChunkType::Function);
        assert_eq!(chunk_type_from_text("Function"), ChunkType::Function);
        assert_eq!(chunk_type_from_text("FUNCTION"), ChunkType::Function);
        assert_eq!(chunk_type_from_text("class"), ChunkType::Class);
        assert_eq!(chunk_type_from_text("module"), ChunkType::Module);
        assert_eq!(chunk_type_from_text("import"), ChunkType::Import);
        assert_eq!(chunk_type_from_text("comment"), ChunkType::Comment);
        assert_eq!(chunk_type_from_text("text"), ChunkType::Text);
        assert_eq!(chunk_type_from_text("other"), ChunkType::Other);
        assert_eq!(chunk_type_from_text("unknown"), ChunkType::Other);
    }

    #[test]
    fn test_chunk_type_to_i32_roundtrip() {
        // Test that enum discriminants match our conversion
        assert_eq!(ChunkType::Function.to_i32(), 0);
        assert_eq!(ChunkType::Class.to_i32(), 1);
        assert_eq!(ChunkType::Module.to_i32(), 2);
        assert_eq!(ChunkType::Import.to_i32(), 3);
        assert_eq!(ChunkType::Comment.to_i32(), 4);
        assert_eq!(ChunkType::Text.to_i32(), 5);
        assert_eq!(ChunkType::Other.to_i32(), 6);

        // Test roundtrip through from_i32
        assert_eq!(ChunkType::from_i32(0), ChunkType::Function);
        assert_eq!(ChunkType::from_i32(1), ChunkType::Class);
        assert_eq!(ChunkType::from_i32(2), ChunkType::Module);
        assert_eq!(ChunkType::from_i32(3), ChunkType::Import);
        assert_eq!(ChunkType::from_i32(4), ChunkType::Comment);
        assert_eq!(ChunkType::from_i32(5), ChunkType::Text);
        assert_eq!(ChunkType::from_i32(6), ChunkType::Other);
        assert_eq!(ChunkType::from_i32(999), ChunkType::Other); // Unknown -> Other
    }
}
