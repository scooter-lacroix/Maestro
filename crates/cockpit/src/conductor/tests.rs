#[cfg(test)]
mod tests {
    use super::super::model::SelectableItem;
    use super::super::pane::ConductorPane;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

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
        std::env::set_var("HOME", temp_home.path());

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
        let cmd = pane.get_start_command(None, false, false);
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
        std::env::set_var("HOME", empty_home);

        pane.load_tracks().unwrap();
        assert!(pane.tracks.is_empty(), "Tracks should be empty");

        let items = pane.get_selectable_items();
        assert!(items.is_empty(), "Selectable items should be empty");

        let track_idx = pane.get_selected_track_index();
        assert!(track_idx.is_none(), "Selected track index should be None");
    }
}
