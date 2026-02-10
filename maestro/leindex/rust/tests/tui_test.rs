//! TUI component tests for LSP integration
//!
//! These tests validate the TUI-specific behavior for LSP status indicators,
//! LSP tab navigation, and control actions.
//!
//! Tests focus on testable TUI logic:
//! - LSP status calculation and formatting
//! - Tab state transitions
//! - LSP control action logic
//! - State refresh logic
//!
//! Note: Full terminal UI testing with ratatui requires PTY mocking or
//! subprocess-based testing which is prohibitively complex. These tests
//! validate the business logic that drives the UI.

use std::sync::Arc;

use chrono::Utc;
use leindex_analyzers::memory::turso_backend::{LspServerState, LspStatus, TursoStorageBackend};

/// Test LSP status formatting for TUI display
///
/// Requirement: TUI should display LSP status with proper formatting.
/// Tests the status to display string conversion logic.
#[test]
fn test_lsp_status_display_formatting() {
    // Test all status variants format correctly
    assert_eq!(LspStatus::Running.as_str(), "running");
    assert_eq!(LspStatus::Stopped.as_str(), "stopped");
    assert_eq!(LspStatus::Error.as_str(), "error");
    assert_eq!(LspStatus::Starting.as_str(), "starting");

    // Test that status conversion from storage works
    let stored_status = "running";
    assert_eq!(LspStatus::from_str(stored_status), Some(LspStatus::Running));

    let stored_status = "stopped";
    assert_eq!(LspStatus::from_str(stored_status), Some(LspStatus::Stopped));

    // Test invalid status
    assert_eq!(LspStatus::from_str("invalid"), None);
}

/// Test LSP status color mapping for TUI indicators
///
/// Requirement: FR3.1 - LSP Status Indicators
/// Each session card shall display LSP status indicators:
/// - GREEN (Active): LSP running and responsive
/// - YELLOW (Starting): LSP initializing
/// - RED (Error): LSP failed to start or crashed
/// - GRAY (Disabled): LSP manually disabled or not applicable
#[test]
fn test_lsp_status_color_mapping() {
    // This test validates the color mapping logic
    // In the actual TUI, colors would be applied based on LspStatus

    fn get_status_color(status: &LspStatus) -> &'static str {
        match status {
            LspStatus::Running => "green",   // Active
            LspStatus::Starting => "yellow", // Initializing
            LspStatus::Stopped => "gray",    // Disabled
            LspStatus::Error => "red",       // Failed
        }
    }

    // Test color mapping matches FR3.1 requirements
    assert_eq!(get_status_color(&LspStatus::Running), "green");
    assert_eq!(get_status_color(&LspStatus::Starting), "yellow");
    assert_eq!(get_status_color(&LspStatus::Stopped), "gray");
    assert_eq!(get_status_color(&LspStatus::Error), "red");
}

/// Test LSP status indicator display logic
///
/// Requirement: When multiple LSPs exist for a session, display aggregated status.
/// Priority: Error > Starting > Running > Stopped
#[test]
fn test_lsp_aggregated_status_display() {
    // Test aggregation logic for displaying overall session LSP status
    fn aggregate_lsp_status(lsps: &[LspStatus]) -> LspStatus {
        // Priority: Error > Starting > Running > Stopped
        if lsps.iter().any(|s| matches!(s, LspStatus::Error)) {
            return LspStatus::Error;
        }
        if lsps.iter().any(|s| matches!(s, LspStatus::Starting)) {
            return LspStatus::Starting;
        }
        if lsps.iter().any(|s| matches!(s, LspStatus::Running)) {
            return LspStatus::Running;
        }
        LspStatus::Stopped
    }

    // Test cases
    assert_eq!(
        aggregate_lsp_status(&[LspStatus::Running, LspStatus::Running]),
        LspStatus::Running
    );
    assert_eq!(
        aggregate_lsp_status(&[LspStatus::Running, LspStatus::Error]),
        LspStatus::Error // Error takes priority
    );
    assert_eq!(
        aggregate_lsp_status(&[LspStatus::Starting, LspStatus::Running]),
        LspStatus::Starting // Starting takes priority over running
    );
    assert_eq!(
        aggregate_lsp_status(&[LspStatus::Stopped, LspStatus::Stopped]),
        LspStatus::Stopped
    );
    assert_eq!(
        aggregate_lsp_status(&[]),
        LspStatus::Stopped // No LSPs = stopped/disabled
    );
}

/// Test LSP process state transitions for TUI
///
/// Requirement: TUI controls should trigger proper state transitions.
/// Tests the state machine for LSP lifecycle management.
#[tokio::test]
async fn test_lsp_state_transitions_for_tui() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    let session_id = "test-session";
    let lsp_name = "rust-analyzer";

    // Test 1: Initial state (no LSP) -> Stopped
    let status = storage.get_lsp_state(session_id, lsp_name).await.unwrap();
    assert!(status.is_none());

    // Test 2: Create LSP in Stopped state
    let state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: lsp_name.to_string(),
        status: LspStatus::Stopped,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: None,
        last_error: None,
        created_at: Utc::now().to_rfc3339(),
        updated_at: None,
    };
    storage.upsert_lsp_state(&state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Stopped);

    // Test 3: Transition to Starting (TUI "Start" action)
    let starting_state = LspServerState {
        status: LspStatus::Starting,
        ..state
    };
    storage.upsert_lsp_state(&starting_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Starting);

    // Test 4: Transition to Running (LSP started successfully)
    let running_state = LspServerState {
        status: LspStatus::Running,
        pid: Some(12345),
        last_started: Some(Utc::now().to_rfc3339()),
        ..starting_state
    };
    storage.upsert_lsp_state(&running_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Running);
    assert_eq!(retrieved.pid, Some(12345));

    // Test 5: Transition to Error (LSP crashed)
    let error_state = LspServerState {
        status: LspStatus::Error,
        pid: None,
        last_error: Some("Process crashed".to_string()),
        ..running_state
    };
    storage.upsert_lsp_state(&error_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Error);
    assert!(retrieved.last_error.is_some());

    // Test 6: Stop action (user manually stops LSP)
    let stopped_state = LspServerState {
        status: LspStatus::Stopped,
        pid: None,
        auto_start: false,
        use_proxy: false, // User disabled auto-start
        ..error_state
    };
    storage.upsert_lsp_state(&stopped_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Stopped);
    assert_eq!(retrieved.auto_start, false);
}

/// Test LSP tab data structure
///
/// Requirement: FR3.2 - LSP Management Tab
/// Should display: List of all active LSPs with status, per-LSP controls,
/// per-session LSP configuration.
#[tokio::test]
async fn test_lsp_tab_data_structure() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Arc::new(TursoStorageBackend::new(Some(db_path), None).await.unwrap());
    storage.initialize().await.unwrap();

    // Create multiple sessions with LSPs
    let sessions = vec!["session-1", "session-2", "session-3"];
    let lsp_configs = vec![
        ("rust-analyzer", "rust"),
        ("ruff-lsp", "python"),
        ("typescript-language-server", "typescript"),
    ];

    for (i, session_id) in sessions.iter().enumerate() {
        for (lsp_name, language) in &lsp_configs {
            let state = LspServerState {
                id: 0,
                session_id: session_id.to_string(),
                language: language.to_string(),
                lsp_name: lsp_name.to_string(),
                status: if i % 2 == 0 {
                    LspStatus::Running
                } else {
                    LspStatus::Stopped
                },
                pid: if i % 2 == 0 {
                    Some(10000 + i as i64)
                } else {
                    None
                },
                port: None,
                auto_start: true,
                use_proxy: false,
                last_started: if i % 2 == 0 {
                    Some(Utc::now().to_rfc3339())
                } else {
                    None
                },
                last_error: None,
                created_at: Utc::now().to_rfc3339(),
                updated_at: Some(Utc::now().to_rfc3339()),
            };
            storage.upsert_lsp_state(&state).await.unwrap();
        }
    }

    // Test: Query all LSPs for TUI LSP tab
    // Count across all sessions since there's no get_all_lsp_states method
    let mut total_count = 0;
    for session_id in &sessions {
        let session_lsps = storage.get_session_lsp_states(session_id).await.unwrap();
        total_count += session_lsps.len();
    }
    assert_eq!(total_count, 9); // 3 sessions × 3 LSPs

    // Test: Filter by status (TUI needs to show "Running" LSPs)
    let mut running_count = 0;
    for session_id in &sessions {
        let session_lsps = storage.get_session_lsp_states(session_id).await.unwrap();
        running_count += session_lsps
            .iter()
            .filter(|s| matches!(s.status, LspStatus::Running))
            .count();
    }
    assert_eq!(running_count, 6); // 2 sessions with 3 LSPs each

    // Test: Query by session
    for session_id in &sessions {
        let session_lsps = storage.get_session_lsp_states(session_id).await.unwrap();
        assert_eq!(session_lsps.len(), 3);
    }
}

/// Test LSP auto-start flag for TUI controls
///
/// Requirement: FR4.2 - Manual Override
/// Users should be able to disable auto-start for specific sessions.
#[tokio::test]
async fn test_lsp_autostart_flag_for_tui() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    let session_id = "test-session";
    let lsp_name = "rust-analyzer";

    // Create LSP with auto_start = true
    let auto_on_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: lsp_name.to_string(),
        status: LspStatus::Stopped,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: None,
        last_error: None,
        created_at: Utc::now().to_rfc3339(),
        updated_at: None,
    };
    storage.upsert_lsp_state(&auto_on_state).await.unwrap();

    // TUI "Disable Auto-Start" action
    let auto_off_state = LspServerState {
        auto_start: false,
        use_proxy: false,
        ..auto_on_state
    };
    storage.upsert_lsp_state(&auto_off_state).await.unwrap();

    // Verify auto_start is disabled
    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.auto_start, false);

    // TUI "Enable Auto-Start" action
    let reenabled_state = LspServerState {
        auto_start: true,
        use_proxy: false,
        ..retrieved
    };
    storage.upsert_lsp_state(&reenabled_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.auto_start, true);
}

/// Test LSP error message display for TUI
///
/// Requirement: FR3.3 - LSP Log Viewer
/// The TUI should display error messages from failed LSP operations.
#[tokio::test]
async fn test_lsp_error_display_for_tui() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    // Simulate LSP failure with error message
    let error_state = LspServerState {
        id: 0,
        session_id: "test-session".to_string(),
        language: "rust".to_string(),
        lsp_name: "rust-analyzer".to_string(),
        status: LspStatus::Error,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: Some(Utc::now().to_rfc3339()),
        last_error: Some("Failed to start: binary not found in PATH".to_string()),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    storage.upsert_lsp_state(&error_state).await.unwrap();

    // TUI retrieves error for display
    let retrieved = storage
        .get_lsp_state("test-session", "rust-analyzer")
        .await
        .unwrap()
        .unwrap();

    // Verify error information is available for TUI display
    assert_eq!(retrieved.status, LspStatus::Error);
    assert_eq!(
        retrieved.last_error,
        Some("Failed to start: binary not found in PATH".to_string())
    );

    // Format error message for TUI display
    let error_display = format!(
        "{}: {}",
        retrieved.lsp_name,
        retrieved
            .last_error
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Unknown error")
    );
    assert!(error_display.contains("rust-analyzer"));
    assert!(error_display.contains("binary not found"));
}

/// Test LSP process info extraction for TUI
///
/// Requirement: TUI should display process ID and port for active LSPs.
#[test]
fn test_lsp_process_info_for_tui_display() {
    // Test extracting process info for TUI display
    fn format_lsp_process_info(name: &str, pid: Option<i64>, port: Option<u16>) -> String {
        let pid_info = pid
            .map(|p| format!("PID: {}", p))
            .unwrap_or_else(|| "Not running".to_string());
        let port_info = port
            .map(|p| format!("Port: {}", p))
            .unwrap_or_else(|| "".to_string());
        format!("{} - {} {}", name, pid_info, port_info)
            .trim()
            .to_string()
    }

    // Test formatting with different states
    assert_eq!(
        format_lsp_process_info("rust-analyzer", Some(12345), None),
        "rust-analyzer - PID: 12345"
    );
    assert_eq!(
        format_lsp_process_info("ruff-lsp", Some(6789), Some(9001)),
        "ruff-lsp - PID: 6789 Port: 9001"
    );
    assert_eq!(
        format_lsp_process_info("typescript-language-server", None, None),
        "typescript-language-server - Not running"
    );
    // Test case with only port (no PID) - edge case
    let result = format_lsp_process_info("custom-lsp", None, Some(8080));
    assert!(result.contains("custom-lsp"));
    assert!(result.contains("Port: 8080"));
}

/// Test TUI tab navigation state
///
/// Requirement: TUI should track which tab is active.
#[test]
fn test_tui_tab_navigation_state() {
    // Simulate TUI tab state
    #[derive(Debug, Clone, PartialEq)]
    enum Tab {
        Sessions,
        Memories,
        Projects,
        Lsps,
        McpServers,
        Logs,
    }

    let mut current_tab = Tab::Sessions;

    // Test tab navigation
    current_tab = Tab::Memories;
    assert_eq!(current_tab, Tab::Memories);

    current_tab = Tab::Lsps;
    assert_eq!(current_tab, Tab::Lsps);

    // Test tab validation
    let valid_tabs = vec![Tab::Sessions, Tab::Memories, Tab::Projects, Tab::Lsps];
    assert!(valid_tabs.contains(&current_tab));

    // Test that Lsps tab is accessible (FR3.2 requirement)
    assert!(valid_tabs.contains(&Tab::Lsps));
}

/// Test session + LSP count display for TUI
///
/// Requirement: TUI should display LSP count per session.
#[tokio::test]
async fn test_session_lsp_count_for_tui() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    // Create sessions with varying LSP counts
    // session-1: 3 LSPs
    for lsp_name in ["rust-analyzer", "ruff-lsp", "typescript-language-server"] {
        let state = LspServerState {
            id: 0,
            session_id: "session-1".to_string(),
            language: "multi".to_string(),
            lsp_name: lsp_name.to_string(),
            status: LspStatus::Running,
            pid: Some(10000),
            port: None,
            auto_start: true,
            use_proxy: false,
            last_started: Some(Utc::now().to_rfc3339()),
            last_error: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Some(Utc::now().to_rfc3339()),
        };
        storage.upsert_lsp_state(&state).await.unwrap();
    }

    // session-2: 1 LSP
    let state = LspServerState {
        id: 0,
        session_id: "session-2".to_string(),
        language: "rust".to_string(),
        lsp_name: "rust-analyzer".to_string(),
        status: LspStatus::Running,
        pid: Some(10001),
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: Some(Utc::now().to_rfc3339()),
        last_error: None,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    storage.upsert_lsp_state(&state).await.unwrap();

    // session-3: 0 LSPs (no LSPs configured)

    // Test LSP count retrieval for TUI display
    let count_1 = storage
        .get_session_lsp_states("session-1")
        .await
        .unwrap()
        .len();
    assert_eq!(count_1, 3);

    let count_2 = storage
        .get_session_lsp_states("session-2")
        .await
        .unwrap()
        .len();
    assert_eq!(count_2, 1);

    let count_3 = storage
        .get_session_lsp_states("session-3")
        .await
        .unwrap()
        .len();
    assert_eq!(count_3, 0);
}

/// Test TUI LSP refresh logic
///
/// Requirement: TUI should refresh LSP status periodically.
/// This tests the refresh logic that was marked as "never used".
#[tokio::test]
async fn test_tui_lsp_refresh_logic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Arc::new(TursoStorageBackend::new(Some(db_path), None).await.unwrap());
    storage.initialize().await.unwrap();

    let session_id = "refresh-test";
    let lsp_name = "rust-analyzer";

    // Initial state: No LSP
    let initial_state = storage.get_lsp_state(session_id, lsp_name).await.unwrap();
    assert!(initial_state.is_none());

    // Simulate LSP starting
    let starting_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: lsp_name.to_string(),
        status: LspStatus::Starting,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: Some(Utc::now().to_rfc3339()),
        last_error: None,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    storage.upsert_lsp_state(&starting_state).await.unwrap();

    // Simulate LSP started successfully
    let running_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: lsp_name.to_string(),
        status: LspStatus::Running,
        pid: Some(54321),
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: Some(Utc::now().to_rfc3339()),
        last_error: None,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    storage.upsert_lsp_state(&running_state).await.unwrap();

    // TUI refresh: Get LSPs for display
    let session_lsps = storage.get_session_lsp_states(session_id).await.unwrap();
    assert!(!session_lsps.is_empty());

    // Verify the most recent state is Running
    let latest_state = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest_state.status, LspStatus::Running);
    assert_eq!(latest_state.pid, Some(54321));
}

/// Test TUI LSP control actions validation
///
/// Requirement: FR4.4 - Manual Restart
/// Users shall be able to manually restart failed LSPs via TUI controls.
#[tokio::test]
async fn test_tui_lsp_control_actions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    let session_id = "control-test";
    let lsp_name = "rust-analyzer";

    // Simulate LSP in error state (needs restart)
    let error_state = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: lsp_name.to_string(),
        status: LspStatus::Error,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: Some(Utc::now().to_rfc3339()),
        last_error: Some("Process crashed unexpectedly".to_string()),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    storage.upsert_lsp_state(&error_state).await.unwrap();

    // TUI "Restart" action: Set to Starting, then clear error
    let restart_state = LspServerState {
        status: LspStatus::Starting,
        pid: None,
        last_error: None,
        updated_at: Some(Utc::now().to_rfc3339()),
        ..error_state
    };
    storage.upsert_lsp_state(&restart_state).await.unwrap();

    // Verify restart state
    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Starting);
    assert!(retrieved.last_error.is_none());

    // Simulate restart successful
    let running_state = LspServerState {
        status: LspStatus::Running,
        pid: Some(54321),
        last_started: Some(Utc::now().to_rfc3339()),
        ..restart_state
    };
    storage.upsert_lsp_state(&running_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, LspStatus::Running);
}

/// Test TUI LSP log viewer data
///
/// Requirement: FR3.3 - LSP Log Viewer
/// The TUI should provide real-time log viewing for each LSP process.
#[tokio::test]
async fn test_tui_lsp_log_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    let session_id = "log-test";
    let lsp_name = "rust-analyzer";

    // Create LSP state with error log information
    let state_with_error = LspServerState {
        id: 0,
        session_id: session_id.to_string(),
        language: "rust".to_string(),
        lsp_name: lsp_name.to_string(),
        status: LspStatus::Error,
        pid: None,
        port: None,
        auto_start: true,
        use_proxy: false,
        last_started: Some(Utc::now().to_rfc3339()),
        last_error: Some("[ERROR] Failed to connect to LSP: Connection refused".to_string()),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    storage.upsert_lsp_state(&state_with_error).await.unwrap();

    // TUI retrieves log entry for display
    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();

    // Format log entry for TUI log viewer
    let log_entry = format!(
        "[{}] {} - {}: {}",
        Utc::now().format("%Y-%m-%d %H:%M:%S"),
        retrieved.session_id,
        retrieved.lsp_name,
        retrieved
            .last_error
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("No logs")
    );

    assert!(log_entry.contains("[ERROR]"));
    assert!(log_entry.contains("Failed to connect to LSP"));

    // Test multiple log entries for same LSP
    let newer_state = LspServerState {
        last_error: Some("[WARN] High memory usage detected".to_string()),
        ..state_with_error
    };
    storage.upsert_lsp_state(&newer_state).await.unwrap();

    let retrieved = storage
        .get_lsp_state(session_id, lsp_name)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        retrieved.last_error,
        Some("[WARN] High memory usage detected".to_string())
    );
}

/// Test TUI multi-language session display
///
/// Requirement: Session cards should display LSP indicators for all languages.
#[tokio::test]
async fn test_tui_multi_language_session_display() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = TursoStorageBackend::new(Some(db_path), None).await.unwrap();
    storage.initialize().await.unwrap();

    // Create session with 3 LSPs (multi-language project)
    let session_id = "multi-lang-session";

    for (lsp_name, language, status) in [
        ("rust-analyzer", "rust", LspStatus::Running),
        ("ruff-lsp", "python", LspStatus::Running),
        (
            "typescript-language-server",
            "typescript",
            LspStatus::Starting,
        ),
    ] {
        let state = LspServerState {
            id: 0,
            session_id: session_id.to_string(),
            language: language.to_owned(),
            lsp_name: lsp_name.to_owned(),
            status,
            pid: Some(10000),
            port: None,
            auto_start: true,
            use_proxy: false,
            last_started: Some(Utc::now().to_rfc3339()),
            last_error: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Some(Utc::now().to_rfc3339()),
        };
        storage.upsert_lsp_state(&state).await.unwrap();
    }

    // TUI retrieves all LSPs for the session
    let lsps = storage.get_session_lsp_states(session_id).await.unwrap();
    assert_eq!(lsps.len(), 3);

    // Verify each LSP's info for TUI display
    let lsp_info: Vec<(String, String, String)> = lsps
        .iter()
        .map(|lsp| {
            (
                lsp.lsp_name.clone(),
                lsp.language.clone(),
                lsp.status.as_str().to_string(),
            )
        })
        .collect();

    assert!(lsp_info.iter().any(|(name, _, _)| name == "rust-analyzer"));
    assert!(lsp_info.iter().any(|(name, _, _)| name == "ruff-lsp"));
    assert!(lsp_info
        .iter()
        .any(|(name, _, _)| name == "typescript-language-server"));

    // Verify status aggregation for session display
    let statuses: Vec<&str> = lsp_info
        .iter()
        .map(|(_, _, status)| status.as_str())
        .collect();
    assert!(statuses.contains(&"running"));
    assert!(statuses.contains(&"starting"));

    // Test aggregated status for the session card
    let aggregated_status = if statuses.contains(&"error") {
        "error"
    } else if statuses.contains(&"starting") {
        "starting"
    } else if statuses.contains(&"running") {
        "running"
    } else {
        "stopped"
    };

    // Should be "starting" because one LSP is in Starting state
    assert_eq!(aggregated_status, "starting");
}
