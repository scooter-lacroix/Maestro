//!
//! # Database Migration Framework
//!
//! A comprehensive migration system for SQLite/DuckDB/Tantivy to Turso migrations.
//! Provides async migration management with rollback support, version tracking,
//! and state management compatible with libsql 0.9+.
//!
//! ## Features
//!
//! - **Async Migration Support**: Full async/await pattern compatible with libsql
//! - **Rollback Capability**: Each migration can define a `down()` method for rollback
//! - **Version Tracking**: Semantic versioning system for migration identification
//! - **State Management**: Persistent migration state tracking in the database
//! - **Idempotent Operations**: Safe to run multiple times without side effects
//! - **Error Handling**: Comprehensive error handling with anyhow::Result
//!
//! ## Usage
//!
//! ```rust,ignore
//! use migrations::{Migration, MigrationManager};
//! use libsql::Database;
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
//!     async fn up(&self, conn: &libsql::Connection) -> anyhow::Result<()> {
//!         // Execute migration up
//!         Ok(())
//!     }
//!
//!     async fn down(&self, conn: &libsql::Connection) -> anyhow::Result<()> {
//!         // Execute migration rollback
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let db = Database::open_with_local_sync(":memory:").await?;
//!     let manager = MigrationManager::new(db.clone());
//!     manager.run_migrations(&[&CreateUsersTable]).await?;
//!     Ok(())
//! }
//! ```
//!

use anyhow::{Context, Result};
use async_trait::async_trait;
use libsql::{params_from_iter, Connection, Database};
use std::fmt;
use std::sync::Arc;

/// Validates that a table name is safe to use in SQL queries.
///
/// This is a defensive measure to prevent SQL injection through table names,
/// even though table names are not typically user-provided in this module.
///
/// # Arguments
///
/// * `name` - The table name to validate
///
/// # Returns
///
/// `Ok(())` if the table name is safe, or an error describing the issue.
fn validate_table_name(name: &str) -> Result<()> {
    // Empty table names are invalid
    if name.is_empty() {
        anyhow::bail!("Table name cannot be empty");
    }

    // Check for SQL injection patterns
    if name.contains(';') || name.contains("--") || name.contains("/*") {
        anyhow::bail!("Table name contains potentially dangerous characters");
    }

    // Table names should be ASCII alphanumeric with underscores only
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        anyhow::bail!("Table name must contain only alphanumeric characters and underscores");
    }

    // Must start with a letter or underscore (SQL identifier rules)
    let first_char = name.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' {
        anyhow::bail!("Table name must start with a letter or underscore");
    }

    Ok(())
}

/// Represents the current state of a migration.
///
/// This enum tracks whether a migration is pending, has been applied,
/// or has been rolled back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationState {
    /// Migration has not been applied yet
    Pending,
    /// Migration has been successfully applied
    Applied,
    /// Migration was rolled back after being applied
    RolledBack,
}

impl fmt::Display for MigrationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationState::Pending => write!(f, "PENDING"),
            MigrationState::Applied => write!(f, "APPLIED"),
            MigrationState::RolledBack => write!(f, "ROLLED_BACK"),
        }
    }
}

impl MigrationState {
    /// Parse a string into a MigrationState
    pub fn from_str(s: &str) -> Self {
        match s {
            "APPLIED" => MigrationState::Applied,
            "ROLLED_BACK" => MigrationState::RolledBack,
            _ => MigrationState::Pending,
        }
    }
}

/// Metadata about a migration's execution state.
///
/// This struct holds information about when a migration was applied,
/// its current state, and any associated metadata.
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// Unique version identifier for the migration
    pub version: String,
    /// Current state of the migration
    pub state: MigrationState,
    /// Timestamp when the migration was applied (RFC3339 format)
    pub applied_at: Option<String>,
    /// Timestamp when the migration was rolled back (RFC3339 format)
    pub rolled_back_at: Option<String>,
    /// Optional checksum for integrity verification
    pub checksum: Option<String>,
}

impl MigrationRecord {
    /// Creates a new migration record with pending state.
    ///
    /// # Arguments
    ///
    /// * `version` - The migration version string
    ///
    /// # Returns
    ///
    /// A new `MigrationRecord` instance in pending state.
    #[must_use]
    pub fn new_pending(version: String) -> Self {
        Self {
            version,
            state: MigrationState::Pending,
            applied_at: None,
            rolled_back_at: None,
            checksum: None,
        }
    }

    /// Marks the migration as applied with current timestamp.
    ///
    /// # Arguments
    ///
    /// * `checksum` - Optional checksum for integrity verification
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[must_use]
    pub fn mark_applied(&mut self, checksum: Option<String>) -> &mut Self {
        self.state = MigrationState::Applied;
        self.applied_at = Some(chrono::Utc::now().to_rfc3339());
        self.checksum = checksum;
        self
    }

    /// Marks the migration as rolled back with current timestamp.
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    #[must_use]
    pub fn mark_rolled_back(&mut self) -> &mut Self {
        self.state = MigrationState::RolledBack;
        self.rolled_back_at = Some(chrono::Utc::now().to_rfc3339());
        self
    }
}

/// Trait representing a database migration.
///
/// Implement this trait to define migrations with `up` (apply) and
/// `down` (rollback) operations. Each migration must have a unique
/// version string for identification and ordering.
///
/// # Example
///
/// ```rust,ignore
/// use migrations::Migration;
/// use libsql::Connection;
///
/// struct MyMigration;
///
/// #[async_trait::async_trait]
/// impl Migration for MyMigration {
///     fn version(&self) -> &str {
///         "2024_01_15_001_create_users"
///     }
///
///     async fn up(&self, conn: &Connection) -> anyhow::Result<()> {
///         // Apply migration
///         Ok(())
///     }
///
///     async fn down(&self, conn: &Connection) -> anyhow::Result<()> {
///         // Rollback migration
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Migration: Send + Sync {
    /// Returns the unique version identifier for this migration.
    ///
    /// Version strings should follow a consistent format for sorting.
    /// Recommended format: `YYYY_MM_DD_NNN descriptive_name`
    ///
    /// # Returns
    ///
    /// A string slice containing the migration version.
    fn version(&self) -> &str;

    /// Applies the migration to the database.
    ///
    /// This method is called when the migration should be applied.
    /// It should contain all necessary SQL statements or operations
    /// to implement the desired database changes.
    ///
    /// # Arguments
    ///
    /// * `conn` - Reference to the libsql connection
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error describing what went wrong.
    async fn up(&self, conn: &Connection) -> Result<()>;

    /// Rolls back the migration.
    ///
    /// This method should reverse the changes made by `up()`.
    /// It is called when rolling back a migration. If a migration
    /// cannot be rolled back, return an error.
    ///
    /// # Arguments
    ///
    /// * `conn` - Reference to the libsql connection
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error describing what went wrong.
    async fn down(&self, conn: &Connection) -> Result<()>;

    /// Returns a checksum for integrity verification.
    ///
    /// This can be used to verify that a migration has not been modified
    /// since it was applied. By default, returns the migration's version string.
    ///
    /// # Returns
    ///
    /// A string containing the migration checksum.
    fn checksum(&self) -> String {
        self.version().to_string()
    }

    /// Returns a human-readable description of this migration.
    ///
    /// # Returns
    ///
    /// A string describing what this migration does.
    fn description(&self) -> String {
        format!("Migration: {}", self.version())
    }
}

/// Configuration for the migration manager.
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Table name for storing migration records
    pub migrations_table: String,
    /// Whether to create the migrations table if it doesn't exist
    pub create_table_if_missing: bool,
    /// Maximum number of migrations to run concurrently (for future optimization)
    ///
    /// Currently, migrations are run sequentially, but this field allows
    /// for future concurrent execution. Set to 1 for sequential execution.
    pub max_concurrent_migrations: usize,
    /// Whether to wrap migrations in transactions for atomicity
    ///
    /// When true, each migration runs in its own transaction. When false,
    /// migrations run without explicit transaction wrapping (useful for
    /// migrations that require DDL statements that can't be rolled back).
    pub use_transactions: bool,
}

impl MigrationConfig {
    /// Creates a new MigrationConfig with the given table name.
    ///
    /// # Arguments
    ///
    /// * `migrations_table` - The name of the table for storing migration records
    ///
    /// # Returns
    ///
    /// A new `MigrationConfig` instance, or an error if the table name is invalid.
    pub fn new(migrations_table: String) -> Result<Self> {
        validate_table_name(&migrations_table)?;
        Ok(Self {
            migrations_table,
            create_table_if_missing: true,
            max_concurrent_migrations: 1, // Sequential by default
            use_transactions: true,       // Use transactions by default
        })
    }

    /// Creates a new MigrationConfig that won't automatically create the table.
    ///
    /// # Arguments
    ///
    /// * `migrations_table` - The name of the table for storing migration records
    ///
    /// # Returns
    ///
    /// A new `MigrationConfig` instance, or an error if the table name is invalid.
    pub fn without_auto_create(migrations_table: String) -> Result<Self> {
        validate_table_name(&migrations_table)?;
        Ok(Self {
            migrations_table,
            create_table_if_missing: false,
            max_concurrent_migrations: 1,
            use_transactions: true,
        })
    }

    /// Sets the maximum number of concurrent migrations.
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum concurrent migrations (1 for sequential)
    ///
    /// # Returns
    ///
    /// Self for builder pattern chaining.
    #[must_use]
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_migrations = max.max(1); // Ensure at least 1
        self
    }

    /// Sets whether to use transactions for migrations.
    ///
    /// # Arguments
    ///
    /// * `use_tx` - Whether to wrap migrations in transactions
    ///
    /// # Returns
    ///
    /// Self for builder pattern chaining.
    #[must_use]
    pub fn with_transactions(mut self, use_tx: bool) -> Self {
        self.use_transactions = use_tx;
        self
    }
}

impl Default for MigrationConfig {
    fn default() -> Self {
        // Default table name is guaranteed to be valid
        Self {
            migrations_table: "schema_migrations".to_string(),
            create_table_if_missing: true,
            max_concurrent_migrations: 1, // Sequential by default
            use_transactions: true,
        }
    }
}

/// Manages database migrations with support for applying and rolling back.
///
/// The `MigrationManager` is the main entry point for running migrations.
/// It tracks applied migrations, handles versioning, and provides rollback
/// capabilities.
///
/// # Example
///
/// ```rust,ignore
/// use migrations::{Migration, MigrationManager};
/// use libsql::Database;
///
/// #[derive(Debug)]
/// struct InitialSchema;
///
/// #[async_trait::async_trait]
/// impl Migration for InitialSchema {
///     fn version(&self) -> &str {
///         "2024_01_01_001_initial_schema"
///     }
///
///     async fn up(&self, conn: &libsql::Connection) -> anyhow::Result<()> {
///         Ok(())
///     }
///
///     async fn down(&self, conn: &libsql::Connection) -> anyhow::Result<()> {
///         Ok(())
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let db = Database::open_with_local_sync(":memory:").await?;
///     let manager = MigrationManager::new(db.clone());
///
///     // Run all migrations
///     manager.run_migrations(&[&InitialSchema]).await?;
///
///     // Get migration status
///     let status = manager.get_status().await?;
///     println!("Applied migrations: {}", status.applied_count());
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct MigrationManager {
    /// Database connection (thread-safe via Arc)
    db: Arc<Database>,
    /// Configuration for migration behavior
    config: MigrationConfig,
}

impl MigrationManager {
    /// Creates a new migration manager with default configuration.
    ///
    /// # Arguments
    ///
    /// * `db` - The libsql database
    ///
    /// # Returns
    ///
    /// A new `MigrationManager` instance.
    #[must_use]
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            config: MigrationConfig::default(),
        }
    }

    /// Creates a new migration manager with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `db` - The libsql database
    /// * `config` - Custom migration configuration
    ///
    /// # Returns
    ///
    /// A new `MigrationManager` instance.
    ///
    /// # Panics
    ///
    /// Panics if the table name in the config is invalid. Use [`MigrationConfig::new`]
    /// to create validated configurations.
    #[must_use]
    pub fn with_config(db: Arc<Database>, config: MigrationConfig) -> Self {
        // Validate table name on construction - panic here since it's a programming error
        validate_table_name(&config.migrations_table)
            .expect("Invalid table name in MigrationConfig");
        Self { db, config }
    }

    /// Get a new connection from the database.
    async fn get_connection(&self) -> Result<Connection> {
        self.db
            .connect()
            .context("Failed to get connection from database")
    }

    /// Initializes the migrations table if it doesn't exist.
    ///
    /// This method is called automatically before running migrations
    /// if `create_table_if_missing` is enabled in the config.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error.
    pub async fn initialize(&self) -> Result<()> {
        let table_name = &self.config.migrations_table;
        let conn = self.get_connection().await?;

        // Create migrations tracking table
        let create_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                version TEXT PRIMARY KEY,
                state TEXT NOT NULL DEFAULT 'PENDING',
                applied_at TEXT,
                rolled_back_at TEXT,
                checksum TEXT,
                description TEXT
            )
            "#,
            table_name
        );

        // Create index for faster lookups
        let create_index_sql = format!(
            r#"
            CREATE INDEX IF NOT EXISTS idx_{}_state ON {} (state)
            "#,
            table_name.replace('.', "_"),
            table_name
        );

        conn.execute(
            &create_sql,
            libsql::params_from_iter(std::iter::empty::<&str>()),
        )
        .await
        .with_context(|| format!("Failed to create migrations table: {}", table_name))?;

        conn.execute(
            &create_index_sql,
            libsql::params_from_iter(std::iter::empty::<&str>()),
        )
        .await
        .context("Failed to create migrations state index")?;

        tracing::info!("Initialized migrations table: {}", table_name);
        Ok(())
    }

    /// Gets all migration records from the database.
    ///
    /// # Returns
    ///
    /// A vector of `MigrationRecord` instances, or an error.
    pub async fn get_migration_records(&self) -> Result<Vec<MigrationRecord>> {
        let table_name = &self.config.migrations_table;
        let conn = self.get_connection().await?;

        let sql = format!(
            r#"
            SELECT version, state, applied_at, rolled_back_at, checksum
            FROM {}
            ORDER BY applied_at ASC
            "#,
            table_name
        );

        let mut rows = conn
            .query(&sql, libsql::params_from_iter(std::iter::empty::<&str>()))
            .await
            .with_context(|| format!("Failed to query migration records from: {}", table_name))?;

        let mut records = Vec::new();

        while let Some(row) = rows.next().await? {
            let version: String = row.get(0)?;
            let state: String = row.get(1)?;
            let applied_at: Option<String> = row.get(2)?;
            let rolled_back_at: Option<String> = row.get(3)?;
            let checksum: Option<String> = row.get(4)?;

            records.push(MigrationRecord {
                version,
                state: MigrationState::from_str(&state),
                applied_at,
                rolled_back_at,
                checksum,
            });
        }

        Ok(records)
    }

    /// Checks if a migration has been applied.
    ///
    /// # Arguments
    ///
    /// * `version` - The migration version to check
    ///
    /// # Returns
    ///
    /// `true` if the migration is applied, `false` otherwise.
    pub async fn is_migration_applied(&self, version: &str) -> Result<bool> {
        let table_name = &self.config.migrations_table;
        let conn = self.get_connection().await?;

        let sql = format!(
            r#"
            SELECT 1 FROM {}
            WHERE version = ? AND state = 'APPLIED'
            LIMIT 1
            "#,
            table_name
        );

        let mut rows = conn
            .query(&sql, libsql::params_from_iter([version]))
            .await
            .with_context(|| format!("Failed to check migration status for: {}", version))?;

        Ok(rows.next().await?.is_some())
    }

    /// Records a migration as applied in the database.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection to use (for atomicity)
    /// * `migration` - The migration to record
    /// * `checksum` - The migration checksum
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error.
    async fn record_migration_applied(
        &self,
        conn: &Connection,
        migration: &dyn Migration,
        checksum: &str,
    ) -> Result<()> {
        let table_name = &self.config.migrations_table;
        let description = migration.description();

        let sql = format!(
            r#"
            INSERT INTO {} (version, state, applied_at, checksum, description)
            VALUES (?, 'APPLIED', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?, ?)
            ON CONFLICT(version) DO UPDATE SET
                state = 'APPLIED',
                applied_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                checksum = excluded.checksum
            "#,
            table_name
        );

        conn.execute(
            &sql,
            params_from_iter([migration.version(), checksum, &description]),
        )
        .await
        .with_context(|| {
            format!(
                "Failed to record migration as applied: {}",
                migration.version()
            )
        })?;

        Ok(())
    }

    /// Records a migration as rolled back in the database.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection to use (for atomicity)
    /// * `version` - The migration version to record
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error.
    async fn record_migration_rolled_back(&self, conn: &Connection, version: &str) -> Result<()> {
        let table_name = &self.config.migrations_table;

        let sql = format!(
            r#"
            UPDATE {}
            SET state = 'ROLLED_BACK', rolled_back_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE version = ?
            "#,
            table_name
        );

        conn.execute(&sql, params_from_iter([version]))
            .await
            .with_context(|| format!("Failed to record migration as rolled back: {}", version))?;

        Ok(())
    }

    /// Runs a single migration up (apply).
    ///
    /// # Arguments
    ///
    /// * `migration` - The migration to apply
    /// * `force` - Whether to force re-application even if already applied
    ///
    /// # Returns
    ///
    /// `Ok(true)` if migration was applied, `Ok(false)` if skipped,
    /// or an error.
    pub async fn migrate_up(&self, migration: &dyn Migration, force: bool) -> Result<bool> {
        let version = migration.version();
        let is_applied = self.is_migration_applied(version).await?;

        if is_applied && !force {
            tracing::debug!("Migration {} already applied, skipping", version);
            return Ok(false);
        }

        let conn = self.get_connection().await?;

        // Use transaction if configured
        if self.config.use_transactions {
            conn.execute(
                "BEGIN",
                params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to begin transaction")?;
        }

        // Apply the migration
        let apply_result = migration.up(&conn).await;

        match apply_result {
            Ok(_) => {
                let checksum = migration.checksum();
                let record_result = self
                    .record_migration_applied(&conn, migration, &checksum)
                    .await;

                // Commit or rollback based on results
                if self.config.use_transactions {
                    if let Err(e) = record_result {
                        conn.execute(
                            "ROLLBACK",
                            params_from_iter(std::iter::empty::<libsql::Value>()),
                        )
                        .await
                        .context("Failed to rollback transaction")?;
                        return Err(e.context("Failed to record migration as applied"));
                    }
                    conn.execute(
                        "COMMIT",
                        params_from_iter(std::iter::empty::<libsql::Value>()),
                    )
                    .await
                    .context("Failed to commit transaction")?;
                } else {
                    record_result?;
                }
            }
            Err(e) => {
                // If application failed, we must rollback the transaction
                if self.config.use_transactions {
                    let _ = conn
                        .execute(
                            "ROLLBACK",
                            params_from_iter(std::iter::empty::<libsql::Value>()),
                        )
                        .await;
                }
                return Err(e);
            }
        }

        tracing::info!("Applied migration: {}", version);

        Ok(true)
    }

    /// Runs a single migration down (rollback).
    ///
    /// # Arguments
    ///
    /// * `migration` - The migration to rollback
    ///
    /// # Returns
    ///
    /// `Ok(true)` if migration was rolled back, `Ok(false)` if not applied,
    /// or an error.
    pub async fn migrate_down(&self, migration: &dyn Migration) -> Result<bool> {
        let version = migration.version();
        let is_applied = self.is_migration_applied(version).await?;

        if !is_applied {
            tracing::debug!("Migration {} not applied, cannot rollback", version);
            return Ok(false);
        }

        let conn = self.get_connection().await?;

        // Use transaction if configured
        if self.config.use_transactions {
            conn.execute(
                "BEGIN",
                params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to begin transaction")?;
        }

        // Rollback the migration
        let rollback_result = migration.down(&conn).await;

        match rollback_result {
            Ok(_) => {
                let record_result = self.record_migration_rolled_back(&conn, version).await;

                // Commit or rollback based on results
                if self.config.use_transactions {
                    if let Err(e) = record_result {
                        conn.execute(
                            "ROLLBACK",
                            params_from_iter(std::iter::empty::<libsql::Value>()),
                        )
                        .await
                        .context("Failed to rollback transaction")?;
                        return Err(e.context("Failed to record migration as rolled back"));
                    }
                    conn.execute(
                        "COMMIT",
                        params_from_iter(std::iter::empty::<libsql::Value>()),
                    )
                    .await
                    .context("Failed to commit transaction")?;
                } else {
                    record_result?;
                }
            }
            Err(e) => {
                // If rollback failed, we must rollback the transaction
                if self.config.use_transactions {
                    let _ = conn
                        .execute(
                            "ROLLBACK",
                            params_from_iter(std::iter::empty::<libsql::Value>()),
                        )
                        .await;
                }
                return Err(e);
            }
        }

        tracing::info!("Rolled back migration: {}", version);

        Ok(true)
    }

    /// Runs all provided migrations in order.
    ///
    /// Migrations are sorted by version string to ensure consistent ordering.
    /// Already applied migrations are skipped unless `force` is set.
    ///
    /// # Arguments
    ///
    /// * `migrations` - Slice of migrations to run
    /// * `force` - Whether to force re-application of already applied migrations
    ///
    /// # Returns
    ///
    /// Number of migrations applied, or an error.
    pub async fn run_migrations(
        &self,
        migrations: &[&dyn Migration],
        force: bool,
    ) -> Result<usize> {
        // Initialize migrations table if needed
        if self.config.create_table_if_missing {
            self.initialize().await?;
        }

        // Sort migrations by version for consistent ordering
        let mut sorted_migrations: Vec<&dyn Migration> = migrations.to_vec();
        sorted_migrations.sort_by(|a, b| a.version().cmp(b.version()));

        let mut applied_count = 0;

        for migration in sorted_migrations.iter() {
            let was_applied = self.migrate_up(*migration, force).await?;

            if was_applied {
                applied_count += 1;
            }
        }

        Ok(applied_count)
    }

    /// Rolls back all migrations in reverse order.
    ///
    /// Only rolls back migrations that have been applied.
    ///
    /// # Arguments
    ///
    /// * `migrations` - Slice of migrations to potentially rollback
    /// * `count` - Number of migrations to rollback (None for all)
    ///
    /// # Returns
    ///
    /// Number of migrations rolled back, or an error.
    pub async fn rollback_migrations(
        &self,
        migrations: &[&dyn Migration],
        count: Option<usize>,
    ) -> Result<usize> {
        // Sort migrations by version for consistent ordering (reverse)
        let mut sorted_migrations: Vec<&dyn Migration> = migrations.to_vec();
        sorted_migrations.sort_by(|a, b| b.version().cmp(a.version()));

        let limit = count.unwrap_or(usize::MAX);
        let mut rolled_back_count = 0;

        for migration in sorted_migrations.iter() {
            if rolled_back_count >= limit {
                break;
            }

            let was_rolled_back = self.migrate_down(*migration).await?;

            if was_rolled_back {
                rolled_back_count += 1;
            }
        }

        Ok(rolled_back_count)
    }

    /// Gets the current migration status.
    ///
    /// # Returns
    ///
    /// A `MigrationStatus` with counts and details, or an error.
    pub async fn get_status(&self) -> Result<MigrationStatus> {
        // Initialize if needed
        if self.config.create_table_if_missing {
            self.initialize().await?;
        }

        let records = self.get_migration_records().await?;

        let applied_count = records
            .iter()
            .filter(|r| r.state == MigrationState::Applied)
            .count();

        let pending_count = records
            .iter()
            .filter(|r| r.state == MigrationState::Pending)
            .count();

        let rolled_back_count = records
            .iter()
            .filter(|r| r.state == MigrationState::RolledBack)
            .count();

        Ok(MigrationStatus {
            applied_count,
            pending_count,
            rolled_back_count,
            records,
        })
    }

    /// Gets the latest applied migration version.
    ///
    /// # Returns
    ///
    /// The latest version string, or None if no migrations applied.
    pub async fn get_latest_version(&self) -> Result<Option<String>> {
        let table_name = &self.config.migrations_table;
        let conn = self.get_connection().await?;

        let sql = format!(
            r#"
            SELECT version FROM {}
            WHERE state = 'APPLIED'
            ORDER BY applied_at DESC, version DESC
            LIMIT 1
            "#,
            table_name
        );

        let mut rows = conn
            .query(&sql, libsql::params_from_iter(std::iter::empty::<&str>()))
            .await
            .with_context(|| format!("Failed to get latest version from table: {}", table_name))?;

        if let Some(row) = rows.next().await? {
            let version: String = row.get(0)?;
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    /// Verifies migration integrity by checking checksums.
    ///
    /// # Arguments
    ///
    /// * `migrations` - Slice of migrations to verify
    ///
    /// # Returns
    ///
    /// Vector of versions with mismatched checksums, or an error.
    pub async fn verify_migrations(&self, migrations: &[&dyn Migration]) -> Result<Vec<String>> {
        let table_name = &self.config.migrations_table;
        let conn = self.get_connection().await?;

        let sql = format!(
            r#"
            SELECT version, checksum FROM {}
            WHERE state = 'APPLIED'
            "#,
            table_name
        );

        let mut rows = conn
            .query(&sql, libsql::params_from_iter(std::iter::empty::<&str>()))
            .await
            .with_context(|| format!("Failed to verify migrations in table: {}", table_name))?;
        let mut mismatches = Vec::new();

        while let Some(row) = rows.next().await? {
            let version: String = row.get(0)?;
            let stored_checksum: Option<String> = row.get(1)?;

            // Find the migration with this version
            if let Some(migration) = migrations.iter().find(|m| m.version() == version) {
                let expected_checksum = migration.checksum();

                if stored_checksum.as_ref() != Some(&expected_checksum) {
                    mismatches.push(version.clone());
                    tracing::warn!(
                        "Checksum mismatch for migration {}: stored={:?}, expected={}",
                        version,
                        stored_checksum,
                        expected_checksum
                    );
                }
            }
        }

        Ok(mismatches)
    }
}

/// Represents the current status of migrations.
///
/// This struct contains counts and details about applied, pending,
/// and rolled back migrations.
#[derive(Debug)]
pub struct MigrationStatus {
    /// Number of successfully applied migrations
    pub applied_count: usize,
    /// Number of pending migrations
    pub pending_count: usize,
    /// Number of rolled back migrations
    pub rolled_back_count: usize,
    /// All migration records
    pub records: Vec<MigrationRecord>,
}

impl MigrationStatus {
    /// Returns the total number of tracked migrations.
    ///
    /// # Returns
    ///
    /// Total count of all migrations (sum of applied, pending, and rolled_back).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.applied_count + self.pending_count + self.rolled_back_count
    }

    /// Returns whether all migrations are applied.
    ///
    /// # Returns
    ///
    /// `true` if there are no pending migrations.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.pending_count == 0
    }
}

/// Sorts migrations by version in ascending order.
///
/// # Arguments
///
/// * `migrations` - Slice of migrations to sort
///
/// # Returns
///
/// A vector of migrations sorted by version.
pub fn sort_migrations_by_version<'a>(migrations: &[&'a dyn Migration]) -> Vec<&'a dyn Migration> {
    let mut sorted = migrations.to_vec();
    sorted.sort_by(|a, b| a.version().cmp(b.version()));
    sorted
}

// ============================================================================
// SQLite/Tantivy/DuckDB → Turso Migrations
// ============================================================================

/// Migration: Create base Turso schema (migrates from SQLite)
///
/// This migration creates all the base tables needed for Maestro's OLTP operations,
/// migrating from the existing SQLite schema.
#[derive(Debug)]
pub struct CreateBaseSchema;

#[async_trait::async_trait]
impl Migration for CreateBaseSchema {
    fn version(&self) -> &str {
        "2026_01_19_001_create_base_schema"
    }

    fn description(&self) -> String {
        "Create base Turso schema (migrate from SQLite)".to_string()
    }

    async fn up(&self, conn: &Connection) -> Result<()> {
        // LSP servers table
        conn.execute(
            r#"
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
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT,
                UNIQUE(session_id, lsp_name)
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_lsp_session ON lsp_servers(session_id)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_lsp_status ON lsp_servers(status)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Sessions table
        conn.execute(
            r#"
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
                started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT,
                last_accessed_at TEXT,
                ended_at TEXT,
                metadata TEXT
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_path ON sessions(project_path)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_group_sort ON sessions(group_path, sort_order)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Projects table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS maestro_projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL UNIQUE,
                project_name TEXT NOT NULL,
                description TEXT,
                project_type TEXT,
                tech_stack TEXT,
                is_active INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT,
                last_scanned_at TEXT
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_path ON maestro_projects(project_path)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_active ON maestro_projects(is_active)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Memories table
        conn.execute(
            r#"
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
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                expires_at TEXT,
                last_accessed TEXT,
                meta_data TEXT,
                tags TEXT
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Create memory indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_expires ON memories(expires_at)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Tracks table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS maestro_tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                project_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'new',
                total_tasks INTEGER DEFAULT 0,
                completed_tasks INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT,
                UNIQUE(project_id, track_id),
                FOREIGN KEY (project_id) REFERENCES maestro_projects(id) ON DELETE CASCADE
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tracks_project ON maestro_tracks(project_id)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tracks_status ON maestro_tracks(status)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Session groups table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS session_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                category TEXT,
                is_expanded INTEGER DEFAULT 1,
                sort_order INTEGER DEFAULT 0,
                parent_id INTEGER REFERENCES session_groups(id) ON DELETE CASCADE
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_groups_path ON session_groups(path)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // MCP servers table
        conn.execute(
            r#"
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
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_mcp_status ON mcp_servers(status)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_mcp_transport ON mcp_servers(transport)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, conn: &Connection) -> Result<()> {
        // Drop tables in reverse order of creation (to handle foreign keys)
        conn.execute(
            "DROP TABLE IF EXISTS mcp_servers",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "DROP TABLE IF EXISTS session_groups",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "DROP TABLE IF EXISTS maestro_tracks",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "DROP TABLE IF EXISTS memories",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "DROP TABLE IF EXISTS maestro_projects",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "DROP TABLE IF EXISTS sessions",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        conn.execute(
            "DROP TABLE IF EXISTS lsp_servers",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        Ok(())
    }
}

/// Migration: Create FTS5 full-text search (migrates from Tantivy)
///
/// This migration creates the FTS5 virtual table for full-text search,
/// replacing Tantivy indexes.
#[derive(Debug)]
pub struct CreateFTS5Indexes;

#[async_trait::async_trait]
impl Migration for CreateFTS5Indexes {
    fn version(&self) -> &str {
        "2026_01_19_002_create_fts5_indexes"
    }

    fn description(&self) -> String {
        "Create FTS5 full-text search indexes (migrate from Tantivy)".to_string()
    }

    async fn up(&self, conn: &Connection) -> Result<()> {
        // Create FTS5 virtual table for memories
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(content, category)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        // Populate FTS5 index with existing memories
        conn.execute(
            r#"
            INSERT INTO memories_fts(rowid, content, category)
            SELECT id, content, category FROM memories
            WHERE id NOT IN (SELECT rowid FROM memories_fts)
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "DROP TABLE IF EXISTS memories_fts",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        Ok(())
    }
}

/// Migration: Create OLAP analytical views (migrates from DuckDB)
///
/// This migration creates optimized views and queries for OLAP operations,
/// replacing DuckDB analytical views.
#[derive(Debug)]
pub struct CreateOLAPViews;

#[async_trait::async_trait]
impl Migration for CreateOLAPViews {
    fn version(&self) -> &str {
        "2026_01_19_003_create_olap_views"
    }

    fn description(&self) -> String {
        "Create OLAP analytical views (migrate from DuckDB)".to_string()
    }

    async fn up(&self, conn: &Connection) -> Result<()> {
        // Note: Turso uses native SQL queries instead of materialized views
        // The OLAP operations are implemented as methods in TursoStorageBackend
        // This migration is a placeholder for any view-specific optimizations
        // that might be added in the future

        // Create a view for active sessions with project info
        conn.execute(
            r#"
            CREATE VIEW IF NOT EXISTS v_active_sessions AS
            SELECT
                s.id,
                s.session_id,
                s.title,
                s.project_path,
                s.status,
                s.started_at,
                s.updated_at,
                s.last_accessed_at,
                p.project_name,
                p.tech_stack
            FROM sessions s
            LEFT JOIN maestro_projects p ON s.project_path = p.project_path
            WHERE s.status = 'running' AND s.ended_at IS NULL
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "DROP VIEW IF EXISTS v_active_sessions",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        Ok(())
    }
}

/// Migration: Add use_proxy column to lsp_servers table
///
/// This migration adds the `use_proxy` column to the lsp_servers table,
/// which tracks whether stdio proxy mode is enabled for an LSP.
#[derive(Debug)]
pub struct AddLspUseProxyColumn;

#[async_trait::async_trait]
impl Migration for AddLspUseProxyColumn {
    fn version(&self) -> &str {
        "2026_01_22_001_add_lsp_use_proxy_column"
    }

    fn description(&self) -> String {
        "Add use_proxy column to lsp_servers table".to_string()
    }

    async fn up(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
            ALTER TABLE lsp_servers ADD COLUMN use_proxy INTEGER DEFAULT 0
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;
        Ok(())
    }

    async fn down(&self, conn: &Connection) -> Result<()> {
        // SQLite doesn't support dropping columns, so we need to recreate the table
        conn.execute(
            r#"
            CREATE TABLE lsp_servers_new (
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
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT,
                UNIQUE(session_id, lsp_name)
            )
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            r#"
            INSERT INTO lsp_servers_new (id, session_id, language, lsp_name, status, pid, port,
                auto_start, last_started, last_error, created_at, updated_at)
            SELECT id, session_id, language, lsp_name, status, pid, port,
                auto_start, last_started, last_error, created_at, updated_at
            FROM lsp_servers
            "#,
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "DROP TABLE lsp_servers",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "ALTER TABLE lsp_servers_new RENAME TO lsp_servers",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_lsp_session ON lsp_servers(session_id)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_lsp_status ON lsp_servers(status)",
            params_from_iter(std::iter::empty::<&str>()),
        )
        .await?;

        Ok(())
    }
}

/// Get all migrations for the LSP integration track.
///
/// Returns a vector of migrations that should be applied to migrate
/// from SQLite/DuckDB/Tantivy to Turso.
pub fn get_lsp_integration_migrations() -> Vec<std::sync::Arc<dyn Migration + Send + Sync>> {
    vec![
        std::sync::Arc::new(CreateBaseSchema) as std::sync::Arc<dyn Migration + Send + Sync>,
        std::sync::Arc::new(CreateFTS5Indexes) as std::sync::Arc<dyn Migration + Send + Sync>,
        std::sync::Arc::new(CreateOLAPViews) as std::sync::Arc<dyn Migration + Send + Sync>,
        std::sync::Arc::new(AddLspUseProxyColumn) as std::sync::Arc<dyn Migration + Send + Sync>,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a temporary in-memory database for testing.
    async fn create_test_db() -> Result<Arc<Database>> {
        // Use a temporary file database since in-memory databases don't share schema
        // across connections in libsql
        let temp_dir = tempfile::TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        // CRITICAL: Use explicit URI with ?threaded=1 for consistent threading
        let db_uri = format!("file:{}?threaded=1", db_path.display());
        let db = libsql::Builder::new_local(&db_uri).build().await?;
        // Keep temp_dir alive by leaking it - the OS will clean up on process exit
        std::mem::forget(temp_dir);
        Ok(Arc::new(db))
    }

    /// Test migration for unit tests.
    #[derive(Debug)]
    struct TestMigration {
        version: &'static str,
        up_sql: &'static str,
        down_sql: &'static str,
    }

    impl TestMigration {
        const fn new(version: &'static str, up_sql: &'static str, down_sql: &'static str) -> Self {
            Self {
                version,
                up_sql,
                down_sql,
            }
        }
    }

    #[async_trait::async_trait]
    impl Migration for TestMigration {
        fn version(&self) -> &str {
            self.version
        }

        async fn up(&self, conn: &Connection) -> Result<()> {
            if !self.up_sql.is_empty() {
                conn.execute(self.up_sql, params_from_iter(std::iter::empty::<&str>()))
                    .await?;
            }
            Ok(())
        }

        async fn down(&self, conn: &Connection) -> Result<()> {
            if !self.down_sql.is_empty() {
                conn.execute(self.down_sql, params_from_iter(std::iter::empty::<&str>()))
                    .await?;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_migration_manager_initialization() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);

        // Initialize should create the migrations table
        manager.initialize().await?;

        // Verify the table exists by checking status
        let status = manager.get_status().await?;
        assert_eq!(status.applied_count, 0);
        assert_eq!(status.pending_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_run_single_migration() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        let migration = TestMigration::new(
            "2024_01_15_001_create_users",
            "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)",
            "DROP TABLE users",
        );

        let applied = manager.run_migrations(&[&migration], false).await?;

        assert_eq!(applied, 1);

        // Verify migration is recorded
        let status = manager.get_status().await?;
        assert_eq!(status.applied_count, 1);
        assert!(status.is_complete());

        Ok(())
    }

    #[tokio::test]
    async fn test_skip_already_applied_migration() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        let migration = TestMigration::new("2024_01_15_001_create_users", "", "");

        // Run twice
        manager.run_migrations(&[&migration], false).await?;
        let count = manager.run_migrations(&[&migration], false).await?;

        // Should only apply once (second run skips)
        assert_eq!(count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_rollback_migration() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        let migration = TestMigration::new(
            "2024_01_15_001_create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "DROP TABLE users",
        );

        // Apply migration
        manager.run_migrations(&[&migration], false).await?;

        // Rollback migration
        let rolled_back = manager.rollback_migrations(&[&migration], None).await?;
        assert_eq!(rolled_back, 1);

        // Verify rollback is recorded
        let status = manager.get_status().await?;
        assert_eq!(status.rolled_back_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_migrations_ordered() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        let migration_v2 = TestMigration::new("2024_01_15_002_add_email", "", "");
        let migration_v1 = TestMigration::new("2024_01_15_001_create_users", "", "");

        // Run in random order
        let applied = manager
            .run_migrations(&[&migration_v2, &migration_v1], false)
            .await?;

        // Verify both applied
        assert_eq!(applied, 2);

        // Verify order (v1 should be applied before v2)
        let records = manager.get_migration_records().await?;
        assert_eq!(records[0].version, "2024_01_15_001_create_users");
        assert_eq!(records[1].version, "2024_01_15_002_add_email");

        Ok(())
    }

    #[tokio::test]
    async fn test_migration_checksum_verification() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        let migration = TestMigration::new("2024_01_15_001_create_users", "", "");

        manager.run_migrations(&[&migration], false).await?;

        // Verify should pass
        let mismatches = manager.verify_migrations(&[&migration]).await?;
        assert!(mismatches.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_latest_version() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        let migration_v1 = TestMigration::new("2024_01_15_001_create_users", "", "");
        let migration_v2 = TestMigration::new("2024_01_15_002_add_email", "", "");

        manager
            .run_migrations(&[&migration_v1, &migration_v2], false)
            .await?;

        let latest = manager.get_latest_version().await?;
        assert_eq!(latest, Some("2024_01_15_002_add_email".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_migration_record_creation() {
        let mut record = MigrationRecord::new_pending("2024_01_15_001".to_string());

        assert_eq!(record.state, MigrationState::Pending);
        assert!(record.applied_at.is_none());

        let _ = record.mark_applied(Some("checksum123".to_string()));

        assert_eq!(record.state, MigrationState::Applied);
        assert!(record.applied_at.is_some());
        assert_eq!(record.checksum, Some("checksum123".to_string()));

        let _ = record.mark_rolled_back();

        assert_eq!(record.state, MigrationState::RolledBack);
        assert!(record.rolled_back_at.is_some());
    }

    #[tokio::test]
    async fn test_migration_state_display() {
        assert_eq!(MigrationState::Pending.to_string(), "PENDING");
        assert_eq!(MigrationState::Applied.to_string(), "APPLIED");
        assert_eq!(MigrationState::RolledBack.to_string(), "ROLLED_BACK");
    }

    #[tokio::test]
    async fn test_migration_config_defaults() {
        let config = MigrationConfig::default();

        assert_eq!(config.migrations_table, "schema_migrations");
        assert!(config.create_table_if_missing);
    }

    #[tokio::test]
    async fn test_migration_status_counts() {
        let status = MigrationStatus {
            applied_count: 5,
            pending_count: 2,
            rolled_back_count: 1,
            records: Vec::new(),
        };

        assert_eq!(status.total_count(), 8);
        assert!(!status.is_complete());
    }

    #[tokio::test]
    async fn test_validate_table_name_valid() {
        // Valid table names
        assert!(validate_table_name("schema_migrations").is_ok());
        assert!(validate_table_name("my_table").is_ok());
        assert!(validate_table_name("_private_table").is_ok());
        assert!(validate_table_name("Table123").is_ok());
    }

    #[tokio::test]
    async fn test_validate_table_name_invalid() {
        // Empty table name
        assert!(validate_table_name("").is_err());

        // SQL injection patterns
        assert!(validate_table_name("table; DROP TABLE users; --").is_err());
        assert!(validate_table_name("table--comment").is_err());
        assert!(validate_table_name("table/*comment*/").is_err());

        // Invalid characters
        assert!(validate_table_name("table with spaces").is_err());
        assert!(validate_table_name("table-with-dashes").is_err());
        assert!(validate_table_name("table.with.dots").is_err());

        // Must start with letter or underscore
        assert!(validate_table_name("1table").is_err());
        assert!(validate_table_name("9table").is_err());
    }

    #[tokio::test]
    async fn test_migration_config_validation() {
        // Valid table names
        assert!(MigrationConfig::new("valid_table".to_string()).is_ok());
        assert!(MigrationConfig::new("_private".to_string()).is_ok());

        // Invalid table names
        assert!(MigrationConfig::new("invalid table".to_string()).is_err());
        assert!(MigrationConfig::new("table;drop".to_string()).is_err());
        assert!(MigrationConfig::new("".to_string()).is_err());
    }

    #[tokio::test]
    async fn test_migration_config_builder_pattern() {
        let config = MigrationConfig::new("my_migrations".to_string())
            .unwrap()
            .with_max_concurrent(4)
            .with_transactions(false);

        assert_eq!(config.migrations_table, "my_migrations");
        assert_eq!(config.max_concurrent_migrations, 4);
        assert!(!config.use_transactions);
    }

    #[tokio::test]
    async fn test_migration_config_max_concurrent_minimum() {
        let config = MigrationConfig::new("test".to_string())
            .unwrap()
            .with_max_concurrent(0);

        // Should enforce minimum of 1
        assert_eq!(config.max_concurrent_migrations, 1);
    }

    #[tokio::test]
    async fn test_migration_config_default_values() {
        let config = MigrationConfig::default();

        assert_eq!(config.migrations_table, "schema_migrations");
        assert!(config.create_table_if_missing);
        assert_eq!(config.max_concurrent_migrations, 1);
        assert!(config.use_transactions);
    }

    #[tokio::test]
    async fn test_lsp_integration_migrations() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        // Create migration instances
        let base_schema = CreateBaseSchema;
        let fts5_indexes = CreateFTS5Indexes;
        let olap_views = CreateOLAPViews;
        let use_proxy = AddLspUseProxyColumn;

        // Run all migrations
        let migrations: Vec<&dyn Migration> = vec![&base_schema, &fts5_indexes, &olap_views, &use_proxy];
        let applied_count = manager.run_migrations(&migrations, false).await?;
        assert_eq!(applied_count, 4, "Should apply 4 migrations");

        // Verify all migrations were applied
        let status = manager.get_status().await?;
        assert_eq!(status.applied_count, 4);
        assert!(status.is_complete());

        // Verify migrations can be rolled back
        let rolled_back = manager.rollback_migrations(&migrations, Some(1)).await?;
        assert_eq!(rolled_back, 1, "Should rollback 1 migration");

        Ok(())
    }

    #[tokio::test]
    async fn test_base_schema_migration() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        // Apply base schema migration
        let applied = manager.run_migrations(&[&CreateBaseSchema], false).await?;
        assert_eq!(applied, 1);

        // Verify tables were created
        let conn = manager.get_connection().await?;

        // Check that lsp_servers table exists
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='lsp_servers'",
                params_from_iter(std::iter::empty::<&str>()),
            )
            .await?;
        assert!(
            rows.next().await?.is_some(),
            "lsp_servers table should exist"
        );

        // Check that memories table exists
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='memories'",
                params_from_iter(std::iter::empty::<&str>()),
            )
            .await?;
        assert!(rows.next().await?.is_some(), "memories table should exist");

        // Check that maestro_projects table exists
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='maestro_projects'",
                params_from_iter(std::iter::empty::<&str>()),
            )
            .await?;
        assert!(
            rows.next().await?.is_some(),
            "maestro_projects table should exist"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_fts5_migration() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        // Apply base schema first (needed for memories table)
        manager.run_migrations(&[&CreateBaseSchema], false).await?;

        // Apply FTS5 migration
        let applied = manager.run_migrations(&[&CreateFTS5Indexes], false).await?;
        assert_eq!(applied, 1);

        // Verify FTS5 table was created
        let conn = manager.get_connection().await?;
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='memories_fts'",
                params_from_iter(std::iter::empty::<&str>()),
            )
            .await?;
        assert!(
            rows.next().await?.is_some(),
            "memories_fts table should exist"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_olap_views_migration() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        // Apply base schema first
        manager.run_migrations(&[&CreateBaseSchema], false).await?;

        // Apply OLAP views migration
        let applied = manager.run_migrations(&[&CreateOLAPViews], false).await?;
        assert_eq!(applied, 1);

        // Verify view was created
        let conn = manager.get_connection().await?;
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='view' AND name='v_active_sessions'",
                params_from_iter(std::iter::empty::<&str>()),
            )
            .await?;
        assert!(
            rows.next().await?.is_some(),
            "v_active_sessions view should exist"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_migration_idempotency() -> Result<()> {
        let db = create_test_db().await?;
        let manager = MigrationManager::new(db);
        manager.initialize().await?;

        // Create migration instances
        let base_schema = CreateBaseSchema;
        let fts5_indexes = CreateFTS5Indexes;
        let olap_views = CreateOLAPViews;

        let migrations: Vec<&dyn Migration> = vec![&base_schema, &fts5_indexes, &olap_views];

        // Run migrations twice
        let count1 = manager.run_migrations(&migrations, false).await?;
        let count2 = manager.run_migrations(&migrations, false).await?;

        // Second run should apply 0 migrations (all already applied)
        assert_eq!(count1, 3, "First run should apply 3 migrations");
        assert_eq!(
            count2, 0,
            "Second run should apply 0 migrations (idempotent)"
        );

        Ok(())
    }
}
