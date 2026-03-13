//! UI Integration Tests for MaesterClaw
//!
//! These tests verify the Cockpit TUI integration with maestro-claw agent framework.
//!
//! Test Categories:
//! - Agent status display in MaesterClaw tab
//! - Session list showing active agent sessions
//! - Turn history display for selected session
//! - Real-time updates via event stream

#[cfg(test)]
mod cockpit_integration_tests {
    use maestro_claw::session::{Session, Thread, Turn, TurnRole};

    /// Test that agent status can be represented for UI display
    #[test]
    fn test_agent_status_display_ready() {
        // Status should show: Running, Idle, Error states
        let status = AgentStatus::Ready;
        assert_eq!(status.label(), "Ready");
        assert!(status.is_active());
    }

    #[test]
    fn test_agent_status_display_running() {
        let status = AgentStatus::Running {
            session_id: "sess-123".to_string(),
            turn_count: 5,
        };
        assert_eq!(status.label(), "Running");
        assert!(status.is_active());
    }

    #[test]
    fn test_agent_status_display_idle() {
        let status = AgentStatus::Idle;
        assert_eq!(status.label(), "Idle");
        assert!(!status.is_active());
    }

    #[test]
    fn test_agent_status_display_error() {
        let status = AgentStatus::Error {
            message: "Provider timeout".to_string(),
        };
        assert_eq!(status.label(), "Error");
        assert!(!status.is_active());
    }

    /// Test that session list can be rendered from active sessions
    #[test]
    fn test_session_list_rendering() {
        let sessions = create_test_sessions(3);
        let list = SessionList::from_sessions(&sessions);

        assert_eq!(list.items.len(), 3);
        assert!(list.items[0].contains("session-0"));
    }

    /// Test that turn history can be displayed for a selected session
    #[test]
    fn test_turn_history_display() {
        let mut session = Session::new();
        // add_thread() now returns &mut Thread directly
        let thread = session.add_thread();
        thread.add_turn(Turn::new(TurnRole::User, "Hello".to_string()));
        thread.add_turn(Turn::new(TurnRole::Assistant, "Hi there!".to_string()));
        thread.add_turn(Turn::new(TurnRole::User, "How are you?".to_string()));

        let history = TurnHistory::from_thread(thread);
        assert_eq!(history.turns.len(), 3);
        assert_eq!(history.turns[0].role, "User");
        assert_eq!(history.turns[0].preview, "Hello");
    }

    /// Test that turn preview is truncated for display
    #[test]
    fn test_turn_preview_truncation() {
        let long_content = "This is a very long turn content that should be truncated in the preview display for UI purposes";
        let turn = Turn::new(TurnRole::User, long_content.to_string());

        let preview = TurnPreview::from_turn(&turn, 40);
        assert!(preview.text.len() <= 43); // 40 + "..."
        assert!(preview.text.ends_with("..."));
    }

    /// Test tool call display in turn history
    #[test]
    fn test_tool_call_in_turn_display() {
        let mut turn = Turn::new(TurnRole::Assistant, "Let me check that.".to_string());
        turn.add_tool_call(
            "call-1".to_string(),
            "bash".to_string(),
            serde_json::json!({"cmd": "ls"}),
        );

        let display = TurnDisplay::from_turn(&turn);
        assert!(display.has_tool_calls);
        assert_eq!(display.tool_call_count, 1);
        assert_eq!(display.tool_names, vec!["bash"]);
    }

    /// Test session summary statistics
    #[test]
    fn test_session_summary_stats() {
        let mut session = Session::new();
        // add_thread() now returns &mut Thread directly
        let thread = session.add_thread();
        thread.add_turn(Turn::new(TurnRole::User, "Hello".to_string()));
        thread.add_turn(Turn::new(TurnRole::Assistant, "Hi!".to_string()));
        thread.add_turn(Turn::new(TurnRole::User, "Help me".to_string()));

        let stats = SessionStats::from_session(&session);
        assert_eq!(stats.total_turns, 3);
        assert_eq!(stats.user_turns, 2);
        assert_eq!(stats.assistant_turns, 1);
    }

    // Helper types for tests (to be implemented in actual code)

    #[derive(Debug, Clone, PartialEq)]
    enum AgentStatus {
        Ready,
        Running {
            session_id: String,
            turn_count: usize,
        },
        Idle,
        Error {
            message: String,
        },
    }

    impl AgentStatus {
        fn label(&self) -> &str {
            match self {
                Self::Ready => "Ready",
                Self::Running { .. } => "Running",
                Self::Idle => "Idle",
                Self::Error { .. } => "Error",
            }
        }

        fn is_active(&self) -> bool {
            matches!(self, Self::Ready | Self::Running { .. })
        }
    }

    struct SessionList {
        items: Vec<String>,
    }

    impl SessionList {
        fn from_sessions(sessions: &[Session]) -> Self {
            Self {
                items: sessions
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("session-{} ({})", i, s.id()))
                    .collect(),
            }
        }
    }

    struct TurnHistory {
        turns: Vec<TurnDisplay>,
    }

    impl TurnHistory {
        fn from_thread(thread: &Thread) -> Self {
            Self {
                turns: thread.turns().map(|t| TurnDisplay::from_turn(t)).collect(),
            }
        }
    }

    struct TurnPreview {
        text: String,
    }

    impl TurnPreview {
        fn from_turn(turn: &Turn, max_len: usize) -> Self {
            let content = &turn.content;
            let text = if content.len() > max_len {
                format!("{}...", &content[..max_len.saturating_sub(3)])
            } else {
                content.clone()
            };
            Self { text }
        }
    }

    struct TurnDisplay {
        role: String,
        preview: String,
        has_tool_calls: bool,
        tool_call_count: usize,
        tool_names: Vec<String>,
    }

    impl TurnDisplay {
        fn from_turn(turn: &Turn) -> Self {
            Self {
                role: format!("{:?}", turn.role),
                preview: TurnPreview::from_turn(turn, 50).text,
                has_tool_calls: !turn.tool_calls.is_empty(),
                tool_call_count: turn.tool_calls.len(),
                tool_names: turn.tool_calls.iter().map(|tc| tc.name.clone()).collect(),
            }
        }
    }

    struct SessionStats {
        total_turns: usize,
        user_turns: usize,
        assistant_turns: usize,
    }

    impl SessionStats {
        fn from_session(session: &Session) -> Self {
            let mut total = 0;
            let mut user = 0;
            let mut assistant = 0;

            for thread in session.threads() {
                for turn in thread.turns() {
                    total += 1;
                    match turn.role {
                        TurnRole::User => user += 1,
                        TurnRole::Assistant => assistant += 1,
                        _ => {}
                    }
                }
            }

            Self {
                total_turns: total,
                user_turns: user,
                assistant_turns: assistant,
            }
        }
    }

    fn create_test_sessions(count: usize) -> Vec<Session> {
        (0..count).map(|_| Session::new()).collect()
    }
}
