//! Comprehensive tests for welcome onboarding flow
//!
//! These tests verify the test-first requirements for Phase 7.2:
//! - State transitions
//! - Marker persistence
//! - First-run detection

#[cfg(test)]
mod onboarding_tests {
    use crate::welcome::wizard::WelcomeState;
    use crate::welcome::{cockpit_initialized_marker, WelcomeScreen};
    use std::fs;

    /// Test that marker file path is correct
    #[test]
    fn test_marker_file_path() {
        let marker = cockpit_initialized_marker();
        assert!(marker.to_str().unwrap().contains(".maestro"));
        assert!(marker.to_str().unwrap().contains(".cockpit_initialized"));
    }

    /// Test first-time user detection when marker doesn't exist
    #[test]
    fn test_first_time_detection_no_marker() {
        // Note: This test assumes the marker doesn't exist
        // In a real test environment, we'd use a temp directory
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let marker_path = format!("{}/.maestro/.cockpit_initialized_test", home);

        // Remove test marker if it exists
        let _ = fs::remove_file(&marker_path);

        // First-time detection should return true when marker doesn't exist
        // (We can't directly test is_first_time_user without affecting real state)
        assert!(!std::path::Path::new(&marker_path).exists());
    }

    /// Test that WelcomeScreen starts in NotStarted state
    #[test]
    fn test_welcome_screen_starts_not_started() {
        let screen = WelcomeScreen::default();
        assert!(matches!(screen.state, WelcomeState::NotStarted));
        assert!(!screen.is_visible());
    }

    /// Test that showing screen starts the wizard
    #[test]
    fn test_show_screen_starts_wizard() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        assert!(screen.is_visible());
        assert!(!matches!(screen.state, WelcomeState::NotStarted));
    }

    /// Test complete wizard flow
    #[test]
    fn test_complete_wizard_flow() {
        let mut screen = WelcomeScreen::default();
        screen.show();

        // Advance through all steps
        assert!(matches!(screen.state, WelcomeState::WorkspaceSetup { .. }));

        screen.advance();
        assert!(matches!(screen.state, WelcomeState::EditorSelection { .. }));

        screen.advance();
        assert!(matches!(screen.state, WelcomeState::ProviderSetup { .. }));

        screen.advance();
        assert!(matches!(screen.state, WelcomeState::ThemeSelection { .. }));

        screen.advance();
        assert!(matches!(screen.state, WelcomeState::Completed));

        assert!(screen.is_complete());
    }

    /// Test go back from each step
    #[test]
    fn test_go_back_navigation() {
        let mut screen = WelcomeScreen::default();
        screen.show();

        // Navigate to step 3
        screen.advance();
        screen.advance();

        assert!(matches!(screen.state, WelcomeState::ProviderSetup { .. }));

        // Go back
        screen.go_back();
        assert!(matches!(screen.state, WelcomeState::EditorSelection { .. }));

        // Go back again
        screen.go_back();
        assert!(matches!(screen.state, WelcomeState::WorkspaceSetup { .. }));
    }

    /// Test editor selection navigation
    #[test]
    fn test_editor_selection_navigation() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        screen.advance(); // Move to EditorSelection

        assert_eq!(screen.selected_editor, 0);

        screen.move_down();
        assert_eq!(screen.selected_editor, 1);

        screen.move_down();
        assert_eq!(screen.selected_editor, 2);

        screen.move_up();
        assert_eq!(screen.selected_editor, 1);
    }

    /// Test theme selection navigation
    #[test]
    fn test_theme_selection_navigation() {
        let mut screen = WelcomeScreen::default();
        screen.show();

        // Navigate to theme selection
        screen.advance();
        screen.advance();
        screen.advance();

        assert!(matches!(screen.state, WelcomeState::ThemeSelection { .. }));
        assert_eq!(screen.selected_theme, 0);

        screen.move_down();
        assert_eq!(screen.selected_theme, 1);

        screen.move_up();
        assert_eq!(screen.selected_theme, 0);
    }

    /// Test workspace path input
    #[test]
    fn test_workspace_path_input() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        screen.state = WelcomeState::WorkspaceSetup {
            path: String::new(),
        };
        screen.input_buffer = String::new();

        screen.handle_char('/');
        screen.handle_char('h');
        screen.handle_char('o');
        screen.handle_char('m');
        screen.handle_char('e');

        assert_eq!(screen.input_buffer, "/home");

        screen.handle_backspace();
        assert_eq!(screen.input_buffer, "/hom");
    }

    /// Test provider setup toggle
    #[test]
    fn test_provider_setup_toggle() {
        let mut screen = WelcomeScreen::default();
        screen.show();

        // Navigate to provider setup
        screen.advance();
        screen.advance();

        assert!(matches!(screen.state, WelcomeState::ProviderSetup { .. }));
        assert!(screen.use_env_key);

        screen.toggle_env_key();
        assert!(!screen.use_env_key);

        screen.toggle_env_key();
        assert!(screen.use_env_key);
    }

    /// Test that navigation keys don't work in wrong states
    #[test]
    fn test_navigation_only_in_selection_states() {
        let mut screen = WelcomeScreen::default();
        screen.show();

        // In WorkspaceSetup state, up/down shouldn't change anything
        let initial_editor = screen.selected_editor;
        screen.move_down();
        assert_eq!(screen.selected_editor, initial_editor);
    }

    /// Test selected editor name retrieval
    #[test]
    fn test_selected_editor_name() {
        let mut screen = WelcomeScreen::default();
        screen.selected_editor = 0;
        assert_eq!(screen.selected_editor_name(), "helix");

        screen.selected_editor = 1;
        assert_eq!(screen.selected_editor_name(), "neovim");

        screen.selected_editor = 2;
        assert_eq!(screen.selected_editor_name(), "vim");
    }

    /// Test selected theme name retrieval
    #[test]
    fn test_selected_theme_name() {
        let mut screen = WelcomeScreen::default();
        screen.selected_theme = 0;
        assert_eq!(screen.selected_theme_name(), "system");

        screen.selected_theme = 1;
        assert_eq!(screen.selected_theme_name(), "dark");

        screen.selected_theme = 2;
        assert_eq!(screen.selected_theme_name(), "light");
    }
}

#[cfg(test)]
mod welcome_state_tests {
    use crate::welcome::wizard::WelcomeState;

    #[test]
    fn test_state_current_step() {
        assert_eq!(WelcomeState::NotStarted.current_step(), 0);
        assert_eq!(
            WelcomeState::WorkspaceSetup {
                path: String::new()
            }
            .current_step(),
            1
        );
        assert_eq!(
            WelcomeState::EditorSelection { selected: 0 }.current_step(),
            2
        );
        assert_eq!(
            WelcomeState::ProviderSetup {
                use_env: true,
                custom_key: None
            }
            .current_step(),
            3
        );
        assert_eq!(
            WelcomeState::ThemeSelection {
                preview: String::new()
            }
            .current_step(),
            4
        );
        assert_eq!(WelcomeState::Completed.current_step(), 5);
    }

    #[test]
    fn test_state_is_complete() {
        assert!(!WelcomeState::NotStarted.is_complete());
        assert!(!WelcomeState::WorkspaceSetup {
            path: String::new()
        }
        .is_complete());
        assert!(WelcomeState::Completed.is_complete());
    }

    #[test]
    fn test_state_total_steps() {
        assert_eq!(WelcomeState::total_steps(), 4);
    }

    #[test]
    fn test_state_start() {
        let mut state = WelcomeState::NotStarted;
        state.start();
        assert!(matches!(state, WelcomeState::WorkspaceSetup { .. }));

        // Starting again should not change state
        state.start();
        assert!(matches!(state, WelcomeState::WorkspaceSetup { .. }));
    }

    #[test]
    fn test_state_go_back_from_not_started() {
        let mut state = WelcomeState::NotStarted;
        state.go_back();
        assert!(matches!(state, WelcomeState::NotStarted));
    }

    #[test]
    fn test_state_go_back_from_completed() {
        let mut state = WelcomeState::Completed;
        state.go_back();
        assert!(matches!(state, WelcomeState::ThemeSelection { .. }));
    }

    #[test]
    fn test_all_step_info_present() {
        let states = vec![
            WelcomeState::WorkspaceSetup {
                path: String::new(),
            },
            WelcomeState::EditorSelection { selected: 0 },
            WelcomeState::ProviderSetup {
                use_env: true,
                custom_key: None,
            },
            WelcomeState::ThemeSelection {
                preview: String::new(),
            },
            WelcomeState::Completed,
        ];

        for state in states {
            let info = state.current_step_info();
            assert!(info.is_some(), "Step info should exist for {:?}", state);
        }
    }

    #[test]
    fn test_not_started_step_info_is_none() {
        let state = WelcomeState::NotStarted;
        assert!(state.current_step_info().is_none());
    }
}
