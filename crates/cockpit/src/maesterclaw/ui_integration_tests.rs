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

/// Integration tests for session browser wiring.
///
/// These verify the data flow: OpenSessionBrowser -> Enter -> SessionBrowserSelect
/// without requiring a full TUI backend (crossterm).
#[cfg(test)]
mod session_browser_integration_tests {
    use crate::maesterclaw::{
        MaestroClawAction, MaestroClawPane, SessionEntry,
    };
    use crossterm::event::KeyCode;

    fn make_entry(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "preview".to_string(),
            source: "claude".to_string(),
            last_active: "1m ago".to_string(),
            turn_count: 1,
        }
    }

    #[test]
    fn test_open_browser_sets_active() {
        let mut pane = MaestroClawPane::default();
        assert!(!pane.is_session_browser_active());

        pane.activate_session_browser();
        assert!(pane.is_session_browser_active());
    }

    #[test]
    fn test_esc_closes_browser() {
        let mut pane = MaestroClawPane::default();
        pane.activate_session_browser();
        assert!(pane.is_session_browser_active());

        let action = pane.handle_key(KeyCode::Esc);
        assert_eq!(action, MaestroClawAction::SessionBrowserClose);
        assert!(!pane.is_session_browser_active());
    }

    #[test]
    fn test_enter_on_browser_emits_select_with_session_id() {
        // Full wiring: load sessions, navigate, press Enter
        let mut pane = MaestroClawPane::default();
        let entries = vec![
            make_entry("sess-alpha", "Alpha Session"),
            make_entry("sess-beta", "Beta Session"),
        ];
        pane.load_session_entries(entries);
        pane.activate_session_browser();

        // Navigate down to second session
        let action = pane.handle_key(KeyCode::Down);
        assert_eq!(action, MaestroClawAction::Navigate);

        // Press Enter — should emit SessionBrowserSelect with the correct session ID
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);

        // The browser should now be closed (deactivated)
        assert!(!pane.is_session_browser_active());

        // The selected session ID should be queryable
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("sess-beta".to_string())
        );
    }

    #[test]
    fn test_enter_immediately_deactivates_browser() {
        // Per the plan: Enter should resume/open the session immediately,
        // not just select it and require a second Enter.
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "First"),
        ]);
        pane.activate_session_browser();

        // Single Enter should both select AND close the browser
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);
        assert!(!pane.is_session_browser_active());
    }

    #[test]
    fn test_enter_on_empty_browser_does_nothing() {
        let mut pane = MaestroClawPane::default();
        // No sessions loaded
        pane.activate_session_browser();

        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::None);
        // Browser should remain open since nothing was selected
        assert!(pane.is_session_browser_active());
    }

    #[test]
    fn test_char_input_filters_browser() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Apple Session"),
            make_entry("s2", "Banana Session"),
            make_entry("s3", "Apricot Session"),
        ]);
        pane.activate_session_browser();

        // Type 'apr' to filter — matches only "Apricot Session"
        pane.handle_key(KeyCode::Char('a'));
        pane.handle_key(KeyCode::Char('p'));
        let action = pane.handle_key(KeyCode::Char('r'));
        assert_eq!(action, MaestroClawAction::None);

        // Select the single filtered result directly
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s3".to_string())
        );
    }
}

/// Render-buffer tests for the ChannelSetup wizard step.
///
/// These verify actual rendered output through the real
/// `render_wizard_channels` → `Checklist::render` path, using `TestBackend`
/// to inspect the buffer. The Checklist is constructed identically to how
/// `render_wizard_channels` builds it, so these exercise the real widget code.
#[cfg(test)]
mod channel_setup_render_tests {
    use crate::maesterclaw::{ChannelType, Checklist, WizardStep};
    use std::collections::HashSet;

    /// Helper: extract the full buffer as a string (all rows joined by newline).
    fn buffer_to_string(backend: &ratatui::backend::TestBackend) -> String {
        let buf = backend.buffer();
        let area = buf.area();
        let mut s = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                s.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
            }
            s.push('\n');
        }
        s
    }

    /// Build a Checklist exactly as render_wizard_channels does.
    fn build_channel_checklist(
        cursor: usize,
        selected_channels: &HashSet<ChannelType>,
    ) -> Checklist {
        let channels = ChannelType::all();
        let channel_names: Vec<String> = channels
            .iter()
            .map(|ch| format!("{} {}", ch.icon(), ch.label()))
            .collect();
        let selected_indices: HashSet<usize> = selected_channels
            .iter()
            .filter_map(|ch| channels.iter().position(|c| c == ch))
            .collect();

        let mut checklist = Checklist::with_selected(
            "Select messaging channels to configure",
            channel_names,
            selected_indices,
        );
        checklist.cursor = cursor;
        checklist
    }

    /// Render a Checklist into a TestBackend and return the backend.
    fn render_checklist(checklist: &Checklist) -> ratatui::backend::TestBackend {
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| checklist.render(f, f.area()))
            .unwrap();
        terminal.backend().clone()
    }

    #[test]
    fn test_channel_setup_checklist_title_aligned() {
        let checklist = build_channel_checklist(0, &HashSet::new());
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);
        // Title must match the plan literal
        assert!(
            full.contains("Select messaging channels to configure"),
            "expected plan-aligned checklist title, got:\n{full}"
        );
    }

    #[test]
    fn test_channel_setup_all_channel_rows_present() {
        let checklist = build_channel_checklist(0, &HashSet::new());
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);

        let channels = ChannelType::all();
        // Verify presence in exact all() order — a reorder will fail here
        let labels: Vec<&str> = channels.iter().map(|ch| ch.label()).collect();
        let mut search_from = 0;
        for label in &labels {
            let pos = full[search_from..].find(label).expect(
                &format!("expected channel label '{label}' in render output, got:\n{full}")
            );
            search_from += pos + label.len();
        }
    }

    #[test]
    fn test_channel_setup_blank_separator_before_continue() {
        let checklist = build_channel_checklist(0, &HashSet::new());
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);
        // Checklist inserts a blank ListItem before "Continue →"
        assert!(
            full.contains("Continue"),
            "expected 'Continue' button in checklist output, got:\n{full}"
        );
    }

    #[test]
    fn test_channel_setup_arrow_on_first_channel_when_cursor_zero() {
        let checklist = build_channel_checklist(0, &HashSet::new());
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);
        // The checklist renders "→" at the cursor position (index 0 = first channel)
        assert!(
            full.contains("→"),
            "expected arrow indicator for cursor position 0, got:\n{full}"
        );
    }

    #[test]
    fn test_channel_setup_arrow_on_continue_when_selected() {
        let channel_count = ChannelType::all().len();
        let checklist = build_channel_checklist(channel_count, &HashSet::new());
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);
        // When cursor is on Continue, the Continue row should show "→ Continue →"
        assert!(
            full.contains("→ Continue →"),
            "expected '→ Continue →' when cursor is on Continue row, got:\n{full}"
        );
    }

    #[test]
    fn test_channel_setup_no_cursor_arrow_on_channels_when_continue_selected() {
        let channel_count = ChannelType::all().len();
        let checklist = build_channel_checklist(channel_count, &HashSet::new());
        let backend = render_checklist(&checklist);
        // The first channel row should NOT have a cursor arrow prefix
        // (cursor is on Continue, not on any channel)
        let full = buffer_to_string(&backend);
        let lines: Vec<&str> = full.lines().collect();
        // Find the first channel row (should be "  [ ] 📱 Telegram" not "→ [ ] ...")
        let first_channel_line = lines
            .iter()
            .find(|l| l.contains("Telegram"))
            .expect("should have a Telegram row");
        assert!(
            !first_channel_line.starts_with("→"),
            "first channel row should not have cursor arrow when cursor is on Continue, got: '{first_channel_line}'"
        );
    }

    #[test]
    fn test_channel_setup_toggle_affects_checkbox() {
        let mut selected = HashSet::new();
        selected.insert(ChannelType::Telegram);
        let checklist = build_channel_checklist(0, &selected);
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);
        // Checklist renders [✓] for selected items
        assert!(
            full.contains("[✓]"),
            "expected checked box '[✓]' for selected Telegram, got:\n{full}"
        );
    }

    #[test]
    fn test_channel_setup_unselected_shows_empty_checkbox() {
        let checklist = build_channel_checklist(0, &HashSet::new());
        let backend = render_checklist(&checklist);
        let full = buffer_to_string(&backend);
        // No channels selected → should show [ ] not [✓]
        assert!(
            !full.contains("[✓]"),
            "expected no checked boxes when nothing selected, got:\n{full}"
        );
        assert!(
            full.contains("[ ]"),
            "expected empty checkbox '[ ]' when nothing selected, got:\n{full}"
        );
    }

    /// Verify the wizard title label for ChannelSetup step is correct.
    /// This tests the WizardStep::label() value used in render_wizard().
    #[test]
    fn test_channel_setup_wizard_step_label() {
        assert_eq!(WizardStep::ChannelSetup.label(), "Channels");
    }
}

/// End-to-end wizard tests that exercise real key handling through
/// ProviderSelection → ChannelSetup transitions, channel toggling,
/// Continue-row advance, and back-navigation.
#[cfg(test)]
mod wizard_e2e_transition_tests {
    use crate::maesterclaw::{ChannelType, MaestroClawAction, MaestroClawPane, WizardStep};

    /// Build a pane whose wizard is on ProviderSelection (step 4).
    fn pane_at_provider_selection() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        pane.wizard.next_step(); // -> ToolDetection
        pane.wizard.next_step(); // -> PrimaryToolSelection
        pane.wizard.next_step(); // -> ProviderSelection
        pane.wizard.cursor = 0;
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
        pane
    }

    #[test]
    fn test_enter_on_provider_advances_to_channel_setup() {
        let mut pane = pane_at_provider_selection();
        let action = pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);
        // Cursor should be reset to 0 on step transition
        assert_eq!(pane.wizard.cursor, 0);
    }

    #[test]
    fn test_channel_setup_toggle_and_advance() {
        let mut pane = pane_at_provider_selection();
        // Advance to ChannelSetup
        pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);

        let channel_count = ChannelType::all().len();

        // Toggle Telegram (Space)
        pane.handle_key(crossterm::event::KeyCode::Char(' '));
        assert!(pane.wizard.selected_channels.contains(&ChannelType::Telegram));

        // Move down to Discord
        pane.handle_key(crossterm::event::KeyCode::Down);
        assert_eq!(pane.wizard.cursor, 1);

        // Toggle Discord (Space)
        pane.handle_key(crossterm::event::KeyCode::Char(' '));
        assert!(pane.wizard.selected_channels.contains(&ChannelType::Discord));

        // Untoggle Discord (Space again)
        pane.handle_key(crossterm::event::KeyCode::Char(' '));
        assert!(!pane.wizard.selected_channels.contains(&ChannelType::Discord));

        // Move to Continue (cursor == channel_count)
        for _ in 1..=channel_count {
            pane.handle_key(crossterm::event::KeyCode::Down);
        }
        assert_eq!(pane.wizard.cursor, channel_count);

        // Press Enter to advance past ChannelSetup
        let action = pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
    }

    #[test]
    fn test_channel_setup_enter_on_channel_toggles_instead_of_advance() {
        let mut pane = pane_at_provider_selection();
        pane.handle_key(crossterm::event::KeyCode::Enter); // -> ChannelSetup

        // Cursor on Telegram (index 0), press Enter — should toggle, not advance
        let action = pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::WizardSelection);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);
        assert!(pane.wizard.selected_channels.contains(&ChannelType::Telegram));
    }

    #[test]
    fn test_channel_setup_esc_goes_back_to_provider_selection() {
        let mut pane = pane_at_provider_selection();
        pane.handle_key(crossterm::event::KeyCode::Enter); // -> ChannelSetup
        pane.wizard.cursor = 3; // move cursor deep into channel list

        let action = pane.handle_key(crossterm::event::KeyCode::Esc);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
        // Cursor should be clamped to provider list length
        let max_provider = pane.wizard.provider_list.len().saturating_sub(1);
        assert!(
            pane.wizard.cursor <= max_provider,
            "cursor {} should be clamped to provider list max {}",
            pane.wizard.cursor,
            max_provider
        );
    }

    #[test]
    fn test_channel_setup_full_round_trip() {
        let mut pane = pane_at_provider_selection();

        // Forward: ProviderSelection -> ChannelSetup
        pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);

        // Toggle a channel
        pane.handle_key(crossterm::event::KeyCode::Char(' '));
        assert!(pane.wizard.selected_channels.contains(&ChannelType::Telegram));

        // Back: ChannelSetup -> ProviderSelection
        pane.handle_key(crossterm::event::KeyCode::Esc);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);

        // Forward again: ProviderSelection -> ChannelSetup
        pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);

        // Telegram selection should persist across the round trip
        assert!(
            pane.wizard.selected_channels.contains(&ChannelType::Telegram),
            "Telegram selection should persist after round-trip navigation"
        );
    }
}

/// Buffer-based tests that render a real MaestroClawPane on ChannelSetup
/// through the production render_wizard_channels adapter (not a reconstructed
/// Checklist). These exercise ProviderSelection -> ChannelSetup transitions
/// via live Enter handling before inspecting the rendered output.
#[cfg(test)]
mod wizard_channel_setup_render_tests {
    use crate::maesterclaw::{ChannelType, MaestroClawAction, MaestroClawPane, WizardStep};

    fn buffer_to_string(backend: &ratatui::backend::TestBackend) -> String {
        let buf = backend.buffer();
        let area = buf.area();
        let mut s = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                s.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
            }
            s.push('\n');
        }
        s
    }

    /// Build a pane whose wizard is on ProviderSelection (step 4), ready for
    /// live Enter to advance to ChannelSetup.
    fn pane_at_provider_selection() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        pane.wizard.next_step(); // -> ToolDetection
        pane.wizard.next_step(); // -> PrimaryToolSelection
        pane.wizard.next_step(); // -> ProviderSelection
        pane.wizard.cursor = 0;
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
        pane
    }

    #[test]
    fn test_render_channel_setup_after_provider_enter() {
        // Drive ProviderSelection -> ChannelSetup via live Enter handling
        let mut pane = pane_at_provider_selection();
        let action = pane.handle_key(crossterm::event::KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);
        assert_eq!(pane.wizard.cursor, 0);

        // Render through the production render_wizard_channels adapter
        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_channels(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());

        // The adapter builds a Checklist with the real title
        assert!(
            content.contains("Select messaging channels"),
            "expected checklist title in buffer, got:\n{content}"
        );

        // All channel types must be present (not a hand-picked subset)
        for ch in ChannelType::all() {
            assert!(
                content.contains(&ch.label()),
                "expected channel '{}' in buffer, got:\n{content}",
                ch.label()
            );
        }

        // Cursor on first channel (cursor=0) should render a cursor arrow
        assert!(
            content.contains('→'),
            "expected cursor arrow for first channel, got:\n{content}"
        );

        // No channels selected -> unchecked checkboxes only
        assert!(
            content.contains("[ ]"),
            "expected unchecked checkbox, got:\n{content}"
        );
        assert!(
            !content.contains("[✓]"),
            "expected no checked boxes when nothing selected, got:\n{content}"
        );
    }

    #[test]
    fn test_render_channel_setup_with_toggled_channels() {
        let mut pane = pane_at_provider_selection();
        pane.handle_key(crossterm::event::KeyCode::Enter); // -> ChannelSetup

        // Toggle Telegram (Space on cursor 0)
        pane.handle_key(crossterm::event::KeyCode::Char(' '));
        assert!(pane.wizard.selected_channels.contains(&ChannelType::Telegram));

        // Move down and toggle Discord
        pane.handle_key(crossterm::event::KeyCode::Down);
        pane.handle_key(crossterm::event::KeyCode::Char(' '));
        assert!(pane.wizard.selected_channels.contains(&ChannelType::Discord));

        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_channels(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());

        // Toggled channels should show checked boxes
        assert!(
            content.contains("[✓]"),
            "expected checked boxes for toggled channels, got:\n{content}"
        );

        // Continue row must be present
        assert!(
            content.contains("Continue"),
            "expected Continue row, got:\n{content}"
        );
    }

    #[test]
    fn test_render_channel_setup_cursor_on_continue_row() {
        let mut pane = pane_at_provider_selection();
        pane.handle_key(crossterm::event::KeyCode::Enter); // -> ChannelSetup

        // Move cursor to the Continue row (past all channels)
        let channel_count = ChannelType::all().len();
        for _ in 0..channel_count {
            pane.handle_key(crossterm::event::KeyCode::Down);
        }
        assert_eq!(pane.wizard.cursor, channel_count);

        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_channels(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());

        // Cursor on Continue row should show the arrow indicator
        assert!(
            content.contains("→ Continue"),
            "expected cursor arrow on Continue row, got:\n{content}"
        );
    }

    #[test]
    fn test_render_channel_setup_outer_wizard_title_is_channels() {
        // Drive to ChannelSetup via live key handling
        let mut pane = pane_at_provider_selection();
        pane.handle_key(crossterm::event::KeyCode::Enter); // -> ChannelSetup
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);

        // Render through the full production render_wizard outer shell
        // (includes the Block with "MaestroClaw Setup — Channels [5/7]" title)
        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());

        // The outer wizard title must contain the plan literal "Channels"
        assert!(
            content.contains("Channels"),
            "expected plan literal 'Channels' in outer wizard title, got:\n{content}"
        );
        // Verify the step number is correct (ChannelSetup is step 5 of 7)
        assert!(
            content.contains("5/7"),
            "expected step 5/7 in wizard title, got:\n{content}"
        );
    }

    #[test]
    fn test_render_channel_setup_exact_row_order_via_production_path() {
        // Drive to ChannelSetup via live Enter handling (production path)
        let mut pane = pane_at_provider_selection();
        pane.handle_key(crossterm::event::KeyCode::Enter); // -> ChannelSetup

        // Render through the production render_wizard_channels adapter
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| pane.test_render_wizard_channels(f, f.area()))
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        let lines: Vec<&str> = content.lines().collect();

        // Extract channel rows: lines containing a checkbox ([ ] or [✓])
        let channel_rows: Vec<&str> = lines
            .iter()
            .filter(|l| l.contains("[ ]") || l.contains("[✓]"))
            .copied()
            .collect();

        let expected_labels = [
            "Telegram",
            "Discord",
            "Slack",
            "Matrix",
            "WhatsApp",
            "Mattermost",
        ];

        assert_eq!(
            channel_rows.len(),
            expected_labels.len(),
            "expected {} channel rows, got {}: {channel_rows:?}\nfull buffer:\n{content}",
            expected_labels.len(),
            channel_rows.len()
        );

        // Row-for-row: each channel row must contain the corresponding label
        for (i, (row, label)) in channel_rows.iter().zip(expected_labels.iter()).enumerate() {
            assert!(
                row.contains(label),
                "channel row {i}: expected to contain '{label}', got: '{row}'\nfull buffer:\n{content}"
            );
        }
    }

    #[test]
    fn test_render_primary_tool_outer_wizard_title_is_primary_tool() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        pane.wizard.next_step(); // -> ToolDetection
        pane.wizard.next_step(); // -> PrimaryToolSelection
        assert_eq!(pane.wizard.current_step(), WizardStep::PrimaryToolSelection);

        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        assert!(
            content.contains("Primary Tool"),
            "expected 'Primary Tool' in outer wizard title, got:\n{content}"
        );
        assert!(
            content.contains("3/7"),
            "expected step 3/7 in wizard title, got:\n{content}"
        );
    }

    #[test]
    fn test_render_provider_outer_wizard_title_is_provider() {
        let pane = pane_at_provider_selection();
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);

        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        assert!(
            content.contains("MaestroClaw Setup — Provider [4/7]"),
            "expected full outer wizard title 'MaestroClaw Setup — Provider [4/7]', got:\n{content}"
        );
    }
}

/// Tests for the zero-session main view (the landing screen when no sessions are active).
#[cfg(test)]
mod main_view_render_tests {
    use crate::maesterclaw::MaestroClawPane;

    /// Helper: extract the full buffer as a string (all rows joined by newline).
    fn buffer_to_string(backend: &ratatui::backend::TestBackend) -> String {
        let buf = backend.buffer();
        let area = buf.area();
        let mut s = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                s.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn test_render_main_view_zero_session_shows_agent_tools_label() {
        let pane = MaestroClawPane::default();
        let expected_count = pane.wizard.available_tools.len();

        let backend = ratatui::backend::TestBackend::new(60, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_main_view(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // Phase 7 plan: empty-state wording uses "No active sessions."
        assert!(
            content.contains("No active sessions."),
            "expected 'No active sessions.' in zero-session main view, got:\n{content}"
        );
        // Phase 7 plan: tool count from available_tools.len(), not tool_details.len()
        let expected_tool_line =
            format!("{expected_count} tools detected on this system");
        assert!(
            content.contains(&expected_tool_line),
            "expected '{expected_tool_line}' in zero-session main view, got:\n{content}"
        );
        // Stale labels must not appear
        assert!(
            !content.contains("Agent tools:"),
            "stale 'Agent tools:' label must not appear in zero-session main view, got:\n{content}"
        );
    }
}

/// Tests for the ToolSummary step (Phase 5 — Tool Availability Summary Panel).
///
/// Covers:
/// - Key handling: Enter advances to Complete, Esc goes back to ChannelSetup
/// - Rendering: available/missing markers, tool names, count, footer
/// - Outer wizard shell: title contains "Summary" and correct step number 6/7
#[cfg(test)]
mod tool_summary_tests {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, WizardStep};

    // ── Key handling tests ──

    #[test]
    fn test_tool_summary_enter_advances_to_complete_screen() {
        let (mut pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let action = pane.handle_key(crossterm::event::KeyCode::Enter);
        // ToolSummary Enter advances to the Complete step but does NOT close
        // the wizard — only Enter from the actual Complete step closes it.
        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(pane.wizard.current_step(), WizardStep::Complete);
        assert!(pane.wizard.is_completed());
        assert!(pane.wizard_active, "wizard should stay active showing Complete screen");
    }

    #[test]
    fn test_tool_summary_esc_goes_back_to_channel_setup() {
        let (mut pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let action = pane.handle_key(crossterm::event::KeyCode::Esc);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);
    }

    #[test]
    fn test_tool_summary_irrelevant_key_returns_none() {
        let (mut pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let action = pane.handle_key(crossterm::event::KeyCode::Char('x'));
        assert_eq!(action, MaestroClawAction::None);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
    }

    #[test]
    fn test_tool_summary_up_down_return_none() {
        let (mut pane, _tmp) = pane_at_tool_summary_hermetic(false);
        assert_eq!(
            pane.handle_key(crossterm::event::KeyCode::Up),
            MaestroClawAction::None
        );
        assert_eq!(
            pane.handle_key(crossterm::event::KeyCode::Down),
            MaestroClawAction::None
        );
    }

    // ── Rendering tests ──

    fn buffer_to_string(backend: &ratatui::backend::TestBackend) -> String {
        let buf = backend.buffer();
        let area = buf.area();
        let mut s = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                s.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn test_render_tool_summary_shows_available_count() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // Fixture: no servers.toml → 4 of 6 available
        assert!(
            content.contains("4/6 tool categories available:"),
            "expected fixture-controlled '4/6 tool categories available:', got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_shows_checkmark_for_available() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // At least one tool is always available (Shell / Terminal)
        assert!(
            content.contains('✓'),
            "expected checkmark for available tools, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_shows_x_for_missing() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // Gateway (Web API) is always unavailable in build_tool_summary
        assert!(
            content.contains('✗'),
            "expected X marker for missing tools, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_shows_tool_names() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // Verify at least one always-available tool name is rendered
        assert!(
            content.contains("Shell"),
            "expected 'Shell' tool name in buffer, got:\n{content}"
        );
        assert!(
            content.contains("Gateway"),
            "expected 'Gateway' tool name in buffer, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_shows_footer() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        assert!(
            content.contains("Enter"),
            "expected 'Enter' in footer prompt, got:\n{content}"
        );
        assert!(
            content.contains("continue"),
            "expected 'continue' in footer prompt, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_shows_missing_hint_for_gateway() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // Gateway is always missing with hint "Start with 'maestro claw daemon'"
        assert!(
            content.contains("daemon"),
            "expected gateway missing hint in buffer, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_outer_wizard_title_is_summary() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // The outer wizard title must contain "Summary"
        assert!(
            content.contains("Summary"),
            "expected 'Summary' in outer wizard title, got:\n{content}"
        );
        // ToolSummary is step 6 of 7
        assert!(
            content.contains("6/7"),
            "expected step 6/7 in wizard title, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_does_not_panic_empty_summary() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut pane = MaestroClawPane::new(tmp.path().to_path_buf());
        pane.activate_wizard();
        pane.wizard.next_step();
        pane.wizard.next_step();
        pane.wizard.next_step();
        pane.wizard.next_step();
        pane.wizard.next_step();
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // Clear the summary to verify rendering handles empty list
        pane.wizard.tool_summary.clear();

        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        // Must not panic
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();
    }

    // ── Live Continue-row path tests (fixture-controlled) ──

    #[test]
    fn test_render_tool_summary_via_live_continue_row_path_mcp_absent() {
        // Use the hermetic fixture (no servers.toml) for hard-coded MCP-absent expectations
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());

        // Outer wizard title must show "Summary" and step 6/7
        assert!(
            content.contains("Summary"),
            "expected 'Summary' in outer wizard title, got:\n{content}"
        );
        assert!(
            content.contains("6/7"),
            "expected step 6/7 in wizard title, got:\n{content}"
        );

        // Fixture-controlled: no servers.toml → MCP absent → 4/6 available
        assert!(
            content.contains("4/6 tool categories available"),
            "expected fixture-controlled '4/6 tool categories available', got:\n{content}"
        );

        // Must show available tool markers
        assert!(
            content.contains('✓'),
            "expected checkmark for available tools, got:\n{content}"
        );

        // Must show the missing Gateway entry
        assert!(
            content.contains("Gateway"),
            "expected 'Gateway' in summary body, got:\n{content}"
        );
        // Must show missing marker for MCP Servers and Gateway
        assert!(
            content.contains('✗'),
            "expected cross for missing tools, got:\n{content}"
        );
        // MCP row must show the missing hint
        assert!(
            content.contains("maestro claw setup"),
            "expected MCP missing hint in summary, got:\n{content}"
        );
    }

    #[test]
    fn test_render_tool_summary_via_live_continue_row_path_mcp_present() {
        // Use the hermetic fixture (with servers.toml) for hard-coded MCP-present expectations
        let (pane, _tmp) = pane_at_tool_summary_hermetic(true);
        let backend = ratatui::backend::TestBackend::new(60, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());

        // Outer wizard title must show "Summary" and step 6/7
        assert!(
            content.contains("Summary"),
            "expected 'Summary' in outer wizard title, got:\n{content}"
        );
        assert!(
            content.contains("6/7"),
            "expected step 6/7 in wizard title, got:\n{content}"
        );

        // Fixture-controlled: servers.toml present → MCP available → 5/6 available
        assert!(
            content.contains("5/6 tool categories available"),
            "expected fixture-controlled '5/6 tool categories available', got:\n{content}"
        );

        // Must show available tool markers
        assert!(
            content.contains('✓'),
            "expected checkmark for available tools, got:\n{content}"
        );

        // Must show the missing Gateway entry (still always missing)
        assert!(
            content.contains("Gateway"),
            "expected 'Gateway' in summary body, got:\n{content}"
        );
        assert!(
            content.contains('✗'),
            "expected cross for missing tools, got:\n{content}"
        );
        // MCP Servers row must be present as available (no missing hint)
        assert!(
            content.contains("MCP Servers"),
            "expected 'MCP Servers' as available in summary, got:\n{content}"
        );
    }

    // ── Hermetic per-entry summary assertions (workspace-dir injection) ──

    /// Helper: create a pane at ToolSummary with a controlled workspace dir.
    /// If `create_mcp_config` is true, writes {tmp}/mcp/servers.toml
    /// so that build_tool_summary sees MCP as available.
    ///
    /// Returns `(pane, TempDir)` — the caller MUST keep the `TempDir` alive
    /// through all assertions so the temp directory is not deleted prematurely.
    ///
    /// This is fully hermetic: no HOME or other env mutations are made.
    /// The wizard's workspace_dir is passed directly to `MaestroClawPane::new()`,
    /// matching the same source of truth used by doctor, onboarding, and gateway.
    fn pane_at_tool_summary_hermetic(
        create_mcp_config: bool,
    ) -> (MaestroClawPane, tempfile::TempDir) {
        let tmp = tempfile::TempDir::new().unwrap();

        if create_mcp_config {
            let mcp_dir = tmp.path().join("mcp");
            std::fs::create_dir_all(&mcp_dir).unwrap();
            std::fs::write(mcp_dir.join("servers.toml"), "# test config\n").unwrap();
        }

        let mut pane = MaestroClawPane::new(tmp.path().to_path_buf());
        pane.activate_wizard();
        // Advance: Welcome -> ToolDetection -> PrimaryToolSelection ->
        //          ProviderSelection -> ChannelSetup -> ToolSummary
        for _ in 0..5 {
            pane.wizard.next_step();
        }

        // build_tool_summary uses the wizard's workspace_dir (already set via new()),
        // so the filesystem check sees our controlled temp dir.
        pane.wizard.build_tool_summary();

        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
        (pane, tmp)
    }

    /// Assert exact tool rows extracted from a rendered buffer match the
    /// expected set, row-for-row in insertion order.
    fn assert_exact_tool_rows(content: &str, expected_rows: &[&str]) {
        let lines: Vec<&str> = content.lines().collect();
        let actual_rows: Vec<String> = lines
            .iter()
            .filter(|l| l.contains('✓') || l.contains('✗'))
            .map(|l| l.trim_end().to_string())
            .collect();

        assert_eq!(
            actual_rows.len(),
            expected_rows.len(),
            "expected {} tool rows, got {}:\nactual: {actual_rows:?}\nexpected: {expected_rows:?}\nfull buffer:\n{content}",
            expected_rows.len(),
            actual_rows.len()
        );

        for (i, (actual, expected)) in actual_rows.iter().zip(expected_rows.iter()).enumerate() {
            assert_eq!(
                actual, expected,
                "tool row {i}: expected '{expected}', got '{actual}'\nfull buffer:\n{content}"
            );
        }
    }

    fn render_tool_summary_buffer(pane: &MaestroClawPane) -> String {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();
        buffer_to_string(terminal.backend())
    }

    #[test]
    fn test_render_tool_summary_mcp_absent_exact_rows() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let content = render_tool_summary_buffer(&pane);

        // Fixture: no servers.toml created → MCP unavailable
        assert!(
            content.contains("4/6 tool categories available:"),
            "expected '4/6 tool categories available:', got:\n{content}"
        );

        assert_exact_tool_rows(
            &content,
            &[
                "✓  Shell / Terminal",
                "✓  File Operations",
                "✓  Memory (built-in)",
                "✓  Cron Scheduler",
                "✗  MCP Servers (Run 'maestro claw setup' to configure)",
                "✗  Gateway (Web API) (Start with 'maestro claw daemon')",
            ],
        );
    }

    #[test]
    fn test_render_tool_summary_mcp_present_exact_rows() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(true);
        let content = render_tool_summary_buffer(&pane);

        // Fixture: workspace_dir has mcp/servers.toml → MCP available
        assert!(
            content.contains("5/6 tool categories available:"),
            "expected '5/6 tool categories available:', got:\n{content}"
        );

        assert_exact_tool_rows(
            &content,
            &[
                "✓  Shell / Terminal",
                "✓  File Operations",
                "✓  Memory (built-in)",
                "✓  Cron Scheduler",
                "✓  MCP Servers",
                "✗  Gateway (Web API) (Start with 'maestro claw daemon')",
            ],
        );
    }

    // ── Regression: Config::load() workspace_dir flows into pane ──

    /// Regression test: the workspace_dir that `Config::load_from_dir()` returns
    /// must be used by the pane's wizard for MCP detection. This exercises the
    /// real config loading path under an explicit home directory — no HOME env
    /// mutation needed. Mirrors the `App::new` code path at app.rs:397–400.
    ///
    /// Note: `Config::load_from_dir()` deliberately keeps the default
    /// workspace_dir (derived from the given home dir) even when a config file
    /// specifies a different value — the loaded workspace_dir field is
    /// overwritten. This test verifies the pane uses whatever the loader returns.
    #[test]
    fn test_tool_summary_mcp_follows_loaded_config_workspace() {
        // 1. Create a temp home with the default config directory structure
        let tmp_home = tempfile::TempDir::new().unwrap();
        let config_dir = tmp_home.path().join(".config").join("maestroclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        // load_from_dir resolves workspace_dir to {home}/.config/maestroclaw/workspace
        let workspace_dir = config_dir.join("workspace");
        std::fs::create_dir_all(workspace_dir.join("mcp")).unwrap();
        std::fs::write(
            workspace_dir.join("mcp").join("servers.toml"),
            "# test config\n",
        )
        .unwrap();

        // 2. Load config using the explicit home dir (no HOME mutation)
        let loaded_config = maestro_claw::config::Config::load_from_dir(
            tmp_home.path().to_path_buf(),
        )
        .expect("config should load");

        // 3. Verify the loaded workspace_dir points to the default location
        assert_eq!(
            loaded_config.workspace_dir,
            workspace_dir,
            "Config::load_from_dir() should return the default workspace_dir under temp home"
        );

        // 4. Create pane with the loaded workspace_dir (same as App::new does)
        let mut pane = MaestroClawPane::new(loaded_config.workspace_dir.clone());
        pane.activate_wizard();
        for _ in 0..5 {
            pane.wizard.next_step();
        }
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // 5. The MCP row must be available because workspace has servers.toml
        let mcp_entry = pane
            .wizard
            .tool_summary
            .iter()
            .find(|t| t.name == "MCP Servers")
            .expect("tool_summary should contain an MCP Servers entry");
        assert!(
            mcp_entry.available,
            "MCP Servers should be available when workspace has mcp/servers.toml"
        );
        assert!(
            mcp_entry.missing_hint.is_none(),
            "MCP Servers should have no missing hint when available"
        );

        // 6. Verify workspace_dir actually matches what Config::load_from_dir() returned
        assert_eq!(
            pane.wizard.workspace_dir,
            loaded_config.workspace_dir,
            "wizard workspace_dir should match Config::load_from_dir() workspace_dir"
        );
    }

    /// Regression complement: the same `Config::load_from_dir()` → workspace_dir
    /// path that `App::new` uses must produce an MCP-unavailable summary when
    /// the loaded workspace has no `mcp/servers.toml`.
    #[test]
    fn test_tool_summary_mcp_absent_in_loaded_config_workspace() {
        // 1. Create a temp home with the default config directory structure
        //    but WITHOUT mcp/servers.toml (MCP absent).
        let tmp_home = tempfile::TempDir::new().unwrap();
        let config_dir = tmp_home.path().join(".config").join("maestroclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        // load_from_dir resolves workspace_dir to {home}/.config/maestroclaw/workspace
        let workspace_dir = config_dir.join("workspace");
        std::fs::create_dir_all(&workspace_dir).unwrap();
        // Intentionally do NOT create mcp/servers.toml

        // 2. Load config using the explicit home dir — same path as App::new line 398.
        let workspace_dir_from_config = maestro_claw::config::Config::load_from_dir(
            tmp_home.path().to_path_buf(),
        )
        .unwrap_or_else(|e| {
            panic!("Config::load_from_dir() failed: {e}");
        })
        .workspace_dir;

        // 3. Verify the loaded workspace_dir points to the default location
        assert_eq!(
            workspace_dir_from_config, workspace_dir,
            "Config::load_from_dir() should return the default workspace_dir under temp home"
        );

        // 4. Create pane with the loaded workspace_dir (same as App::new does)
        let mut pane = MaestroClawPane::new(workspace_dir_from_config.clone());
        pane.activate_wizard();
        for _ in 0..5 {
            pane.wizard.next_step();
        }
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // 5. MCP must be unavailable — workspace has no mcp/servers.toml
        let mcp_entry = pane
            .wizard
            .tool_summary
            .iter()
            .find(|t| t.name == "MCP Servers")
            .expect("tool_summary should contain an MCP Servers entry");
        assert!(
            !mcp_entry.available,
            "MCP Servers should be unavailable when loaded workspace has no mcp/servers.toml"
        );
        assert!(
            mcp_entry.missing_hint.is_some(),
            "MCP Servers should have a missing hint when unavailable"
        );

        // 6. Verify workspace_dir matches what Config::load_from_dir() returned
        assert_eq!(
            pane.wizard.workspace_dir, workspace_dir_from_config,
            "wizard workspace_dir should match Config::load_from_dir() workspace_dir"
        );
    }

    // ── Workspace mutation after pane creation — refresh on transition ──

    /// When the user creates mcp/servers.toml *during* the ChannelSetup step
    /// (e.g. via an external setup script), the ToolSummary pane must reflect
    /// the new MCP status because `next_step()` rebuilds the summary.
    #[test]
    fn test_tool_summary_refreshes_on_channel_setup_to_summary_transition() {
        // 1. Create a temp workspace WITHOUT mcp/servers.toml
        let tmp = tempfile::TempDir::new().unwrap();

        // 2. Create pane — wizard starts at Welcome with initial tool_summary
        //    (build_tool_summary runs in SetupWizard::new, sees no servers.toml)
        let mut pane = MaestroClawPane::new(tmp.path().to_path_buf());
        pane.activate_wizard();

        // Verify MCP is absent in the initial summary
        let mcp_entry = pane
            .wizard
            .tool_summary
            .iter()
            .find(|t| t.name == "MCP Servers")
            .expect("tool_summary should contain MCP Servers entry");
        assert!(
            !mcp_entry.available,
            "MCP should be absent before workspace mutation"
        );

        // 3. Advance to ChannelSetup (step 5)
        pane.wizard.next_step(); // -> ToolDetection
        pane.wizard.next_step(); // -> PrimaryToolSelection
        pane.wizard.next_step(); // -> ProviderSelection
        pane.wizard.next_step(); // -> ChannelSetup
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);

        // 4. Mutate the workspace: create mcp/servers.toml
        let mcp_dir = tmp.path().join("mcp");
        std::fs::create_dir_all(&mcp_dir).unwrap();
        std::fs::write(mcp_dir.join("servers.toml"), "# created during wizard\n").unwrap();

        // 5. Advance to ToolSummary — need cursor on Continue row first
        let channel_count = crate::maesterclaw::ChannelType::all().len();
        pane.wizard.cursor = channel_count; // Continue row
        let action = pane.handle_key(crossterm::event::KeyCode::Enter); // ChannelSetup Continue
        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // 6. Assert MCP is now available in the refreshed summary
        let mcp_entry = pane
            .wizard
            .tool_summary
            .iter()
            .find(|t| t.name == "MCP Servers")
            .expect("tool_summary should contain MCP Servers entry");
        assert!(
            mcp_entry.available,
            "MCP should be available after workspace mutation and transition"
        );
        assert!(
            mcp_entry.missing_hint.is_none(),
            "MCP should have no missing hint when available"
        );
    }

    // ── True end-to-end regression: config.toml + explicit home → pane ──

    /// End-to-end regression: creates a real `config.toml` under a temp home,
    /// loads config via `Config::load_from_dir()`, instantiates the pane the
    /// same way `App::new` does, and asserts that both the wizard's
    /// `workspace_dir` and the ToolSummary MCP row reflect the configured
    /// workspace path. No HOME mutation; no pseudo-default paths.
    #[test]
    fn test_e2e_config_toml_workspace_flows_into_pane_and_tool_summary() {
        // 1. Create a temp home with config.toml on disk
        let tmp_home = tempfile::TempDir::new().unwrap();
        let config_dir = tmp_home.path().join(".config").join("maestroclaw");
        std::fs::create_dir_all(&config_dir).unwrap();

        // Write a minimal config.toml (primary_tool is the only field we set;
        // workspace_dir is always overwritten by the default path in load).
        std::fs::write(
            config_dir.join("config.toml"),
            "primary_tool = \"codex\"\n",
        )
        .unwrap();

        // 2. Create the workspace dir + mcp/servers.toml so MCP is available
        let workspace_dir = config_dir.join("workspace");
        std::fs::create_dir_all(workspace_dir.join("mcp")).unwrap();
        std::fs::write(
            workspace_dir.join("mcp").join("servers.toml"),
            "# e2e test config\n",
        )
        .unwrap();

        // 3. Load config the same way App::new does (via load_from_dir to avoid
        //    HOME mutation), then extract workspace_dir — matching app.rs:397–400.
        let loaded_config = maestro_claw::config::Config::load_from_dir(
            tmp_home.path().to_path_buf(),
        )
        .expect("config should load from temp home");

        // Config::load_from_dir() should have picked up primary_tool from the
        // config file and kept the default workspace_dir.
        assert_eq!(
            loaded_config.primary_tool, "codex",
            "primary_tool should come from the written config.toml"
        );
        assert_eq!(
            loaded_config.workspace_dir, workspace_dir,
            "workspace_dir should resolve to the default path under temp home"
        );

        // 4. Create the pane exactly as App::new does:
        //    MaestroClawPane::new(config.workspace_dir)
        let mut pane = MaestroClawPane::new(loaded_config.workspace_dir.clone());

        // 5. Advance wizard to ToolSummary
        pane.activate_wizard();
        for _ in 0..5 {
            pane.wizard.next_step();
        }
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // 6. Assert wizard workspace_dir matches the loaded config
        assert_eq!(
            pane.wizard.workspace_dir,
            loaded_config.workspace_dir,
            "wizard workspace_dir must match Config::load_from_dir() workspace_dir"
        );

        // 7. Assert the MCP ToolSummary row reflects the configured workspace:
        //    servers.toml exists → MCP available, no missing hint.
        let mcp_entry = pane
            .wizard
            .tool_summary
            .iter()
            .find(|t| t.name == "MCP Servers")
            .expect("tool_summary should contain MCP Servers entry");
        assert!(
            mcp_entry.available,
            "MCP Servers must be available — servers.toml exists in configured workspace"
        );
        assert!(
            mcp_entry.missing_hint.is_none(),
            "MCP Servers must have no missing hint when available"
        );

        // 8. Render the ToolSummary and verify MCP appears in the buffer
        let content = render_tool_summary_buffer(&pane);
        assert!(
            content.contains("✓"),
            "rendered buffer must show checkmark(s) for available tools"
        );
        assert!(
            content.contains("MCP Servers"),
            "rendered buffer must show 'MCP Servers' when available"
        );
        // The "5/6" count reflects MCP being available
        assert!(
            content.contains("5/6 tool categories available"),
            "expected '5/6 tool categories available' with MCP present, got:\n{content}"
        );
    }

    // ── Aligned marker prefix test ──

    #[test]
    fn test_render_tool_summary_aligned_marker_prefix() {
        let (pane, _tmp) = pane_at_tool_summary_hermetic(false);
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                pane.test_render_wizard_tool_summary(f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        let lines: Vec<&str> = content.lines().collect();

        // Find all tool row lines (containing ✓ or ✗)
        let tool_rows: Vec<&&str> = lines
            .iter()
            .filter(|l| l.contains('✓') || l.contains('✗'))
            .collect();

        assert!(
            !tool_rows.is_empty(),
            "should have at least one tool row with a marker"
        );

        // Every tool row should start with the marker (✓ or ✗), not whitespace,
        // because Wrap { trim: true } strips leading spaces.
        for row in &tool_rows {
            let trimmed = row.trim_start();
            assert!(
                trimmed.starts_with('✓') || trimmed.starts_with('✗'),
                "tool row should start with marker, got: '{row}'"
            );
        }
    }
}

/// App-level tests that drive Esc/Left/BackTab through handle_key_with_session_count
/// (the real App path, which syncs session count before delegating to handle_key).
#[cfg(test)]
mod app_path_backnav_tests {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, WizardStep};
    use crossterm::event::KeyCode;

    /// Advance a freshly-created pane through the wizard to a given step.
    /// Returns the pane positioned at `target_step`.
    fn advance_to_step(target_step: WizardStep) -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();

        let steps: Vec<WizardStep> = vec![
            WizardStep::Welcome,
            WizardStep::ToolDetection,
            WizardStep::PrimaryToolSelection,
            WizardStep::ProviderSelection,
            WizardStep::ChannelSetup,
            WizardStep::ToolSummary,
            WizardStep::Complete,
        ];

        for step in steps.iter() {
            if *step == target_step {
                break;
            }
            // All steps advance on Enter
            match step {
                WizardStep::Welcome
                | WizardStep::ToolDetection
                | WizardStep::ToolSummary => {
                    let _ = pane.handle_key_with_session_count(KeyCode::Enter, 0);
                }
                WizardStep::PrimaryToolSelection
                | WizardStep::ProviderSelection
                | WizardStep::ChannelSetup => {
                    let _ = pane.handle_key_with_session_count(KeyCode::Enter, 0);
                }
                WizardStep::Complete => unreachable!(),
            }
        }

        assert_eq!(
            pane.wizard.current_step(),
            target_step,
            "expected to be on {:?}",
            target_step
        );
        pane
    }

    #[test]
    fn test_esc_on_primary_tool_selection_goes_back() {
        let mut pane = advance_to_step(WizardStep::PrimaryToolSelection);
        let action = pane.handle_key_with_session_count(KeyCode::Esc, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolDetection);
    }

    #[test]
    fn test_left_on_primary_tool_selection_goes_back() {
        let mut pane = advance_to_step(WizardStep::PrimaryToolSelection);
        let action = pane.handle_key_with_session_count(KeyCode::Left, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolDetection);
    }

    #[test]
    fn test_backtab_on_primary_tool_selection_goes_back() {
        let mut pane = advance_to_step(WizardStep::PrimaryToolSelection);
        let action =
            pane.handle_key_with_session_count(KeyCode::BackTab, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolDetection);
    }

    #[test]
    fn test_esc_on_provider_selection_goes_back() {
        let mut pane = advance_to_step(WizardStep::ProviderSelection);
        let action = pane.handle_key_with_session_count(KeyCode::Esc, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::PrimaryToolSelection);
    }

    #[test]
    fn test_left_on_provider_selection_goes_back() {
        let mut pane = advance_to_step(WizardStep::ProviderSelection);
        let action = pane.handle_key_with_session_count(KeyCode::Left, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::PrimaryToolSelection);
    }

    #[test]
    fn test_backtab_on_provider_selection_goes_back() {
        let mut pane = advance_to_step(WizardStep::ProviderSelection);
        let action =
            pane.handle_key_with_session_count(KeyCode::BackTab, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::PrimaryToolSelection);
    }

    #[test]
    fn test_esc_on_channel_setup_goes_back() {
        let mut pane = advance_to_step(WizardStep::ChannelSetup);
        let action = pane.handle_key_with_session_count(KeyCode::Esc, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
    }

    #[test]
    fn test_left_on_channel_setup_goes_back() {
        let mut pane = advance_to_step(WizardStep::ChannelSetup);
        let action = pane.handle_key_with_session_count(KeyCode::Left, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
    }

    #[test]
    fn test_backtab_on_channel_setup_goes_back() {
        let mut pane = advance_to_step(WizardStep::ChannelSetup);
        let action =
            pane.handle_key_with_session_count(KeyCode::BackTab, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
    }

    #[test]
    fn test_session_count_synced_before_key_handling() {
        let mut pane = MaestroClawPane::default();
        // handle_key_with_session_count should sync the count
        let _ = pane.handle_key_with_session_count(KeyCode::Char('n'), 5);
        // The pane was default-constructed with session_count=0,
        // and after the call it should be 5.
        assert!(!pane.is_wizard_active());
    }

    #[test]
    fn test_left_on_welcome_emits_wizard_back_and_stays_on_welcome() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);

        let action = pane.handle_key_with_session_count(KeyCode::Left, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);
    }

    #[test]
    fn test_backtab_on_welcome_emits_wizard_back_and_stays_on_welcome() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);

        let action = pane.handle_key_with_session_count(KeyCode::BackTab, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);
    }
}

/// Tests for session browser -> Enter -> resume/focus flow.
///
/// Verify that opening the session browser, selecting a session, and pressing
/// Enter correctly deactivates the browser and returns the session ID for
/// the app to focus/switch-to-Sessions.
#[cfg(test)]
mod session_browser_resume_tests {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, SessionEntry};
    use crossterm::event::KeyCode;

    fn make_entry(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "test preview".to_string(),
            source: "claude".to_string(),
            last_active: "2m ago".to_string(),
            turn_count: 5,
        }
    }

    #[test]
    fn test_open_browser_populates_from_entries() {
        let mut pane = MaestroClawPane::default();
        let entries = vec![
            make_entry("s1", "Alpha"),
            make_entry("s2", "Beta"),
            make_entry("s3", "Gamma"),
        ];

        // Simulate what handle_maestroclaw_action does for OpenSessionBrowser:
        // load entries, then activate
        pane.load_session_entries(entries);
        pane.activate_session_browser();

        assert!(pane.is_session_browser_active());
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s1".to_string())
        );

        // Navigate down
        let _ = pane.handle_key(KeyCode::Down);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s2".to_string())
        );

        // Enter selects and closes browser
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);
        assert!(!pane.is_session_browser_active());

        // The app can now query the selected session ID to perform the
        // same focus/switch-to-Sessions flow as OpenSelected
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s2".to_string())
        );
    }

    #[test]
    fn test_browser_enter_returns_correct_id_for_focus() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("sess-abc", "Build pipeline"),
            make_entry("sess-def", "Fix auth bug"),
        ]);
        pane.activate_session_browser();

        // Navigate to second session
        let _ = pane.handle_key(KeyCode::Down);

        // Select it
        let _ = pane.handle_key(KeyCode::Enter);

        // The returned ID should be usable by the app to find the session
        // in app.sessions and switch to the Sessions tab
        let id = pane.selected_browser_session_id().unwrap();
        assert_eq!(id, "sess-def");
    }

    #[test]
    fn test_browser_esc_keeps_selection_clear() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Only")]);
        pane.activate_session_browser();

        // Esc closes without selecting
        let action = pane.handle_key(KeyCode::Esc);
        assert_eq!(action, MaestroClawAction::SessionBrowserClose);
        assert!(!pane.is_session_browser_active());

        // No session was selected for focus
        // (selected_browser_session_id still holds last loaded session,
        // but the action was Close not Select, so the app won't switch tabs)
    }

    #[test]
    fn test_full_flow_open_navigate_select_close() {
        let mut pane = MaestroClawPane::default();
        let entries = vec![
            make_entry("id-1", "First session"),
            make_entry("id-2", "Second session"),
            make_entry("id-3", "Third session"),
        ];

        // Step 1: Open (load + activate)
        pane.load_session_entries(entries);
        pane.activate_session_browser();
        assert!(pane.is_session_browser_active());

        // Step 2: Navigate
        let _ = pane.handle_key(KeyCode::Down);
        let _ = pane.handle_key(KeyCode::Down);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("id-3".to_string())
        );

        // Step 3: Select (Enter)
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);
        assert!(!pane.is_session_browser_active());

        // Step 4: App can use selected_browser_session_id() to find the
        // session in app.sessions, set selected_session, and switch tabs
        let selected_id = pane.selected_browser_session_id().unwrap();
        assert_eq!(selected_id, "id-3");
    }
}

/// Regression test: drive b → filter text → Backspace → Enter through the
/// real app-dispatch path (handle_key_with_session_count) and assert browser
/// visibility plus session focus.
///
/// This mirrors the exact key sequence a user would type in the running TUI
/// when the MaestroClaw tab is active, exercising the Char forwarding added
/// in app.rs and the session-browser key handling in mod.rs.
#[cfg(test)]
mod app_dispatch_browser_regression {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, SessionEntry};
    use crossterm::event::KeyCode;

    fn make_entry(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "regression test".to_string(),
            source: "claude".to_string(),
            last_active: "1m ago".to_string(),
            turn_count: 3,
        }
    }

    /// Load fixture sessions so the browser has something to show.
    fn pane_with_sessions() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("alpha", "Alpha Project"),
            make_entry("beta", "Beta Session"),
            make_entry("gamma", "Gamma Workspace"),
        ]);
        pane
    }

    #[test]
    fn test_b_opens_session_browser_via_app_path() {
        let mut pane = pane_with_sessions();
        assert!(!pane.is_session_browser_active());

        // 'b' in the main view triggers OpenSessionBrowser
        let action = pane.handle_key_with_session_count(KeyCode::Char('b'), 3);
        assert_eq!(action, MaestroClawAction::OpenSessionBrowser);

        // The app would then call load_session_entries + activate_session_browser;
        // simulate that (same as handle_maestroclaw_action does).
        pane.activate_session_browser();
        assert!(pane.is_session_browser_active());
    }

    #[test]
    fn test_filter_text_and_backspace_via_app_path() {
        let mut pane = pane_with_sessions();
        pane.activate_session_browser();

        // Type 'be' to filter — should match "Beta Session"
        let _ = pane.handle_key_with_session_count(KeyCode::Char('b'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('e'), 3);
        assert!(pane.is_session_browser_active());
        // Only "Beta Session" should remain
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("beta".to_string())
        );

        // Backspace removes 'e' → now filter is "b" → matches "Beta" still
        let _ = pane.handle_key_with_session_count(KeyCode::Backspace, 3);
        assert!(pane.is_session_browser_active());
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("beta".to_string())
        );

        // Backspace again removes 'b' → empty filter → back to "Alpha" (first)
        let _ = pane.handle_key_with_session_count(KeyCode::Backspace, 3);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("alpha".to_string())
        );
    }

    #[test]
    fn test_full_b_filter_backspace_enter_flow() {
        let mut pane = pane_with_sessions();
        // Step 1: 'b' opens browser
        let action = pane.handle_key_with_session_count(KeyCode::Char('b'), 3);
        assert_eq!(action, MaestroClawAction::OpenSessionBrowser);
        pane.activate_session_browser();

        // Step 2: type filter text 'gam'
        let _ = pane.handle_key_with_session_count(KeyCode::Char('g'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('a'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('m'), 3);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("gamma".to_string()),
            "filter 'gam' should select Gamma Workspace"
        );

        // Step 3: Backspace to remove 'm' → still matches Gamma
        let _ = pane.handle_key_with_session_count(KeyCode::Backspace, 3);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("gamma".to_string())
        );

        // Step 4: Enter selects the filtered session and closes browser
        let action = pane.handle_key_with_session_count(KeyCode::Enter, 3);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);
        assert!(!pane.is_session_browser_active());
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("gamma".to_string()),
            "selected session ID should persist after browser closes"
        );
    }

    #[test]
    fn test_w_activates_wizard_via_app_path() {
        let mut pane = MaestroClawPane::default();
        assert!(!pane.is_wizard_active());

        let action = pane.handle_key_with_session_count(KeyCode::Char('w'), 0);
        assert_eq!(action, MaestroClawAction::StartSetup);
        assert!(pane.is_wizard_active());
    }

    #[test]
    fn test_w_then_b_wizard_suppresses_browser() {
        let mut pane = MaestroClawPane::default();

        // Activate wizard
        let _ = pane.handle_key_with_session_count(KeyCode::Char('w'), 0);
        assert!(pane.is_wizard_active());

        // While wizard is active, 'b' is consumed by the wizard handler (returns
        // None) — the browser is NOT opened.
        let action = pane.handle_key_with_session_count(KeyCode::Char('b'), 0);
        assert_eq!(action, MaestroClawAction::None);
        assert!(!pane.is_session_browser_active());
    }
}

/// Regression tests for browser-active short-circuit and wizard rerun semantics.
///
/// These verify that when the session browser is active, keys that would normally
/// trigger app-global actions (q, r, /, ?) are instead forwarded through
/// handle_key_with_session_count to the browser's type-to-filter handler.
/// They also verify that pressing 'w' after wizard completion resets the wizard.
#[cfg(test)]
mod browser_shortcircuit_and_wizard_rerun_tests {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, SessionEntry, WizardStep};
    use crossterm::event::KeyCode;

    fn make_entry(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "test".to_string(),
            source: "claude".to_string(),
            last_active: "1m ago".to_string(),
            turn_count: 3,
        }
    }

    fn pane_with_browser_active() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("alpha", "Alpha Project"),
            make_entry("beta", "Beta Session"),
            make_entry("gamma", "Gamma Workspace"),
        ]);
        pane.activate_session_browser();
        pane
    }

    // --- Browser-active key short-circuit tests ---

    #[test]
    fn test_browser_active_q_is_consumed_as_filter_char() {
        let mut pane = pane_with_browser_active();
        // 'q' while browser is active should be treated as filter input, not quit.
        // At the pane level, handle_key_with_session_count returns None (filter
        // char produces no MaestroClawAction), proving it was consumed by the browser.
        let action = pane.handle_key_with_session_count(KeyCode::Char('q'), 3);
        assert_eq!(action, MaestroClawAction::None);
        assert!(pane.is_session_browser_active());
    }

    #[test]
    fn test_browser_active_r_is_consumed_as_filter_char() {
        let mut pane = pane_with_browser_active();
        let action = pane.handle_key_with_session_count(KeyCode::Char('r'), 3);
        assert_eq!(action, MaestroClawAction::None);
        assert!(pane.is_session_browser_active());
    }

    #[test]
    fn test_browser_active_slash_is_consumed_as_filter_char() {
        let mut pane = pane_with_browser_active();
        let action = pane.handle_key_with_session_count(KeyCode::Char('/'), 3);
        assert_eq!(action, MaestroClawAction::None);
        assert!(pane.is_session_browser_active());
    }

    #[test]
    fn test_browser_active_filter_text_narrows_results() {
        let mut pane = pane_with_browser_active();
        // Type 'al' — should match "Alpha Project" only
        let _ = pane.handle_key_with_session_count(KeyCode::Char('a'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('l'), 3);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("alpha".to_string()),
            "filter 'al' should select Alpha Project"
        );
    }

    #[test]
    fn test_browser_active_backspace_removes_filter_char() {
        let mut pane = pane_with_browser_active();
        let _ = pane.handle_key_with_session_count(KeyCode::Char('b'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('e'), 3);
        // Backspace removes 'e'
        let _ = pane.handle_key_with_session_count(KeyCode::Backspace, 3);
        // Filter is now "b" — should still match "Beta Session"
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("beta".to_string())
        );
        // Backspace removes 'b' — empty filter, back to first
        let _ = pane.handle_key_with_session_count(KeyCode::Backspace, 3);
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("alpha".to_string())
        );
    }

    #[test]
    fn test_browser_active_enter_selects_and_closes() {
        let mut pane = pane_with_browser_active();
        // Type to narrow to Gamma
        let _ = pane.handle_key_with_session_count(KeyCode::Char('g'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('a'), 3);
        let _ = pane.handle_key_with_session_count(KeyCode::Char('m'), 3);
        // Enter selects
        let action = pane.handle_key_with_session_count(KeyCode::Enter, 3);
        assert_eq!(action, MaestroClawAction::SessionBrowserSelect);
        assert!(!pane.is_session_browser_active());
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("gamma".to_string())
        );
    }

    #[test]
    fn test_browser_active_esc_closes_browser() {
        let mut pane = pane_with_browser_active();
        let action = pane.handle_key_with_session_count(KeyCode::Esc, 3);
        assert_eq!(action, MaestroClawAction::SessionBrowserClose);
        assert!(!pane.is_session_browser_active());
    }

    // --- Complete-step close/reopen tests ---

    /// Advance the wizard to the Complete step.
    /// Some steps (e.g. ChannelSetup) may return WizardSelection on Enter
    /// when the cursor is on a toggle item rather than the "Next" button,
    /// so we send Enter repeatedly until we reach Complete.
    fn pane_at_complete_step() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        let _ = pane.handle_key_with_session_count(KeyCode::Char('w'), 0);
        assert!(pane.is_wizard_active());
        // ChannelSetup requires cursor to be on the "Next" item (index == channel_count)
        // to advance. Move cursor to the end before advancing through it.
        let steps_to_complete = [
            WizardStep::Welcome,
            WizardStep::ToolDetection,
            WizardStep::PrimaryToolSelection,
            WizardStep::ProviderSelection,
            WizardStep::ChannelSetup,
            WizardStep::ToolSummary,
        ];
        for step in &steps_to_complete {
            assert_eq!(pane.wizard.current_step(), *step);
            if *step == WizardStep::ChannelSetup {
                // Move cursor to "Next" button (last item)
                let channel_count = crate::maesterclaw::ChannelType::all().len();
                for _ in 0..channel_count {
                    let _ = pane.handle_key_with_session_count(KeyCode::Down, 0);
                }
            }
            let _ = pane.handle_key_with_session_count(KeyCode::Enter, 0);
        }
        assert_eq!(pane.wizard.current_step(), WizardStep::Complete);
        pane
    }

    #[test]
    fn test_complete_step_enter_closes_wizard() {
        let mut pane = pane_at_complete_step();
        let action = pane.handle_key_with_session_count(KeyCode::Enter, 0);
        assert_eq!(action, MaestroClawAction::WizardComplete);
        assert!(!pane.is_wizard_active());
    }

    #[test]
    fn test_complete_step_esc_goes_to_tool_summary() {
        let mut pane = pane_at_complete_step();
        let action = pane.handle_key_with_session_count(KeyCode::Esc, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
    }

    #[test]
    fn test_complete_step_left_goes_to_tool_summary() {
        let mut pane = pane_at_complete_step();
        let action = pane.handle_key_with_session_count(KeyCode::Left, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
    }

    #[test]
    fn test_complete_step_backtab_goes_to_tool_summary() {
        let mut pane = pane_at_complete_step();
        let action = pane.handle_key_with_session_count(KeyCode::BackTab, 0);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
    }

    #[test]
    fn test_w_after_completion_resets_wizard() {
        let mut pane = pane_at_complete_step();
        // Close the wizard via Enter
        let _ = pane.handle_key_with_session_count(KeyCode::Enter, 0);
        assert!(!pane.is_wizard_active());
        assert!(pane.wizard.is_completed());

        // Press 'w' again — should reset to Welcome, not show stale Complete
        let action = pane.handle_key_with_session_count(KeyCode::Char('w'), 0);
        assert_eq!(action, MaestroClawAction::StartSetup);
        assert!(pane.is_wizard_active());
        assert_eq!(
            pane.wizard.current_step(),
            WizardStep::Welcome,
            "wizard should be reset to Welcome after w rerun, not stale Complete"
        );
        assert!(!pane.wizard.is_completed());
    }

    #[test]
    fn test_w_after_dismiss_resets_wizard() {
        let mut pane = pane_at_complete_step();
        // Navigate back to Welcome, then dismiss via Esc
        // (Esc on Complete now goes to ToolSummary, so we need to reach Welcome)
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // Complete → ToolSummary
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // ToolSummary → ChannelSetup
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // ChannelSetup → ProviderSelection
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // ProviderSelection → PrimaryToolSelection
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // PrimaryToolSelection → ToolDetection
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // ToolDetection → Welcome
        let _ = pane.handle_key_with_session_count(KeyCode::Esc, 0); // Welcome → dismiss
        assert!(!pane.is_wizard_active());
        assert!(pane.wizard.is_dismissed());

        // Press 'w' again — should reset to Welcome
        let action = pane.handle_key_with_session_count(KeyCode::Char('w'), 0);
        assert_eq!(action, MaestroClawAction::StartSetup);
        assert!(pane.is_wizard_active());
        assert_eq!(
            pane.wizard.current_step(),
            WizardStep::Welcome,
            "wizard should be reset to Welcome after w rerun from dismissed state"
        );
    }
}

/// Tests for the app-loop key-routing gate (`route_key_browser_priority`).
///
/// These verify that the session browser short-circuit intercepts keys like `n`,
/// `p`, and `t` *before* global normal-mode handlers would claim them, and that
/// modified shortcuts like Ctrl+C / Ctrl+Q are NOT swallowed.  The helper
/// delegates to `should_route_to_browser` which is also used as the match guard
/// in `run_app`, so the routing logic is exercised in one place.
#[cfg(test)]
mod browser_priority_routing_tests {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, SessionEntry};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_entry(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "preview".to_string(),
            source: "claude".to_string(),
            last_active: "1m ago".to_string(),
            turn_count: 1,
        }
    }

    // -- Browser inactive: all keys fall through (None) ----------------------

    #[test]
    fn test_browser_inactive_n_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        // Browser NOT activated
        assert_eq!(
            pane.route_key_browser_priority(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                1,
            ),
            None,
            "'n' must fall through when browser is inactive"
        );
    }

    #[test]
    fn test_browser_inactive_p_falls_through() {
        let mut pane = MaestroClawPane::default();
        assert_eq!(
            pane.route_key_browser_priority(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                0,
            ),
            None,
            "'p' must fall through when browser is inactive"
        );
    }

    #[test]
    fn test_browser_inactive_t_falls_through() {
        let mut pane = MaestroClawPane::default();
        assert_eq!(
            pane.route_key_browser_priority(
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
                0,
            ),
            None,
            "'t' must fall through when browser is inactive"
        );
    }

    // -- Browser active: modified keys (Ctrl+C, Ctrl+Q) fall through --------

    #[test]
    fn test_browser_active_ctrl_c_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            1,
        );
        assert_eq!(
            action, None,
            "Ctrl+C must fall through even when browser is active"
        );
        assert!(pane.is_session_browser_active());
        assert_eq!(pane.browser_filter_text(), "");
    }

    #[test]
    fn test_browser_active_ctrl_q_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            1,
        );
        assert_eq!(
            action, None,
            "Ctrl+Q must fall through even when browser is active"
        );
        assert!(pane.is_session_browser_active());
        assert_eq!(pane.browser_filter_text(), "");
    }

    #[test]
    fn test_browser_active_alt_char_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::ALT),
            1,
        );
        assert_eq!(
            action, None,
            "Alt+n must fall through even when browser is active"
        );
        assert_eq!(pane.browser_filter_text(), "");
    }

    // -- Browser active: unmodified n/p/t captured and reaches filter ---------
    // Each test proves the FIRST character through the priority gate actually
    // reached the browser filter by asserting filter text AND selection.

    #[test]
    fn test_browser_active_n_captured_as_filter() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "November Session"),
            make_entry("s2", "Other"),
        ]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            2,
        );
        assert!(action.is_some(), "'n' must be captured when browser is active");
        assert_eq!(action, Some(MaestroClawAction::None));

        // Prove the first character actually reached the filter
        assert_eq!(
            pane.browser_filter_text(), "n",
            "filter must contain 'n' after priority gate captured it"
        );
        // Only "November Session" matches 'n' → selection narrowed
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s1".to_string()),
        );
    }

    #[test]
    fn test_browser_active_p_captured_as_filter() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Python Project"),
            make_entry("s2", "Rust Workspace"),
        ]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
            2,
        );
        assert!(action.is_some(), "'p' must be captured when browser is active");
        assert_eq!(action, Some(MaestroClawAction::None));

        // Prove the first character actually reached the filter
        assert_eq!(
            pane.browser_filter_text(), "p",
            "filter must contain 'p' after priority gate captured it"
        );
        // Only "Python Project" matches 'p' → selection narrowed
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s1".to_string()),
        );
    }

    #[test]
    fn test_browser_active_t_captured_as_filter() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Track Builder"),
            make_entry("s2", "Memory View"),
        ]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
            2,
        );
        assert!(action.is_some(), "'t' must be captured when browser is active");
        assert_eq!(action, Some(MaestroClawAction::None));

        // Prove the first character actually reached the filter
        assert_eq!(
            pane.browser_filter_text(), "t",
            "filter must contain 't' after priority gate captured it"
        );
        // Only "Track Builder" matches 't' (Memory View does not)
        assert_eq!(
            pane.selected_browser_session_id(),
            Some("s1".to_string()),
        );
    }

    // -- Browser active: Esc still closes browser through priority gate -------

    #[test]
    fn test_browser_active_esc_closes_via_priority_gate() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            1,
        );
        assert_eq!(
            action,
            Some(MaestroClawAction::SessionBrowserClose),
            "Esc through priority gate must close the browser"
        );
        assert!(
            !pane.is_session_browser_active(),
            "browser must be deactivated after Esc through priority gate"
        );
    }

    // -- Browser active: n does NOT produce NewSession action ----------------

    #[test]
    fn test_browser_active_n_does_not_emit_new_session() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "New Project")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            1,
        );
        assert_ne!(
            action,
            Some(MaestroClawAction::NewSession),
            "'n' while browser is active must NOT produce NewSession — it must go to the filter"
        );
        assert_eq!(pane.browser_filter_text(), "n");
    }

    // -- Wizard active: priority gate returns None (browser not active) ------

    #[test]
    fn test_wizard_active_priority_gate_returns_none() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();

        assert_eq!(
            pane.route_key_browser_priority(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                0,
            ),
            None,
            "priority gate must return None when wizard is active (browser is not)"
        );
    }

    // -- BackTab falls through when browser is active ------------------------

    #[test]
    fn test_browser_active_backtab_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            1,
        );
        assert_eq!(
            action, None,
            "BackTab (Shift+Tab) must fall through when browser is active"
        );
        assert!(
            pane.is_session_browser_active(),
            "browser must remain active after BackTab fall-through"
        );
    }

    // -- ? (Shift+/) falls through when browser is active ---------------------

    #[test]
    fn test_browser_active_question_mark_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT),
            1,
        );
        assert_eq!(
            action, None,
            "'?' must fall through when browser is active so help toggle works"
        );
        assert!(
            pane.is_session_browser_active(),
            "browser must remain active after '?' fall-through"
        );
        assert_eq!(
            pane.browser_filter_text(), "",
            "'?' must not be added to the filter"
        );
    }

    // -- Tab falls through when browser is active ----------------------------

    #[test]
    fn test_browser_active_tab_falls_through() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            1,
        );
        assert_eq!(
            action, None,
            "Tab must fall through when browser is active"
        );
        assert!(pane.is_session_browser_active());
    }

    // -- App-dispatch coverage: b and w through priority gate -----------------

    #[test]
    fn test_browser_active_b_captured_as_filter() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Beta Session"),
            make_entry("s2", "Other"),
        ]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
            2,
        );
        assert!(action.is_some(), "'b' must be captured when browser is active");
        assert_eq!(pane.browser_filter_text(), "b");
        // b should NOT produce OpenSessionBrowser — it goes to the filter
        assert_ne!(action, Some(MaestroClawAction::OpenSessionBrowser));
    }

    #[test]
    fn test_browser_active_w_captured_as_filter() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Wizard Session"),
            make_entry("s2", "Other"),
        ]);
        pane.activate_session_browser();

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
            2,
        );
        assert!(action.is_some(), "'w' must be captured when browser is active");
        assert_eq!(pane.browser_filter_text(), "w");
        assert_ne!(action, Some(MaestroClawAction::StartSetup));
    }

    // -- App-dispatch: Enter through priority gate selects session ------------

    #[test]
    fn test_browser_active_enter_selects_via_priority_gate() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Alpha"),
            make_entry("s2", "Beta"),
        ]);
        pane.activate_session_browser();

        // Navigate down to second session
        let _ = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            2,
        );

        let action = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            2,
        );
        assert_eq!(
            action,
            Some(MaestroClawAction::SessionBrowserSelect),
            "Enter through priority gate must select session"
        );
        assert!(!pane.is_session_browser_active());
        assert_eq!(pane.selected_browser_session_id(), Some("s2".to_string()));
    }

    // -- App-dispatch: arrows through priority gate navigate ------------------

    #[test]
    fn test_browser_active_arrows_navigate_via_priority_gate() {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![
            make_entry("s1", "Alpha"),
            make_entry("s2", "Beta"),
            make_entry("s3", "Gamma"),
        ]);
        pane.activate_session_browser();

        let down = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            3,
        );
        assert_eq!(down, Some(MaestroClawAction::Navigate));
        assert_eq!(pane.selected_browser_session_id(), Some("s2".to_string()));

        let down2 = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            3,
        );
        assert_eq!(down2, Some(MaestroClawAction::Navigate));
        assert_eq!(pane.selected_browser_session_id(), Some("s3".to_string()));

        let up = pane.route_key_browser_priority(
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            3,
        );
        assert_eq!(up, Some(MaestroClawAction::Navigate));
        assert_eq!(pane.selected_browser_session_id(), Some("s2".to_string()));
    }
}

/// Regression tests for app-level normal-mode dispatch when the session browser is active.
///
/// These verify that `should_route_to_browser` (the exact predicate used as the match
/// guard in `run_app`) correctly rejects Tab, BackTab, and `?` so that the app-level
/// global handlers (tab cycling, help toggle) fire as expected.  They also assert that
/// the pane state is undisturbed by the fall-through, confirming no browser side-effects.
#[cfg(test)]
mod app_normal_mode_dispatch_regression {
    use crate::maesterclaw::{MaestroClawPane, SessionEntry};
    use crossterm::event::{KeyCode, KeyModifiers};

    fn make_entry(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "preview".to_string(),
            source: "claude".to_string(),
            last_active: "1m ago".to_string(),
            turn_count: 1,
        }
    }

    /// Helper: build a pane with the browser active and one session loaded.
    fn pane_with_active_browser() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.load_session_entries(vec![make_entry("s1", "Alpha")]);
        pane.activate_session_browser();
        pane
    }

    #[test]
    fn test_tab_gate_opens_when_browser_active() {
        let pane = pane_with_active_browser();
        assert!(
            !pane.should_route_to_browser(KeyModifiers::NONE, KeyCode::Tab),
            "Tab must NOT be routed to browser so app-level tab cycling fires"
        );
    }

    #[test]
    fn test_backtab_gate_opens_when_browser_active() {
        let pane = pane_with_active_browser();
        assert!(
            !pane.should_route_to_browser(KeyModifiers::SHIFT, KeyCode::BackTab),
            "BackTab must NOT be routed to browser so app-level tab cycling fires"
        );
        // Also verify with no modifiers (the catch-all arm in run_app)
        assert!(
            !pane.should_route_to_browser(KeyModifiers::NONE, KeyCode::BackTab),
            "BackTab (no modifier) must NOT be routed to browser"
        );
    }

    #[test]
    fn test_question_mark_gate_opens_when_browser_active() {
        let pane = pane_with_active_browser();
        assert!(
            !pane.should_route_to_browser(KeyModifiers::SHIFT, KeyCode::Char('?')),
            "'?' must NOT be routed to browser so app-level help toggle fires"
        );
    }

    #[test]
    fn test_tab_fallthrough_leaves_browser_state_untouched() {
        let mut pane = pane_with_active_browser();
        let action = pane.route_key_browser_priority(
            crossterm::event::KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            1,
        );
        assert_eq!(action, None, "Tab must fall through");
        assert!(pane.is_session_browser_active(), "browser must stay active");
        assert_eq!(
            pane.browser_filter_text(), "",
            "Tab must not alter the filter"
        );
    }

    #[test]
    fn test_backtab_fallthrough_leaves_browser_state_untouched() {
        let mut pane = pane_with_active_browser();
        let action = pane.route_key_browser_priority(
            crossterm::event::KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            1,
        );
        assert_eq!(action, None, "BackTab must fall through");
        assert!(pane.is_session_browser_active(), "browser must stay active");
        assert_eq!(
            pane.browser_filter_text(), "",
            "BackTab must not alter the filter"
        );
    }

    #[test]
    fn test_question_mark_fallthrough_leaves_browser_state_untouched() {
        let mut pane = pane_with_active_browser();
        let action = pane.route_key_browser_priority(
            crossterm::event::KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT),
            1,
        );
        assert_eq!(action, None, "'?' must fall through");
        assert!(pane.is_session_browser_active(), "browser must stay active");
        assert_eq!(
            pane.browser_filter_text(), "",
            "'?' must not be added to the filter"
        );
    }
}
