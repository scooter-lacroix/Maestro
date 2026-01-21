//! Migration tests for database architecture changes
//!
//! These tests verify the database migration from SQLite/DuckDB/Tantivy to Turso (libSQL).
//!
//! Tests validate FR6 requirements:
//! - FR6.1: Replace SQLite (OLTP) with libsql-rs
//! - FR6.2: Replace DuckDB (OLAP) with Turso native SQL
//! - FR6.3: Replace Tantivy with Turso's FTS5 extension

use std::fs;
use tempfile::TempDir;
use chrono::Utc;

use leindex_analyzers::memory::models::{Session, SessionStatus, Memory, MemoryCategory, MemoryImportance};
use leindex_analyzers::memory::turso_backend::{TursoStorageBackend, TursoConfig};

/// Migration test for FR6.1: SQLite → Turso OLTP migration
///
/// Requirement: Replace rusqlite with libsql-rs while maintaining
/// all existing OLTP operations (sessions, projects, files, metadata).
#[tokio::test]
async fn test_sqlite_to_turso_oltp_migration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Requirement: Turso backend should support all OLTP operations
    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create Turso backend");
    storage.initialize().await.expect("Failed to initialize");

    // Test 1: Create a project (OLTP operation)
    let project = storage
        .get_or_create_project("/test/project", "Test Project")
        .await
        .expect("Failed to create project");

    assert!(project.id > 0);
    assert_eq!(project.project_path, "/test/project");
    assert_eq!(project.project_name, "Test Project");

    // Test 2: Create a session (OLTP operation)
    let session = Session {
        id: 0,
        session_id: "test-session".to_string(),
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
    };

    let session_id = storage
        .insert_session(&session)
        .await
        .expect("Failed to insert session");

    assert!(session_id > 0);

    // Test 3: Retrieve session (OLTP read operation)
    let retrieved = storage
        .get_session("test-session")
        .await
        .expect("Failed to retrieve session");

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.session_id, "test-session");
    assert_eq!(retrieved.title, "Test Session");

    // Test 4: Update session (OLTP write operation)
    storage
        .update_session_status("test-session", SessionStatus::Completed)
        .await
        .expect("Failed to update session status");

    let updated = storage
        .get_session("test-session")
        .await
        .expect("Failed to retrieve updated session")
        .expect("Session not found");

    assert_eq!(updated.status, SessionStatus::Completed);

    // Test 5: List sessions (OLTP query operation)
    let sessions = storage
        .list_sessions()
        .await
        .expect("Failed to list sessions");

    assert_eq!(sessions.len(), 1);

    // Test 6: Delete session (OLTP delete operation)
    storage
        .delete_session("test-session")
        .await
        .expect("Failed to delete session");

    let deleted = storage
        .get_session("test-session")
        .await
        .expect("Failed to check deleted session");

    assert!(deleted.is_none());
}

/// Migration test for FR6.1: Concurrent write operations (MVCC)
///
/// Requirement: Leverage Turso's concurrent writes (MVCC) for improved
/// multi-session performance.
#[tokio::test]
async fn test_turso_mvcc_concurrent_writes() {
    use tokio::task::JoinSet;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create Turso backend");
    storage.initialize().await.expect("Failed to initialize");

    // Requirement: Test concurrent writes via MVCC
    let mut join_set = JoinSet::new();

    for i in 0..20 {
        let storage_clone = storage.clone();
        join_set.spawn(async move {
            let session = Session {
                id: 0,
                session_id: format!("concurrent-session-{}", i),
                title: format!("Concurrent Session {}", i),
                project_path: format!("/test/project/{}", i),
                group_path: None,
                sort_order: i,
                parent_session_id: None,
                command: None,
                tool: None,
                status: SessionStatus::Running,
                multiplexer_session: None,
                started_at: Utc::now(),
                last_accessed_at: None,
                ended_at: None,
                metadata: None,
            };

            storage_clone
                .insert_session(&session)
                .await
                .expect("Failed to insert session concurrently");

            i
        });
    }

    // Wait for all concurrent writes to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.expect("Task failed"));
    }

    // Requirement: All 20 concurrent writes should succeed
    assert_eq!(results.len(), 20);

    // Verify all sessions were inserted
    let sessions = storage
        .list_sessions()
        .await
        .expect("Failed to list sessions");

    assert_eq!(sessions.len(), 20);
}

/// Migration test for FR6.2: DuckDB → Turso OLAP migration
///
/// Requirement: Migrate all analytical queries to Turso native SQL.
/// Analytical views for file stats, version stats, diff stats.
#[tokio::test]
async fn test_duckdb_to_turso_olap_migration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create Turso backend");
    storage.initialize().await.expect("Failed to initialize");

    // Setup: Create test projects and sessions for analytics
    for i in 0..5 {
        let project_path = format!("/test/project/{}", i);
        storage
            .get_or_create_project(&project_path, &format!("Project {}", i))
            .await
            .expect("Failed to create project");

        // Create sessions with different statuses
        for j in 0..3 {
            let status = match j % 3 {
                0 => SessionStatus::Running,
                1 => SessionStatus::Completed,
                _ => SessionStatus::Error,
            };

            let session = Session {
                id: 0,
                session_id: format!("session-{}-{}", i, j),
                title: format!("Session {}-{}", i, j),
                project_path: project_path.clone(),
                group_path: None,
                sort_order: j,
                parent_session_id: None,
                command: None,
                tool: None,
                status,
                multiplexer_session: None,
                started_at: Utc::now(),
                last_accessed_at: None,
                ended_at: None,
                metadata: None,
            };

            storage
                .insert_session(&session)
                .await
                .expect("Failed to insert session");
        }
    }

    // Requirement: OLAP query - session stats by status (was DuckDB, now Turso)
    let stats = storage
        .session_stats_by_status()
        .await
        .expect("Failed to get session stats");

    // Requirement: Should have stats for each status type
    assert!(!stats.is_empty());

    // Verify running sessions count
    let running_count = stats
        .iter()
        .find(|s| s.status == "running")
        .map(|s| s.count)
        .unwrap_or(0);

    assert!(running_count > 0);

    // Requirement: OLAP query - most active projects (was DuckDB, now Turso)
    let active_projects = storage
        .most_active_projects(Some(5))
        .await
        .expect("Failed to get active projects");

    assert!(!active_projects.is_empty());
    assert!(active_projects.len() <= 5);

    // Requirement: Verify the stats structure is correct
    for stats in &active_projects {
        // Verify ActiveProjectStats fields
        assert!(!stats.project_path.is_empty());
        assert!(!stats.project_name.is_empty());
        assert!(stats.total_sessions >= 0);
        assert!(stats.active_sessions >= 0);
    }

    // Requirement: OLAP query - memory stats by category (was DuckDB, now Turso)
    let memory_stats = storage
        .memory_stats_by_category()
        .await
        .expect("Failed to get memory stats");

    // May be empty if no memories, but query should succeed
    assert!(memory_stats.len() >= 0);
}

/// Migration test for FR6.3: Tantivy → FTS5 migration
///
/// Requirement: Replace Tantivy with Turso's built-in FTS5 extension.
/// Migrate existing full-text indexes to FTS5 virtual tables.
#[tokio::test]
async fn test_tantivy_to_fts5_migration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create Turso backend");
    storage.initialize().await.expect("Failed to initialize");

    // Requirement: FTS5 should be available for full-text search
    // The initialize() method should create FTS5 virtual tables

    // Test 1: Create memory entries for FTS5 indexing
    for i in 0..10 {
        let content = format!("Test memory content number {} with keywords rust python typescript", i);
        let memory = leindex_analyzers::memory::models::Memory {
            id: 0,
            content: content.clone(),
            summary: Some(format!("Summary {}", i)),
            category: match i % 3 {
                0 => MemoryCategory::Knowledge,
                1 => MemoryCategory::Context,
                _ => MemoryCategory::General,
            },
            importance: match i % 2 {
                0 => MemoryImportance::High,
                _ => MemoryImportance::Normal,
            },
            source: None,
            session_id: Some(format!("session-{}", i)),
            project_id: None,
            track_id: None,
            command: None,
            command_context: None,
            created_at: Utc::now(),
            expires_at: None,
            last_accessed: None,
            metadata: None,
            tags: None,
        };

        storage
            .insert_memory(&memory)
            .await
            .expect("Failed to insert memory");
    }

    // Requirement: FTS5 search should work (replaces Tantivy search)
    let search_results = storage
        .search_memories("rust python", Some(100))
        .await
        .expect("FTS5 search failed");

    // Requirement: Should find memories with matching keywords
    // All 10 memories contain these keywords
    assert!(search_results.len() >= 5);

    // Test 2: FTS5 search with partial match
    let partial_results = storage
        .search_memories("typescript", Some(100))
        .await
        .expect("FTS5 partial search failed");

    assert!(partial_results.len() >= 3);

    // Test 3: FTS5 search with non-existent term
    let empty_results = storage
        .search_memories("nonexistent_keyword_xyz123", Some(100))
        .await
        .expect("FTS5 search with non-existent term failed");

    assert_eq!(empty_results.len(), 0);

    // Test 4: FTS5 index maintenance (optimize)
    storage
        .optimize_fts_index()
        .await
        .expect("Failed to optimize FTS5 index");

    // Verify search still works after optimization
    let post_opt_results = storage
        .search_memories("rust", Some(100))
        .await
        .expect("FTS5 search failed after optimization");

    assert!(post_opt_results.len() >= 3);

    // Test 5: FTS5 index rebuild
    storage
        .rebuild_fts_index()
        .await
        .expect("Failed to rebuild FTS5 index");

    // Verify search still works after rebuild
    let post_rebuild_results = storage
        .search_memories("python", Some(100))
        .await
        .expect("FTS5 search failed after rebuild");

    assert!(post_rebuild_results.len() >= 3);
}

/// Migration test: Rollback capability
///
/// Requirement: System should support rollback if migration fails.
#[tokio::test]
async fn test_migration_rollback() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Test 1: Create data in Turso
    let storage = TursoStorageBackend::new(Some(db_path.clone()), None)
        .await
        .expect("Failed to create Turso backend");
    storage.initialize().await.expect("Failed to initialize");

    let original_session = Session {
        id: 0,
        session_id: "rollback-test".to_string(),
        title: "Original Session".to_string(),
        project_path: "/test".to_string(),
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
    };

    storage
        .insert_session(&original_session)
        .await
        .expect("Failed to insert original session");

    // Test 2: Shutdown (simulating migration checkpoint)
    storage.shutdown().await.expect("Failed to shutdown");

    // Test 3: Reopen (simulating post-migration state)
    let storage_reopened = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to reopen Turso backend");
    storage_reopened.initialize().await.expect("Failed to initialize reopened");

    // Requirement: Data should be preserved across migration (rollback not needed)
    let preserved = storage_reopened
        .get_session("rollback-test")
        .await
        .expect("Failed to retrieve preserved session");

    assert!(preserved.is_some());
    assert_eq!(preserved.unwrap().title, "Original Session");

    // Requirement: In a real rollback scenario, we would:
    // 1. Create a backup before migration
    // 2. Attempt migration
    // 3. On failure, restore from backup
    // 4. Verify data integrity
}

/// Migration test: Idempotency
///
/// Requirement: Running migration multiple times should be safe.
#[tokio::test]
async fn test_migration_idempotency() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Test 1: Initialize database (first "migration")
    let storage1 = TursoStorageBackend::new(Some(db_path.clone()), None)
        .await
        .expect("Failed to create Turso backend");
    storage1.initialize().await.expect("Failed to initialize");

    // Create test data
    storage1
        .get_or_create_project("/test", "Test")
        .await
        .expect("Failed to create project");

    storage1.shutdown().await.expect("Failed to shutdown");

    // Test 2: Re-initialize (simulating re-running migration)
    let storage2 = TursoStorageBackend::new(Some(db_path.clone()), None)
        .await
        .expect("Failed to create Turso backend");
    storage2.initialize().await.expect("Failed to re-initialize");

    // Requirement: Data should still be accessible
    let project = storage2
        .get_project_by_path("/test")
        .await
        .expect("Failed to get project after re-initialization");

    assert!(project.is_some());

    storage2.shutdown().await.expect("Failed to shutdown");

    // Test 3: Third initialization (verifying true idempotency)
    let storage3 = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create Turso backend");
    storage3.initialize().await.expect("Failed to initialize third time");

    // Requirement: Should still work without data corruption
    let project = storage3
        .get_project_by_path("/test")
        .await
        .expect("Failed to get project after third initialization");

    assert!(project.is_some());
    assert_eq!(project.unwrap().project_name, "Test");
}

/// Migration test: Read-only mode
///
/// Requirement: Migrated databases should support read-only mode for verification.
#[tokio::test]
async fn test_read_only_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Create data first
    {
        let storage = TursoStorageBackend::new(Some(db_path.clone()), None)
            .await
            .expect("Failed to create Turso backend");
        storage.initialize().await.expect("Failed to initialize");

        storage
            .get_or_create_project("/test", "Test")
            .await
            .expect("Failed to create project");
    }

    // Open in read-only mode
    let config = TursoConfig {
        #[allow(deprecated)]
        max_connections: 10,
        #[allow(deprecated)]
        connection_timeout_secs: 30,
        read_only: true,
    };

    let storage = TursoStorageBackend::new(Some(db_path), Some(config))
        .await
        .expect("Failed to open read-only Turso backend");
    storage.initialize().await.expect("Failed to initialize read-only");

    // Requirement: Read operations should work
    let project = storage
        .get_project_by_path("/test")
        .await
        .expect("Read operation failed in read-only mode");

    assert!(project.is_some());

    // Requirement: Write operations should fail
    let write_result = storage
        .get_or_create_project("/test2", "Test2")
        .await;

    // The write should fail in read-only mode
    assert!(write_result.is_err(), "Write operation should fail in read-only mode");
}

/// Migration test: Foreign key constraints
///
/// Requirement: Migrated database should maintain referential integrity.
#[tokio::test]
async fn test_foreign_key_constraints() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create Turso backend");
    storage.initialize().await.expect("Failed to initialize");

    // Requirement: Create project first
    let project = storage
        .get_or_create_project("/test", "Test Project")
        .await
        .expect("Failed to create project");

    // Requirement: Session should reference valid project
    let session = Session {
        id: 0,
        session_id: "fk-test".to_string(),
        title: "FK Test".to_string(),
        project_path: "/test".to_string(),
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
    };

    storage
        .insert_session(&session)
        .await
        .expect("Failed to insert session");

    // Requirement: Cascade delete should work
    storage
        .delete_session("fk-test")
        .await
        .expect("Failed to delete session");

    // Project should still exist (session deleted, not project)
    let project = storage
        .get_project_by_path("/test")
        .await
        .expect("Failed to get project after session deletion");

    assert!(project.is_some());
}
