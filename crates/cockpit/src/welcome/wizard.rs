//! Welcome wizard state machine for first-time user onboarding

use serde::{Deserialize, Serialize};

/// Welcome wizard state machine
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WelcomeState {
    /// Wizard has not started
    #[default]
    NotStarted,
    /// Step 1: Workspace setup
    WorkspaceSetup { path: String },
    /// Step 2: Editor selection
    EditorSelection { selected: usize },
    /// Step 3: AI Provider setup (optional)
    ProviderSetup {
        use_env: bool,
        custom_key: Option<String>,
    },
    /// Step 4: Theme selection
    ThemeSelection { preview: String },
    /// Wizard completed
    Completed,
}

/// Welcome wizard step definition
#[derive(Clone, Debug)]
pub struct WelcomeStep {
    pub number: usize,
    pub title: String,
    pub description: String,
    pub help_text: String,
}

impl WelcomeStep {
    pub fn new(
        number: usize,
        title: impl Into<String>,
        description: impl Into<String>,
        help_text: impl Into<String>,
    ) -> Self {
        Self {
            number,
            title: title.into(),
            description: description.into(),
            help_text: help_text.into(),
        }
    }
}

impl WelcomeState {
    /// Get the current step number (1-indexed)
    pub fn current_step(&self) -> usize {
        match self {
            Self::NotStarted => 0,
            Self::WorkspaceSetup { .. } => 1,
            Self::EditorSelection { .. } => 2,
            Self::ProviderSetup { .. } => 3,
            Self::ThemeSelection { .. } => 4,
            Self::Completed => 5,
        }
    }

    /// Get total steps
    pub fn total_steps() -> usize {
        4
    }

    /// Check if wizard is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Get the current step info
    pub fn current_step_info(&self) -> Option<WelcomeStep> {
        match self {
            Self::NotStarted => None,
            Self::WorkspaceSetup { .. } => Some(WelcomeStep::new(
                1,
                "Workspace Setup",
                "Maestro needs a workspace directory for projects and tracks.",
                "[Tab: Next Field] [Enter: Accept] [Esc: Use Defaults]",
            )),
            Self::EditorSelection { .. } => Some(WelcomeStep::new(
                2,
                "Editor Selection",
                "Choose your preferred editor for opening files from Maestro.",
                "[↑↓: Navigate] [Enter: Select] [Esc: Skip]",
            )),
            Self::ProviderSetup { .. } => Some(WelcomeStep::new(
                3,
                "AI Provider Setup (Optional)",
                "Configure your AI provider API key for enhanced features.",
                "[Tab: Toggle Use Env] [Enter: Continue] [Esc: Skip]",
            )),
            Self::ThemeSelection { .. } => Some(WelcomeStep::new(
                4,
                "Theme Selection",
                "Choose a visual theme for the TUI.",
                "[↑↓: Preview] [Enter: Select] [Esc: Use Default]",
            )),
            Self::Completed => Some(WelcomeStep::new(
                5,
                "Setup Complete!",
                "Maestro is ready to use. Press any key to continue.",
                "[Enter: Start Maestro]",
            )),
        }
    }

    /// Transition to the next step
    pub fn advance(&mut self, workspace_path: &str, _selected_editor: usize, theme_name: &str) {
        *self = match self {
            Self::NotStarted => Self::WorkspaceSetup {
                path: workspace_path.to_string(),
            },
            Self::WorkspaceSetup { .. } => Self::EditorSelection { selected: 0 },
            Self::EditorSelection { .. } => Self::ProviderSetup {
                use_env: true,
                custom_key: None,
            },
            Self::ProviderSetup { .. } => Self::ThemeSelection {
                preview: theme_name.to_string(),
            },
            Self::ThemeSelection { .. } => Self::Completed,
            Self::Completed => Self::Completed,
        };
    }

    /// Go back to the previous step
    pub fn go_back(&mut self) {
        *self = match self {
            Self::NotStarted => Self::NotStarted,
            Self::WorkspaceSetup { .. } => Self::NotStarted,
            Self::EditorSelection { .. } => Self::WorkspaceSetup {
                path: super::default_workspace_path(),
            },
            Self::ProviderSetup { .. } => Self::EditorSelection { selected: 0 },
            Self::ThemeSelection { .. } => Self::ProviderSetup {
                use_env: true,
                custom_key: None,
            },
            Self::Completed => Self::ThemeSelection {
                preview: "system".to_string(),
            },
        };
    }

    /// Start the wizard
    pub fn start(&mut self) {
        if matches!(self, Self::NotStarted) {
            *self = Self::WorkspaceSetup {
                path: super::default_workspace_path(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_state_default() {
        let state = WelcomeState::default();
        assert_eq!(state, WelcomeState::NotStarted);
        assert_eq!(state.current_step(), 0);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_welcome_state_start() {
        let mut state = WelcomeState::default();
        state.start();
        assert!(matches!(state, WelcomeState::WorkspaceSetup { .. }));
        assert_eq!(state.current_step(), 1);
    }

    #[test]
    fn test_welcome_state_advance() {
        let mut state = WelcomeState::WorkspaceSetup {
            path: "/test".to_string(),
        };
        state.advance("/test", 0, "system");
        assert!(matches!(state, WelcomeState::EditorSelection { .. }));
        assert_eq!(state.current_step(), 2);
    }

    #[test]
    fn test_welcome_state_complete_flow() {
        let mut state = WelcomeState::default();
        state.start();
        assert_eq!(state.current_step(), 1);

        state.advance("/test", 0, "system");
        assert_eq!(state.current_step(), 2);

        state.advance("/test", 0, "system");
        assert_eq!(state.current_step(), 3);

        state.advance("/test", 0, "system");
        assert_eq!(state.current_step(), 4);

        state.advance("/test", 0, "system");
        assert_eq!(state.current_step(), 5);
        assert!(state.is_complete());
    }

    #[test]
    fn test_welcome_state_go_back() {
        let mut state = WelcomeState::EditorSelection { selected: 0 };
        state.go_back();
        assert!(matches!(state, WelcomeState::WorkspaceSetup { .. }));

        state.go_back();
        assert!(matches!(state, WelcomeState::NotStarted));
    }

    #[test]
    fn test_welcome_state_total_steps() {
        assert_eq!(WelcomeState::total_steps(), 4);
    }

    #[test]
    fn test_current_step_info() {
        let state = WelcomeState::WorkspaceSetup {
            path: "/test".to_string(),
        };
        let info = state.current_step_info().unwrap();
        assert_eq!(info.number, 1);
        assert!(info.title.contains("Workspace"));
    }

    #[test]
    fn test_completed_step_info() {
        let state = WelcomeState::Completed;
        let info = state.current_step_info().unwrap();
        assert_eq!(info.number, 5);
        assert!(info.title.contains("Complete"));
    }
}
