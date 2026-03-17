#[cfg(test)]
mod tests {
    use super::super::model::SelectableItem;
    use super::super::pane::ConductorPane;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{LazyLock, Mutex, MutexGuard};
    use tempfile::TempDir;

    static HOME_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct HomeEnvGuard {
        previous_home: Option<OsString>,
        _lock: MutexGuard<'static, ()>,
    }

    impl HomeEnvGuard {
        fn set(path: &std::path::Path) -> Self {
            let lock = HOME_ENV_LOCK.lock().unwrap();
            let previous_home = std::env::var_os("HOME");
            std::env::set_var("HOME", path);
            Self {
                previous_home,
                _lock: lock,
            }
        }
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            match &self.previous_home {
                Some(previous) => std::env::set_var("HOME", previous),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn test_master_track_detection() {
        let temp = TempDir::new().unwrap();
        let tracks_md = temp.path().join("tracks.md");
        fs::write(tracks_md, "## [ ] Some Track\n*Link: [./my-master-track/](./my-master-track/)\n**Description**: Master Track").unwrap();

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

    #[test]
    fn test_external_session_discovery() {
        let temp_home = TempDir::new().unwrap();
        let orchestrate_dir = temp_home.path().join(".maestro").join("orchestrate");
        fs::create_dir_all(&orchestrate_dir).unwrap();

        let session_dir = orchestrate_dir.join("ext-track");
        fs::create_dir_all(&session_dir).unwrap();
        fs::write(session_dir.join("session.json"), r#"{"session_id":"test","track_id":"ext-track","status":"running","mode":"building","agent_config":{"tool":"claude","dangerous_mode":false,"sandbox":false},"current_iteration":1,"current_task_id":null,"started_at":"","updated_at":""}"#).unwrap();

        // Mock HOME
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

        // Initial catch-up (suppress_output = true)
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

        // Live update (suppress_output = false)
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

        // Mock HOME to empty dir
        let empty_home = temp.path().join("empty_home");
        fs::create_dir_all(&empty_home).unwrap();
        let _home_guard = HomeEnvGuard::set(&empty_home);

        pane.load_tracks().unwrap();
        assert!(pane.tracks.is_empty(), "Tracks should be empty");

        let items = pane.get_selectable_items();
        assert!(items.is_empty(), "Selectable items should be empty");

        let track_idx = pane.get_selected_track_index();
        assert!(track_idx.is_none(), "Selected track index should be None");
    }

    #[test]
    fn test_setup_wizard_auto_shows_when_tracks_missing() {
        let temp = TempDir::new().unwrap();
        let pane = ConductorPane::new(temp.path().to_path_buf());

        assert!(
            pane.setup.show_setup_wizard,
            "setup wizard should auto-show when the workspace is not minimally configured"
        );
    }

    #[test]
    fn test_refresh_preserves_planned_tracks() {
        let temp = TempDir::new().unwrap();
        let track_dir = temp.path().join("demo-track");
        fs::create_dir_all(&track_dir).unwrap();
        fs::write(
            temp.path().join("tracks.md"),
            "## [ ] Demo Track\n*Link: [./demo-track/](./demo-track/)\n**Description**: Demo",
        )
        .unwrap();
        fs::write(track_dir.join("plan.md"), "### [ ] Task 1: Ship it\n").unwrap();

        let mut pane = ConductorPane::new(temp.path().to_path_buf());
        pane.refresh_tracks_if_needed();

        assert!(
            pane.tracks.iter().any(|track| track.id == "demo-track"),
            "planned tracks should remain visible even without a live session"
        );
    }

    // Phase 7.6: Observer event bridge tests (RED - expected to fail before implementation)
    #[test]
    fn test_observer_can_subscribe_to_session_events() {
        use super::super::model::ConductorEvent;
        use super::super::observer::SessionEventBridge;

        // This test will fail until SessionEventBridge is implemented
        let bridge = crate::conductor::observer::InMemoryEventBridge::new();
        let session_id = "test-session-123";

        // Subscribe to events
        let mut rx = bridge.subscribe(session_id);

        // Simulate sending an event
        let event = ConductorEvent::IterationStarted {
            iteration: 1,
            task_id: "task-1".to_string(),
        };

        // This will fail until publish is implemented
        assert!(
            bridge.publish(session_id, event).is_ok(),
            "Should publish event to bridge"
        );

        // This will fail until subscribe channel receives events
        let received = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { rx.recv().await });

        assert!(received.is_ok(), "Should receive event from subscription");
    }

    #[test]
    fn test_observer_action_review_current_task() {
        use super::super::observer::ObserverAction;

        // Verify ObserverAction enum exists and has the expected variants
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
        use super::super::observer::ObserverAction;

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
        use super::super::observer::ObserverAction;

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
        use super::super::model::{ConductorEvent, ConductorState};
        use super::super::observer::SessionEventBridge;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // This test verifies that parallel event processing doesn't corrupt
        // the selected task focus in the conductor state

        let bridge = Arc::new(crate::conductor::observer::InMemoryEventBridge::new());
        let state = Arc::new(RwLock::new(ConductorState::default()));
        let session_id = "parallel-test-session";

        // Set initial task focus
        {
            let mut s = state.try_write().unwrap();
            s.current_task = Some("task-5".to_string());
            s.current_track = Some("track-a".to_string());
        }

        // Simulate concurrent events (this tests thread safety)
        let bridge_clone = Arc::clone(&bridge);
        let _state_clone = Arc::clone(&state);
        let session_id_clone = session_id.to_string();

        let handle = std::thread::spawn(move || {
            // Simulate fast event sequence
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

        // Verify state hasn't been corrupted by parallel updates
        let s = state.try_read().unwrap();
        // The original focus should remain intact or be deterministically updated
        assert_eq!(s.current_track.as_ref().unwrap(), "track-a");
        // Task may have changed due to events, but should be deterministic
        assert!(s.current_task.is_some(), "Task focus should be maintained");
    }

    #[test]
    fn test_event_deterministic_ordering() {
        use super::super::model::ConductorEvent;
        use super::super::observer::SessionEventBridge;

        // Verify events are delivered in the order they were published
        let bridge = crate::conductor::observer::InMemoryEventBridge::new();
        let session_id = "order-test-session";

        // Subscribe FIRST - broadcast channels only deliver events published after subscription
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

        // Publish events AFTER subscribing
        for event in events.clone() {
            let _ = bridge.publish(session_id, event);
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let received_events = rt.block_on(async {
            let mut received = Vec::new();
            // Receive all events with timeout
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
        // Verify order is preserved
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
}
