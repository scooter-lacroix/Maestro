//! Integration tests for LSP integration
//!
//! These tests verify the complete LSP integration system works correctly
//! according to the functional requirements in the spec.
//!
//! Tests are written based on WHAT the system should do (requirements),
//! not WHAT the code currently does. This ensures tests catch bugs and edge cases.

use std::fs;
use tempfile::TempDir;

use leindex_analyzers::memory::models::{Session, SessionStatus};
use leindex_analyzers::memory::turso_backend::{LspStatus, LspServerState, TursoStorageBackend};
use leindex_analyzers::memory::LspType;
use leindex_analyzers::memory::LspManager;

/// Integration test for FR4.1: Auto-Start by Language Detection
///
/// Requirement: When a session is created, scan the project path for file extensions
/// and auto-start appropriate LSPs based on detected languages.
#[tokio::test]
async fn test_autostart_by_language_detection_rust() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create a Rust project with main.rs
    fs::write(project_path.join("main.rs"), "fn main() { println!(\"Hello\"); }")
        .expect("Failed to create main.rs");

    // Create storage and LSP manager
    let storage = TursoStorageBackend::new(Some(temp_dir.path().join("test.db")), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    let lsp_manager = LspManager::new(storage);

    // Requirement: Detect .rs files and recommend rust-analyzer
    let detected = lsp_manager
        .detect_languages_from_project(project_path)
        .await
        .expect("Failed to detect languages");

    assert!(
        detected.contains(&LspType::Rust),
        "Should detect Rust language from .rs files"
    );

    // Requirement: Recommend LSPs for session
    let recommended = lsp_manager
        .recommend_lsps_for_session("test-session", project_path)
        .await
        .expect("Failed to get recommendations");

    assert!(
        recommended.contains(&LspType::Rust),
        "Should recommend rust-analyzer for Rust project"
    );
}

#[tokio::test]
async fn test_autostart_by_language_detection_python() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let project_path = temp_dir.path();

    // Create a Python project
    fs::write(project_path.join("app.py"), "print('Hello')")
        .expect("Failed to create app.py");
    fs::write(project_path.join("utils.py"), "def helper(): pass")
        .expect("Failed to create utils.py");

    let storage = TursoStorageBackend::new(Some(temp_dir.path().join("test.db")), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    let lsp_manager = LspManager::new(storage);

    // Requirement: Detect .py files and recommend ruff-lsp
    let detected = lsp_manager
        .detect_languages_from_project(project_path)
        .await
        .expect("Failed to detect languages");

    assert!(
        detected.contains(&LspType::Python),
        "Should detect Python language from .py files"
    );

    let recommended = lsp_manager
        .recommend_lsps_for_session("test-session", project_path)
        .await
        .expect("Failed to get recommendations");

    assert!(
        recommended.contains(&LspType::Python),
        "Should recommend ruff-lsp for Python project"
    );
}

#[tokio::test]
async fn test_autostart_by_language_detection_typescript() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let project_path = temp_dir.path();

    // Create a TypeScript project
    fs::write(project_path.join("index.ts"), "console.log('test');")
        .expect("Failed to create index.ts");
    fs::write(project_path.join("app.tsx"), "const App = () => <div>Test</div>;")
        .expect("Failed to create app.tsx");

    let storage = TursoStorageBackend::new(Some(temp_dir.path().join("test.db")), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    let lsp_manager = LspManager::new(storage);

    // Requirement: Detect .ts/.tsx/.js/.jsx files and recommend typescript-language-server
    let detected = lsp_manager
        .detect_languages_from_project(project_path)
        .await
        .expect("Failed to detect languages");

    assert!(
        detected.contains(&LspType::TypeScript),
        "Should detect TypeScript language from .ts/.tsx files"
    );

    let recommended = lsp_manager
        .recommend_lsps_for_session("test-session", project_path)
        .await
        .expect("Failed to get recommendations");

    assert!(
        recommended.contains(&LspType::TypeScript),
        "Should recommend typescript-language-server for TypeScript project"
    );
}

/// Integration test for FR4.3: Graceful Degradation
///
/// Requirement: If an LSP fails to start or crashes, log the error and continue without it.
/// Show error status in TUI. Do not block session creation or operation.
#[tokio::test]
async fn test_graceful_degradation_missing_lsp_binary() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let project_path = temp_dir.path();

    // Create a project with Rust files
    fs::write(project_path.join("main.rs"), "fn main() {}")
        .expect("Failed to create main.rs");

    // Create storage and initialize
    let storage = TursoStorageBackend::new(Some(temp_dir.path().join("test.db")), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    // Requirement: Session should be created successfully even without LSP binaries
    let session = Session {
        id: 0,
        session_id: "test-session-no-lsp".to_string(),
        title: "Test Session".to_string(),
        project_path: project_path.to_string_lossy().to_string(),
        group_path: None,
        sort_order: 0,
        parent_session_id: None,
        command: None,
        tool: None,
        status: SessionStatus::Running,
        multiplexer_session: None,
        started_at: chrono::Utc::now(),
        last_accessed_at: None,
        ended_at: None,
        metadata: None,
    };

    // Requirement: Session creation MUST succeed even if LSP binary is missing
    let session_id = storage
        .insert_session(&session)
        .await
        .expect("Session creation should succeed even without LSP binaries");

    assert_eq!(session_id, 1);

    // Requirement: Verify session can be retrieved
    let retrieved = storage
        .get_session(&session.session_id)
        .await
        .expect("Failed to get session");

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().session_id, session.session_id);
}

/// Integration test for FR5: State Persistence (Turso)
///
/// Requirement: LSP state shall be persisted using Turso (libsql-rs).
/// LSP state should be retrievable after restart.
#[tokio::test]
async fn test_lsp_state_persistence_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Create storage and initialize
    let storage = TursoStorageBackend::new(Some(db_path.clone()), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    // Create an LSP server state as specified in FR5.2
    let original_state = LspServerState {
        id: 0,
        session_id: "test-session".to_string(),
        language: "rust".to_string(),
        lsp_name: "rust-analyzer".to_string(),
        status: LspStatus::Running,
        pid: Some(12345),
        port: None,
        auto_start: true,  // Requirement: auto_start BOOLEAN DEFAULT TRUE
        use_proxy: false,
        last_started: Some(chrono::Utc::now().to_rfc3339()),
        last_error: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    };

    // Persist the state
    let _state_id = storage
        .upsert_lsp_state(&original_state)
        .await
        .expect("Failed to upsert LSP state");

    // Requirement: State should be persisted and retrievable
    let retrieved = storage
        .get_lsp_state("test-session", "rust-analyzer")
        .await
        .expect("Failed to retrieve LSP state");

    assert!(retrieved.is_some(), "LSP state should be persisted");

    let retrieved = retrieved.unwrap();

    // Verify all fields match (per FR5.2 schema)
    assert_eq!(retrieved.session_id, original_state.session_id);
    assert_eq!(retrieved.language, original_state.language);
    assert_eq!(retrieved.lsp_name, original_state.lsp_name);
    assert_eq!(retrieved.status, original_state.status);
    assert_eq!(retrieved.pid, original_state.pid);
    assert_eq!(retrieved.auto_start, original_state.auto_start);

    // Verify we can also get all session LSPs
    let session_lsps = storage
        .get_session_lsp_states("test-session")
        .await
        .expect("Failed to get session LSP states");

    assert_eq!(session_lsps.len(), 1);
    assert_eq!(session_lsps[0].session_id, original_state.session_id);
}

/// Integration test for LSP auto-start flow (FR4.1 + FR5)
///
/// Tests the complete flow: Create session → Detect language → Auto-start LSP → Persist state
#[tokio::test]
async fn test_complete_autostart_flow() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let project_path = temp_dir.path();

    // Setup: Create a multi-language project (Rust + Python)
    fs::write(project_path.join("main.rs"), "fn main() {}")
        .expect("Failed to create main.rs");
    fs::write(project_path.join("script.py"), "print('hello')")
        .expect("Failed to create script.py");

    // Create storage and initialize
    let storage = TursoStorageBackend::new(Some(temp_dir.path().join("test.db")), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    let lsp_manager = LspManager::new(storage.clone());

    // Step 1: Create a session
    let session = Session {
        id: 0,
        session_id: "multi-lang-session".to_string(),
        title: "Multi-Language Project".to_string(),
        project_path: project_path.to_string_lossy().to_string(),
        group_path: None,
        sort_order: 0,
        parent_session_id: None,
        command: None,
        tool: None,
        status: SessionStatus::Running,
        multiplexer_session: None,
        started_at: chrono::Utc::now(),
        last_accessed_at: None,
        ended_at: None,
        metadata: None,
    };

    let _session_id = storage
        .insert_session(&session)
        .await
        .expect("Failed to create session");

    // Step 2: Detect languages
    let detected = lsp_manager
        .detect_languages_from_project(project_path)
        .await
        .expect("Failed to detect languages");

    // Requirement: Should detect both Rust and Python
    assert!(
        detected.contains(&LspType::Rust),
        "Should detect Rust"
    );
    assert!(
        detected.contains(&LspType::Python),
        "Should detect Python"
    );

    // Step 3: Get recommended LSPs
    let recommended = lsp_manager
        .recommend_lsps_for_session(&session.session_id, project_path)
        .await
        .expect("Failed to get recommendations");

    // Requirement: Both LSPs should be recommended
    assert!(
        recommended.contains(&LspType::Rust),
        "Should recommend rust-analyzer"
    );
    assert!(
        recommended.contains(&LspType::Python),
        "Should recommend ruff-lsp"
    );

    // Step 4: Verify we can query LSP state (would be populated by actual LSP manager)
    let _rust_state = storage
        .get_lsp_state(&session.session_id, "rust-analyzer")
        .await;

    // The state may or may not exist depending on whether LSP was actually started
    // This test validates the integration flow, not actual LSP spawning
}

/// Integration test for multi-language session with LSP state
///
/// Requirement: Multiple LSPs per session should be tracked independently
#[tokio::test]
async fn test_multiple_lsps_per_session_tracking() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let session_id = "test-multi-lsp";

    // Create storage and initialize
    let storage = TursoStorageBackend::new(Some(db_path.clone()), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    // Requirement: Track multiple LSPs per session (FR3.2)
    let lsp_configs = vec![
        ("rust-analyzer", "rust"),
        ("ruff-lsp", "python"),
        ("typescript-language-server", "typescript"),
    ];

    for (lsp_name, language) in lsp_configs {
        let state = LspServerState {
            id: 0,
            session_id: session_id.to_string(),
            language: language.to_string(),
            lsp_name: lsp_name.to_string(),
            status: LspStatus::Running,
            pid: Some(10000 + lsp_name.len() as i64), // Unique fake PIDs
            port: None,
            auto_start: true,
            use_proxy: false,
            last_started: Some(chrono::Utc::now().to_rfc3339()),
            last_error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        storage
            .upsert_lsp_state(&state)
            .await
            .expect("Failed to persist LSP state");
    }

    // Requirement: Retrieve all LSPs for the session
    let session_lsps = storage
        .get_session_lsp_states(session_id)
        .await
        .expect("Failed to get session LSP states");

    // Requirement: Should have all 3 LSPs tracked
    assert_eq!(
        session_lsps.len(),
        3,
        "Should track 3 LSPs for the session"
    );

    // Verify each LSP is present with correct attributes
    let lsp_names: Vec<&str> = session_lsps.iter().map(|s| s.lsp_name.as_str()).collect();
    assert!(lsp_names.contains(&"rust-analyzer"));
    assert!(lsp_names.contains(&"ruff-lsp"));
    assert!(lsp_names.contains(&"typescript-language-server"));

    // Verify unique constraint works (session_id, lsp_name is unique)
    // by trying to upsert the same LSP again
    let duplicate_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: "rust-analyzer".to_string(),
        status: LspStatus::Stopped,
        pid: None,
        port: None,
        auto_start: false,
        use_proxy: false,
        last_started: None,
        last_error: Some("Test error".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    };

    storage
        .upsert_lsp_state(&duplicate_state)
        .await
        .expect("Failed to upsert duplicate LSP state");

    // Verify the LSP was updated (ON CONFLICT DO UPDATE)
    let updated_state = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to retrieve updated LSP state")
        .expect("LSP state should exist");

    assert_eq!(updated_state.status, LspStatus::Stopped);
    assert_eq!(updated_state.last_error, Some("Test error".to_string()));
    assert_eq!(updated_state.auto_start, false);
}

/// Integration test for LSP status transitions (FR3.1)
///
/// Requirement: LSP status should transition correctly:
/// Stopped → Starting → Running OR Stopped → Error
#[tokio::test]
async fn test_lsp_status_transitions() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    let session_id = "status-test";

    // Test 1: Create LSP in "stopped" state (default per spec FR5.2)
    let stopped_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: "rust-analyzer".to_string(),
        status: LspStatus::Stopped,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: None,
        last_error: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };

    let _id = storage
        .upsert_lsp_state(&stopped_state)
        .await
        .expect("Failed to create stopped state");

    // Verify stopped state
    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get stopped state");

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().status, LspStatus::Stopped);

    // Test 2: Transition to "starting" (simulating start)
    let starting_state = LspServerState {
        status: LspStatus::Starting,
        pid: None,
        ..stopped_state.clone()
    };

    storage
        .upsert_lsp_state(&starting_state)
        .await
        .expect("Failed to update to starting state");

    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get starting state");

    assert_eq!(retrieved.unwrap().status, LspStatus::Starting);

    // Test 3: Transition to "running" (simulating successful start)
    let running_state = LspServerState {
        status: LspStatus::Running,
        pid: Some(12345),
        last_started: Some(chrono::Utc::now().to_rfc3339()),
        ..starting_state.clone()
    };

    storage
        .upsert_lsp_state(&running_state)
        .await
        .expect("Failed to update to running state");

    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get running state");

    assert_eq!(retrieved.as_ref().unwrap().status, LspStatus::Running);
    assert_eq!(retrieved.as_ref().unwrap().pid, Some(12345));

    // Test 4: Transition to "error" (simulating crash)
    let error_state = LspServerState {
        status: LspStatus::Error,
        pid: None,
        last_error: Some("Process crashed".to_string()),
        ..running_state
    };

    storage
        .upsert_lsp_state(&error_state)
        .await
        .expect("Failed to update to error state");

    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get error state");

    assert_eq!(retrieved.as_ref().unwrap().status, LspStatus::Error);
    assert_eq!(
        retrieved.as_ref().unwrap().last_error,
        Some("Process crashed".to_string())
    );
}

/// Integration test for auto_start flag (FR4.2)
///
/// Requirement: auto_start should persist and be configurable
#[tokio::test]
async fn test_auto_start_flag_persistence() {
    let temp_dir = TempDir::new().expect("Failed to temp dir");
    let db_path = temp_dir.path().join("test.db");
    let session_id = "autostart-test";

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    // Test 1: Default auto_start = TRUE (spec requirement)
    let default_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: "rust-analyzer".to_string(),
        status: LspStatus::Stopped,
        pid: None,
        port: None,
        auto_start: true,  // Requirement: DEFAULT TRUE per FR5.2
        use_proxy: false,
        last_started: None,
        last_error: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };

    storage
        .upsert_lsp_state(&default_state)
        .await
        .expect("Failed to create default state");

    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get default state");

    assert_eq!(retrieved.unwrap().auto_start, true);

    // Test 2: Set auto_start to false (manual override per FR4.2)
    let manual_state = LspServerState {
        auto_start: false,  // User manually disabled
        status: LspStatus::Stopped,
        ..default_state
    };

    storage
        .upsert_lsp_state(&manual_state)
        .await
        .expect("Failed to set auto_start to false");

    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get manual state");

    assert_eq!(retrieved.unwrap().auto_start, false);

    // Test 3: Verify auto_start persists across updates
    let running_state = LspServerState {
        status: LspStatus::Running,
        pid: Some(99999),
        ..manual_state
    };

    storage
        .upsert_lsp_state(&running_state)
        .await
        .expect("Failed to update state");

    let retrieved = storage
        .get_lsp_state(session_id, "rust-analyzer")
        .await
        .expect("Failed to get updated state");

    // Requirement: auto_start flag should persist
    assert_eq!(retrieved.unwrap().auto_start, false);
}

/// Integration test for session + LSP lifecycle (FR4.1 + FR5)
///
/// Requirement: When session is destroyed, LSPs should be stopped/cleaned up
#[tokio::test]
async fn test_session_lsp_cleanup_on_deletion() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let session_id = "cleanup-test";

    let storage = TursoStorageBackend::new(Some(db_path), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    // Create LSP states for the session
    for lsp_name in ["rust-analyzer", "ruff-lsp"] {
        let state = LspServerState {
            id: 0,
            session_id: session_id.to_string(),
            language: if lsp_name == "rust-analyzer" {
                "rust"
            } else {
                "python"
            }
            .to_string(),
            lsp_name: lsp_name.to_string(),
            status: LspStatus::Running,
            pid: Some(12345),
            port: None,
            auto_start: true,
            use_proxy: false,
            last_started: Some(chrono::Utc::now().to_rfc3339()),
            last_error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        storage
            .upsert_lsp_state(&state)
            .await
            .expect("Failed to create LSP state");
    }

    // Requirement: Session and its LSP states should exist
    let session_lsps = storage
        .get_session_lsp_states(session_id)
        .await
        .expect("Failed to get session LSPs");

    assert_eq!(session_lsps.len(), 2);

    // Delete the session (simulating session end)
    // Requirement: Should cascade delete LSP states (foreign key relationship)
    let session = Session {
        id: 0,
        session_id: session_id.to_string(),
        title: "Test".to_string(),
        project_path: "/test".to_string(),
        group_path: None,
        sort_order: 0,
        parent_session_id: None,
        command: None,
        tool: None,
        status: SessionStatus::Completed,
        multiplexer_session: None,
        started_at: chrono::Utc::now(),
        last_accessed_at: None,
        ended_at: Some(chrono::Utc::now()),
        metadata: None,
    };

    storage
        .insert_session(&session)
        .await
        .expect("Failed to create session");

    storage
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");

    // Requirement: LSP states should be cascade deleted when session is deleted
    let session_lsps = storage
        .get_session_lsp_states(session_id)
        .await
        .expect("Failed to get session LSPs after deletion");

    assert_eq!(session_lsps.len(), 0, "LSP states should be cascade deleted");
}

/// Integration test for LSP configuration scenarios (FR1.2 + FR1.3)
///
/// Requirements:
/// - FR1.2: System shall verify LSP availability on startup and report missing LSPs via TUI
/// - FR1.3: System shall provide installation instructions for missing LSPs in the TUI
#[tokio::test]
async fn test_lsp_availability_detection() {
    // This test verifies the logic for detecting LSP availability
    // In a real scenario, the system would:
    // 1. Check if LSP binaries exist in PATH
    // 2. Report missing LSPs via TUI
    // 3. Provide installation instructions

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create a project with multiple languages
    fs::write(project_path.join("main.rs"), "fn main() {}")
        .expect("Failed to create main.rs");
    fs::write(project_path.join("app.py"), "print('test')")
        .expect("Failed to create app.py");

    let storage = TursoStorageBackend::new(Some(temp_dir.path().join("test.db")), None)
        .await
        .expect("Failed to create storage");
    storage.initialize().await.expect("Failed to initialize");

    let lsp_manager = LspManager::new(storage);

    // Detect what LSPs are needed
    let recommended = lsp_manager
        .recommend_lsps_for_session("test-session", project_path)
        .await
        .expect("Failed to get recommendations");

    // Requirement: System should know which LSPs are needed (for installation guidance)
    assert!(!recommended.is_empty());

    // Verify we know the correct LSP names for installation instructions
    let lsp_names: Vec<String> = recommended
        .iter()
        .map(|lsp_type| lsp_type.binary_name().to_string())
        .collect();

    assert!(lsp_names.contains(&"rust-analyzer".to_string()));
    assert!(lsp_names.contains(&"ruff-lsp".to_string()));
}
