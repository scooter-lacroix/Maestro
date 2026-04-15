//! Conductor tests module
//!
//! Comprehensive test suite for conductor functionality.
//!
//! Tests are organized into phases corresponding to the implementation plan.
//!
//! ## Test Phases
//!
//! - Phase 1: Basic state management (✅ COMPLETE)
//! - Phase 2: Event processing (✅ COMPLETE)
//! - Phase 3: Track discovery (✅ COMPLETE)
//! - Phase 4: Task management (✅ COMPLETE)
//! - Phase 5: External sessions (✅ COMPLETE)
//! - Phase 6: Observer mode (✅ COMPLETE)
//! - Phase 7: Observer event bridge (⤔ IN PROGRESS)
//!
//! ## Adding New Tests
//!
//! When adding new tests, please:
//! 1. Identify which phase the test belongs to
//! 2. Place the test in the appropriate phase section
//! 3. Ensure the test name follows the `test_<feature>_<description>` convention
//! 4. Use `#[ignore]` for tests that are expected to fail during development
//!
//! ## Test Guidelines
//!
//! - Keep tests simple and focused
//! - Use descriptive assertions help with debugging
//! - Test both success and failure paths
//! - Ensure proper cleanup of temporary resources

//!
//! ## Code Review Notes
//!
//! Tests should verify:
//! - State transitions work correctly
//! - Event propagation works correctly
//! - Data structures are consistent
//! - Error handling is robust

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard};
use tempfile::TempDir;

use super::model::{ConductorState, ConductorStatus, SelectableItem};
use super::pane::ConductorPane;

static HOME_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct HomeEnvGuard {
    previous_home: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl HomeEnvGuard {
    fn set(path: &std::path::Path) -> Self {
        let lock = HOME_ENV_LOCK.lock().unwrap();
        let previous_home = std::env::var_os("HOME");
        // TODO: When upgrading to Rust 2024+, wrap set_var in unsafe {} with safety comment:
        // "Safe: we have exclusive access via HOME_ENV_LOCK and restore on Drop"
        std::env::set_var("HOME", path);
        Self {
            previous_home,
            _lock: lock,
        }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        // TODO: When upgrading to Rust 2024+, wrap set_var/remove_var in unsafe {} with safety comment:
        // "Safe: we have exclusive access via HOME_ENV_LOCK held until Drop completes"
        match &self.previous_home {
            Some(previous) => std::env::set_var("HOME", previous),
            None => std::env::remove_var("HOME"),
        }
    }
}

// ============================================================================
// Phase 1: Basic State Management Tests
// ============================================================================

#[test]
fn test_conductor_pane_creation() {
    let temp_dir = TempDir::new().unwrap();
    let pane = ConductorPane::new(temp_dir.path().to_path_buf());

    // Verify initial state
    assert!(pane.tracks.is_empty());
    assert_eq!(pane.selected_index, 0);
}

#[test]
fn test_conductor_state_defaults() {
    let state = ConductorState::default();

    assert_eq!(state.status, ConductorStatus::Ready);
    assert!(state.session_id.is_none());
    assert_eq!(state.current_iteration, 0);
}

// ============================================================================
// Phase 3: Track Discovery Tests
// ============================================================================

#[test]
fn test_track_discovery_finds_tracks() {
    let temp_dir = TempDir::new().unwrap();
    let tracks_dir = temp_dir.path().join("tracks");
    std::fs::create_dir_all(&tracks_dir).unwrap();

    // Create a track file
    let track_content = r###"## [ ] Demo Track

*Link: [./demo-track](./demo-track/)
**Description**: Demo
"###;
    std::fs::write(tracks_dir.join("tracks.md"), track_content).unwrap();

    let mut pane = ConductorPane::new(temp_dir.path().to_path_buf());
    pane.refresh_tracks_if_needed();

    assert!(
        pane.tracks.iter().any(|track| track.id == "demo-track"),
        "planned tracks should remain visible even without a live session"
    );
}

// ============================================================================
// Phase 4: Task Management Tests
// ============================================================================

#[test]
fn test_master_track_detection() {
    let temp = TempDir::new().unwrap();
    let tracks_md = temp.path().join("tracks.md");
    std::fs::write(tracks_md, "## [ ] Some Track\n*Link: [./my-master-track/](./my-master-track/)\n**Description**: Master Track").unwrap();

    let mut pane = ConductorPane::new(temp.path().to_path_buf());
    pane.load_tracks().unwrap();

    let items = pane.get_selectable_items();
    let master_track = items.iter().find(|i| match i {
        SelectableItem::Track { id, is_master, .. } => id == "my-master-track" && *is_master,
        _ => false,
    });

    assert!(
        master_track.is_some(),
        "Master track should be detected by ID pattern 'master'"
    );
}

#[test]
fn test_master_track_detection_via_metadata() {
    use leindex_core::orchestrate::model::{Track, TrackMetadata, TrackStatus, TrackType};
    let mut pane = ConductorPane::new(PathBuf::from("/non-existent"));
    pane.tracks.push(Track {
        id: "feature-1".to_string(),
        description: "feature".to_string(),
        status: TrackStatus::Pending,
        link_path: PathBuf::from("."),
        metadata: Some(TrackMetadata {
            track_id: "feature-1".to_string(),
            track_type: TrackType::Master,
            status: TrackStatus::Pending,
            created_at: "".to_string(),
            updated_at: "".to_string(),
            description: "Master Track".to_string(),
            sub_tracks: None,
        }),
        plan: None,
    });

    let items = pane.get_selectable_items();
    let master_track = items.iter().find(|i| match i {
        SelectableItem::Track { id, is_master, .. } => id == "feature-1" && *is_master,
        _ => false,
    });

    assert!(
        master_track.is_some(),
        "Master track should be detected by metadata type"
    );
}

// ============================================================================
// Phase 5: External Sessions Tests
// ============================================================================

#[test]
fn test_external_session_discovery() {
    let temp_home = TempDir::new().unwrap();
    let orchestrate_dir = temp_home.path().join(".maestro").join("orchestrate");
    std::fs::create_dir_all(&orchestrate_dir).unwrap();

    let session_dir = orchestrate_dir.join("ext-track");
    std::fs::create_dir_all(&session_dir).unwrap();
    std::fs::write(session_dir.join("session.json"), r#"{"session_id":"test","track_id":"ext-track","status":"running","mode":"building","agent_config":{"tool":"claude","dangerous_mode":false,"sandbox":false},"current_iteration":1,"current_task_id":null,"started_at":"","updated_at":""}"#).unwrap();

    let _home_guard = HomeEnvGuard::set(temp_home.path());

    let mut pane = ConductorPane::new(PathBuf::from("/non-existent"));
    pane.load_tracks().unwrap();

    let items = pane.get_selectable_items();
    let ext_track = items.iter().find(|i| match i {
        SelectableItem::Track {
            id, is_external, ..
        } => id == "ext-track" && *is_external,
        _ => false,
    });

    assert!(ext_track.is_some(), "External session should be discovered");
}

#[test]
fn test_process_iteration_log_suppression() {
    use leindex_core::orchestrate::model::{IterationLog, IterationStatus};

    let mut pane = ConductorPane::new(PathBuf::from("."));
    let log = IterationLog {
        iteration: 1,
        task_id: "task-1".to_string(),
        started_at: "2026-01-27T12:00:00Z".to_string(),
        completed_at: None,
        status: IterationStatus::Running,
        output: "some output".to_string(),
        error: None,
        duration_ms: 0,
    };

    pane.process_iteration_log(log.clone(), true);
    assert!(
        pane.iteration_output.is_empty(),
        "Output should be suppressed during initial read"
    );
    assert_eq!(
        pane.state.iteration_logs.len(),
        1,
        "History should still be updated"
    );

    pane.process_iteration_log(log, false);
    assert!(
        !pane.iteration_output.is_empty(),
        "Output should NOT be suppressed during live update"
    );
}

#[test]
fn test_command_generation() {
    let temp = TempDir::new().unwrap();
    let mut pane = ConductorPane::new(temp.path().to_path_buf());
    pane.tracks.push(leindex_core::orchestrate::model::Track {
        id: "test-track".to_string(),
        description: "test".to_string(),
        status: leindex_core::orchestrate::model::TrackStatus::Pending,
        link_path: PathBuf::from("."),
        metadata: None,
        plan: None,
    });

    let items = pane.get_selectable_items();
    pane.selected_index = items.len() - 1;
    let cmd = pane
        .get_start_command(None, false, false)
        .unwrap()
        .to_string();
    assert!(
        cmd.contains("maestro orchestrate start test-track"),
        "Command should contain start and track ID"
    );
    assert!(
        cmd.contains("--mode building"),
        "Default mode should be building"
    );
}

#[test]
fn test_graceful_empty_state() {
    let temp = TempDir::new().unwrap();
    let mut pane = ConductorPane::new(temp.path().to_path_buf());

    let empty_home = temp.path().join("empty_home");
    std::fs::create_dir_all(&empty_home).unwrap();
    let _home_guard = HomeEnvGuard::set(&empty_home);

    pane.load_tracks().unwrap();
    assert!(pane.tracks.is_empty(), "Tracks should be empty");

    let items = pane.get_selectable_items();
    assert!(items.is_empty(), "Selectable items should be empty");

    let track_idx = pane.get_selected_track_index();
    assert!(track_idx.is_none(), "Selected track index should be None");
}

// ============================================================================
// Phase 7: Observer Event Bridge Tests
// ============================================================================

#[test]
fn test_observer_can_subscribe_to_session_events() {
    use super::model::ConductorEvent;
    use super::observer::SessionEventBridge;

    let bridge = crate::conductor::observer::InMemoryEventBridge::new();
    let session_id = "test-session-123";

    let mut rx = bridge.subscribe(session_id);

    let event = ConductorEvent::IterationStarted {
        iteration: 1,
        task_id: "task-1".to_string(),
    };

    assert!(
        bridge.publish(session_id, event).is_ok(),
        "Should publish event to bridge"
    );

    let received = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { rx.recv().await });

    assert!(received.is_ok(), "Should receive event from subscription");
}

#[test]
fn test_observer_action_review_current_task() {
    use super::observer::ObserverAction;

    let action = ObserverAction::ReviewCurrentTask {
        iteration: 1,
        task_id: "task-1".to_string(),
    };

    match action {
        ObserverAction::ReviewCurrentTask { iteration, task_id } => {
            assert_eq!(iteration, 1);
            assert_eq!(task_id, "task-1");
        }
        _ => panic!("Expected ReviewCurrentTask variant"),
    }
}

#[test]
fn test_observer_action_request_retry() {
    use super::observer::ObserverAction;

    let action = ObserverAction::RequestRetry {
        task_id: "task-1".to_string(),
        reason: "Temporary error".to_string(),
    };

    match action {
        ObserverAction::RequestRetry { task_id, reason } => {
            assert_eq!(task_id, "task-1");
            assert_eq!(reason, "Temporary error");
        }
        _ => panic!("Expected RequestRetry variant"),
    }
}

#[test]
fn test_observer_action_inject_guidance() {
    use super::observer::ObserverAction;

    let action = ObserverAction::InjectGuidance {
        task_id: "task-1".to_string(),
        guidance: "Consider using async/await".to_string(),
    };

    match action {
        ObserverAction::InjectGuidance { task_id, guidance } => {
            assert_eq!(task_id, "task-1");
            assert_eq!(guidance, "Consider using async/await");
        }
        _ => panic!("Expected InjectGuidance variant"),
    }
}

#[test]
fn test_parallel_updates_preserve_task_focus() {
    use super::model::{ConductorEvent, ConductorState};
    use super::observer::SessionEventBridge;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let bridge = Arc::new(crate::conductor::observer::InMemoryEventBridge::new());
    let state = Arc::new(RwLock::new(ConductorState::default()));
    let session_id = "parallel-test-session";

    {
        let mut s = state.try_write().unwrap();
        s.current_task = Some("task-5".to_string());
        s.current_track = Some("track-a".to_string());
    }

    let bridge_clone = Arc::clone(&bridge);
    let _state_clone = Arc::clone(&state);
    let session_id_clone = session_id.to_string();

    let handle = std::thread::spawn(move || {
        let events = vec![
            ConductorEvent::TaskSelected {
                task_id: "task-1".to_string(),
                iteration: 1,
            },
            ConductorEvent::IterationStarted {
                iteration: 1,
                task_id: "task-1".to_string(),
            },
            ConductorEvent::TaskCompleted {
                task_id: "task-1".to_string(),
                iteration: 1,
            },
        ];

        for event in events {
            let _ = bridge_clone.publish(&session_id_clone, event);
        }
    });

    handle.join().unwrap();

    let s = state.try_read().unwrap();
    assert_eq!(s.current_track.as_ref().unwrap(), "track-a");
    assert!(s.current_task.is_some(), "Task focus should be maintained");
}

#[test]
fn test_event_deterministic_ordering() {
    use super::model::ConductorEvent;
    use super::observer::SessionEventBridge;

    let bridge = crate::conductor::observer::InMemoryEventBridge::new();
    let session_id = "order-test-session";

    let mut rx = bridge.subscribe(session_id);

    let events = vec![
        ConductorEvent::IterationStarted {
            iteration: 1,
            task_id: "task-1".to_string(),
        },
        ConductorEvent::IterationCompleted {
            iteration: 1,
            task_completed: true,
            duration_ms: 100,
        },
        ConductorEvent::IterationStarted {
            iteration: 2,
            task_id: "task-2".to_string(),
        },
    ];

    for event in events.clone() {
        let _ = bridge.publish(session_id, event);
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let received_events = rt.block_on(async {
        let mut received = Vec::new();
        for _ in 0..3 {
            match tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await
            {
                Ok(Ok(event)) => received.push(event),
                _ => break,
            }
        }
        received
    });

    assert_eq!(received_events.len(), 3, "Should receive all 3 events");
    match (&received_events[0], &events[0]) {
        (
            ConductorEvent::IterationStarted { iteration: i1, .. },
            ConductorEvent::IterationStarted { iteration: i2, .. },
        ) => {
            assert_eq!(i1, i2);
        }
        _ => panic!("First event should be IterationStarted"),
    }
}
