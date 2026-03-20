//! Provider selection widget for MaestroClaw setup wizard

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::wizard::{SetupWizard, WizardStep};
#[cfg(test)]
use super::wizard::ProviderChoice;

/// Render the provider selection step
pub fn render_provider_selection(wizard: &SetupWizard, frame: &mut Frame, area: Rect) {
    // Create layout with 3 sections: header, provider list, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Header
                Constraint::Min(5),    // Provider list (minimum 5 rows)
                Constraint::Length(2), // Footer
            ]
            .as_ref(),
        )
        .split(area);

    // Render header
    render_header(frame, chunks[0], wizard);

    // Render provider list
    render_provider_list(frame, chunks[1], wizard);

    // Render footer
    render_footer(frame, chunks[2], wizard);
}

/// Render the header section
fn render_header(frame: &mut Frame, area: Rect, wizard: &SetupWizard) {
    let step_text = format!(
        "[Step {}/{}] Select Inference Provider",
        wizard.current_step().number(),
        WizardStep::TOTAL_STEPS,
    );

    let header_lines = vec![
        Line::from(vec![
            Span::styled(
                step_text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Choose how MaestroClaw connects to LLM models.",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
    ];

    let header = Paragraph::new(header_lines).wrap(Wrap { trim: false });

    frame.render_widget(header, area);
}

/// Render the provider list section
fn render_provider_list(frame: &mut Frame, area: Rect, wizard: &SetupWizard) {
    let items: Vec<ListItem> = wizard
        .provider_list
        .iter()
        .enumerate()
        .map(|(idx, provider)| {
            let is_selected = wizard.cursor == idx;

            // Arrow indicator
            let arrow = if is_selected { " → " } else { "   " };

            // Status circle
            let (status_circle, status_color) = if provider.is_configured {
                ("●", Color::Green)
            } else {
                ("○", Color::DarkGray)
            };

            // Label style
            let label_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(Line::from(vec![
                Span::raw(arrow),
                Span::styled(status_circle, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::raw(provider.icon),
                Span::raw(" "),
                Span::styled(&provider.label, label_style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Providers "),
    );

    frame.render_widget(list, area);
}

/// Render the footer section
fn render_footer(frame: &mut Frame, area: Rect, _wizard: &SetupWizard) {
    let footer_text = vec![Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled(" select  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(" back", Style::default().fg(Color::DarkGray)),
    ])];

    let footer = Paragraph::new(footer_text).wrap(Wrap { trim: false });

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a mock SetupWizard for testing
    fn mock_wizard() -> SetupWizard {
        let mut wizard = SetupWizard::default();
        wizard.provider_list = vec![
            ProviderChoice {
                id: "openai".to_string(),
                label: "OpenAI".to_string(),
                is_configured: true,
                icon: "🤖",
            },
            ProviderChoice {
                id: "anthropic".to_string(),
                label: "Anthropic".to_string(),
                is_configured: false,
                icon: "🧠",
            },
        ];
        wizard
    }

    #[test]
    fn test_render_does_not_panic() {
        let wizard = mock_wizard();

        // Create a mock terminal backend
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        // Render should not panic
        terminal
            .draw(|f| {
                let area = f.area();
                render_provider_selection(&wizard, f, area);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_different_cursor_positions() {
        let mut wizard = mock_wizard();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        // Test with cursor at position 0
        wizard.cursor = 0;
        terminal
            .draw(|f| {
                render_provider_selection(&wizard, f, f.area());
            })
            .unwrap();

        // Test with cursor at position 1
        wizard.cursor = 1;
        terminal
            .draw(|f| {
                render_provider_selection(&wizard, f, f.area());
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_empty_provider_list() {
        let mut wizard = mock_wizard();
        wizard.provider_list.clear();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        // Should not panic with empty list
        terminal
            .draw(|f| {
                render_provider_selection(&wizard, f, f.area());
            })
            .unwrap();
    }

    /// Helper: extract all visible text from a TestBackend buffer as a single string
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
    fn test_render_long_labels_narrow_terminal() {
        let mut wizard = SetupWizard::default();
        // Use the real long provider labels from build_provider_list
        wizard.provider_list = vec![
            ProviderChoice {
                id: "openai".to_string(),
                label: "OpenAI (GPT models via OPENAI_API_KEY)".to_string(),
                is_configured: true,
                icon: "🤖",
            },
            ProviderChoice {
                id: "anthropic".to_string(),
                label: "Anthropic (Claude models via ANTHROPIC_API_KEY)".to_string(),
                is_configured: false,
                icon: "🧠",
            },
            ProviderChoice {
                id: "openrouter".to_string(),
                label: "OpenRouter (100+ models, pay-per-use)".to_string(),
                is_configured: false,
                icon: "🌐",
            },
            ProviderChoice {
                id: "ollama".to_string(),
                label: "Ollama (local models, no API key needed)".to_string(),
                is_configured: false,
                icon: "🏠",
            },
            ProviderChoice {
                id: "custom".to_string(),
                label: "Custom OpenAI-compatible endpoint".to_string(),
                is_configured: false,
                icon: "⚙️",
            },
        ];
        wizard.cursor = 0;

        let backend = ratatui::backend::TestBackend::new(40, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                render_provider_selection(&wizard, f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        // Key UI elements must be present even at 40-col width
        assert!(content.contains("OpenAI"), "Buffer should contain OpenAI provider label");
        assert!(content.contains("Anthropic"), "Buffer should contain Anthropic provider label");
        assert!(content.contains("Providers"), "Buffer should contain list title");
        assert!(content.contains("→"), "Buffer should contain selection arrow");
    }

    #[test]
    fn test_render_long_labels_very_narrow_terminal() {
        let mut wizard = SetupWizard::default();
        wizard.provider_list = vec![
            ProviderChoice {
                id: "openai".to_string(),
                label: "OpenAI (GPT models via OPENAI_API_KEY)".to_string(),
                is_configured: true,
                icon: "🤖",
            },
            ProviderChoice {
                id: "anthropic".to_string(),
                label: "Anthropic (Claude models via ANTHROPIC_API_KEY)".to_string(),
                is_configured: false,
                icon: "🧠",
            },
        ];
        wizard.cursor = 1;

        let backend = ratatui::backend::TestBackend::new(30, 16);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        // Must not panic even at 30 cols with long labels
        terminal
            .draw(|f| {
                render_provider_selection(&wizard, f, f.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend());
        assert!(content.contains("Anthropic"), "Second provider should be visible at 30 cols");
    }
}

/// Tests for ProviderSelection keyboard interaction through MaestroClawPane
#[cfg(test)]
mod key_handling_tests {
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, WizardStep};
    use crossterm::event::KeyCode;

    /// Create a pane with wizard active on the ProviderSelection step
    fn pane_at_provider_selection() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        // Advance: Welcome -> ToolDetection -> PrimaryToolSelection -> ProviderSelection
        pane.wizard.next_step(); // -> ToolDetection
        pane.wizard.next_step(); // -> PrimaryToolSelection
        pane.wizard.next_step(); // -> ProviderSelection
        pane.wizard.cursor = 0;
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);
        pane
    }

    #[test]
    fn test_provider_selection_down_increments_cursor() {
        let mut pane = pane_at_provider_selection();
        let initial = pane.wizard.cursor;

        let action = pane.handle_key(KeyCode::Down);

        assert_eq!(action, MaestroClawAction::WizardSelection);
        assert_eq!(pane.wizard.cursor, initial + 1);
    }

    #[test]
    fn test_provider_selection_up_decrements_cursor() {
        let mut pane = pane_at_provider_selection();
        pane.wizard.cursor = 1; // Move to second item first

        let action = pane.handle_key(KeyCode::Up);

        assert_eq!(action, MaestroClawAction::WizardSelection);
        assert_eq!(pane.wizard.cursor, 0);
    }

    #[test]
    fn test_provider_selection_up_clamps_at_zero() {
        let mut pane = pane_at_provider_selection();
        pane.wizard.cursor = 0;

        let action = pane.handle_key(KeyCode::Up);

        assert_eq!(action, MaestroClawAction::WizardSelection);
        assert_eq!(pane.wizard.cursor, 0, "Cursor should not go below 0");
    }

    #[test]
    fn test_provider_selection_down_clamps_at_last() {
        let mut pane = pane_at_provider_selection();
        let max = pane.wizard.provider_list.len().saturating_sub(1);
        pane.wizard.cursor = max;

        let action = pane.handle_key(KeyCode::Down);

        assert_eq!(action, MaestroClawAction::WizardSelection);
        assert_eq!(
            pane.wizard.cursor, max,
            "Cursor should not exceed last provider index"
        );
    }

    #[test]
    fn test_provider_selection_enter_on_configured_advances() {
        let mut pane = pane_at_provider_selection();
        // First provider (openai) is configured by env var or default mock
        pane.wizard.provider_list[0].is_configured = true;
        pane.wizard.cursor = 0;

        let action = pane.handle_key(KeyCode::Enter);

        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(
            pane.wizard.selected_provider,
            Some(0),
            "selected_provider should be set to cursor position"
        );
        assert_eq!(
            pane.wizard.current_step(),
            WizardStep::ChannelSetup,
            "Should advance to ChannelSetup"
        );
        assert_eq!(
            pane.wizard.cursor, 0,
            "Cursor should reset to 0 after advancing"
        );
    }

    #[test]
    fn test_provider_selection_enter_on_unconfigured_still_advances() {
        let mut pane = pane_at_provider_selection();
        pane.wizard.provider_list[0].is_configured = false;
        pane.wizard.cursor = 0;

        let action = pane.handle_key(KeyCode::Enter);

        assert_eq!(
            action, MaestroClawAction::WizardAdvanced,
            "Enter on unconfigured provider should still advance (selection-only)"
        );
        assert_eq!(
            pane.wizard.selected_provider,
            Some(0),
            "selected_provider should be set regardless of is_configured"
        );
        assert_eq!(
            pane.wizard.current_step(),
            WizardStep::ChannelSetup,
            "Should advance to ChannelSetup"
        );
        assert!(
            !pane.wizard.provider_list[0].is_configured,
            "is_configured must NOT be mutated by selection"
        );
    }

    #[test]
    fn test_provider_selection_char_key_returns_none() {
        let mut pane = pane_at_provider_selection();

        // Char keys are ignored in selection-only mode
        let action = pane.handle_key(KeyCode::Char('s'));

        assert_eq!(action, MaestroClawAction::None, "Char key should produce None in selection-only mode");
    }

    #[test]
    fn test_cursor_clamped_on_provider_to_tool_back_out() {
        let mut pane = pane_at_provider_selection();

        // Set up a small tool_details to simulate few detected tools
        pane.wizard.tool_details = vec![
            ("claude".to_string(), Some("1.0.0".to_string()), None),
            ("gemini".to_string(), Some("1.0.0".to_string()), None),
        ];

        // Set cursor to last provider (provider_list typically has 5 entries)
        let provider_max = pane.wizard.provider_list.len().saturating_sub(1);
        pane.wizard.cursor = provider_max;

        let action = pane.handle_key(KeyCode::Esc);

        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::PrimaryToolSelection);
        // Cursor must be clamped to tool_details range, not provider_list range
        let tool_max = pane.wizard.tool_details.len().saturating_sub(1);
        assert_eq!(
            pane.wizard.cursor, tool_max,
            "Cursor {} should be clamped to tool_details max {} when backing out to PrimaryToolSelection",
            pane.wizard.cursor, tool_max
        );
    }

    #[test]
    fn test_provider_selection_esc_goes_back() {
        let mut pane = pane_at_provider_selection();

        let action = pane.handle_key(KeyCode::Esc);

        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(
            pane.wizard.current_step(),
            WizardStep::PrimaryToolSelection,
            "Should go back to PrimaryToolSelection"
        );
    }

    #[test]
    fn test_non_char_keys_still_return_none() {
        let mut pane = pane_at_provider_selection();

        // Tab, Home, End should still be ignored (not Char keys)
        let ignored_keys = [KeyCode::Tab, KeyCode::Home, KeyCode::End];

        for key in ignored_keys {
            let action = pane.handle_key(key);
            assert_eq!(
                action, MaestroClawAction::None,
                "Key {:?} should produce None action on ProviderSelection",
                key
            );
        }
    }

    #[test]
    fn test_provider_selection_full_navigation_cycle() {
        let mut pane = pane_at_provider_selection();
        let provider_count = pane.wizard.provider_list.len();

        // Navigate down through all providers
        for i in 0..provider_count.saturating_sub(1) {
            pane.handle_key(KeyCode::Down);
            assert_eq!(pane.wizard.cursor, i + 1);
        }

        // Try to go past the end
        pane.handle_key(KeyCode::Down);
        assert_eq!(
            pane.wizard.cursor,
            provider_count - 1,
            "Should stay at last item"
        );

        // Navigate back up to start
        for i in (0..provider_count - 1).rev() {
            pane.handle_key(KeyCode::Up);
            assert_eq!(pane.wizard.cursor, i);
        }

        // Try to go before start
        pane.handle_key(KeyCode::Up);
        assert_eq!(pane.wizard.cursor, 0, "Should stay at first item");
    }

    #[test]
    fn test_provider_selection_empty_list_keys_safe() {
        let mut pane = pane_at_provider_selection();
        pane.wizard.provider_list.clear();
        pane.wizard.cursor = 0;

        // Should not panic on navigation with empty list
        let action_down = pane.handle_key(KeyCode::Down);
        assert_eq!(action_down, MaestroClawAction::WizardSelection);
        assert_eq!(pane.wizard.cursor, 0);

        let action_up = pane.handle_key(KeyCode::Up);
        assert_eq!(action_up, MaestroClawAction::WizardSelection);
        assert_eq!(pane.wizard.cursor, 0);

        // Enter on empty list should not panic — returns None since get() fails
        let action_enter = pane.handle_key(KeyCode::Enter);
        assert_eq!(action_enter, MaestroClawAction::None);
    }

    #[test]
    fn test_cursor_clamped_on_channel_setup_return() {
        let mut pane = pane_at_provider_selection();
        let provider_count = pane.wizard.provider_list.len();

        // Advance to ChannelSetup
        pane.wizard.provider_list[0].is_configured = true;
        pane.wizard.cursor = 0;
        pane.handle_key(KeyCode::Enter);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);

        // Move cursor to a high value (simulating ChannelSetup having 6 channels)
        pane.wizard.cursor = 5;

        // Press Esc to go back to ProviderSelection
        pane.handle_key(KeyCode::Esc);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);

        // Cursor should be clamped to provider list range
        assert!(
            pane.wizard.cursor < provider_count,
            "Cursor {} should be clamped to provider list len {}",
            pane.wizard.cursor,
            provider_count
        );
    }

    #[test]
    fn test_out_of_range_cursor_enter_is_safe() {
        let mut pane = pane_at_provider_selection();
        let max = pane.wizard.provider_list.len().saturating_sub(1);
        pane.wizard.cursor = max + 10; // way out of range

        let action = pane.handle_key(KeyCode::Enter);

        // Should clamp cursor and return None (not advance)
        assert_eq!(action, MaestroClawAction::None);
        assert_eq!(pane.wizard.cursor, max, "Cursor should be clamped");
    }
}

/// Tests for ChannelSetup step (Phase 4 checklist integration)
#[cfg(test)]
mod channel_setup_tests {
    use crate::maesterclaw::channels::ChannelType;
    use crate::maesterclaw::{MaestroClawAction, MaestroClawPane, WizardStep};
    use crossterm::event::KeyCode;

    /// Helper: create a pane with wizard on ChannelSetup step
    fn pane_at_channel_setup() -> MaestroClawPane {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        // Advance: Welcome -> ToolDetection -> PrimaryToolSelection -> ProviderSelection -> ChannelSetup
        pane.wizard.next_step();
        pane.wizard.next_step();
        pane.wizard.next_step();
        pane.wizard.next_step();
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);
        pane.wizard.cursor = 0;
        pane
    }

    #[test]
    fn test_channel_setup_up_clamps_at_zero() {
        let mut pane = pane_at_channel_setup();
        let action = pane.handle_key(KeyCode::Up);
        assert_eq!(pane.wizard.cursor, 0);
        assert_eq!(action, MaestroClawAction::WizardSelection);
    }

    #[test]
    fn test_channel_setup_down_moves_cursor() {
        let mut pane = pane_at_channel_setup();
        pane.handle_key(KeyCode::Down);
        assert_eq!(pane.wizard.cursor, 1);
        pane.handle_key(KeyCode::Down);
        assert_eq!(pane.wizard.cursor, 2);
    }

    #[test]
    fn test_channel_setup_down_clamps_at_continue_button() {
        let mut pane = pane_at_channel_setup();
        let channel_count = ChannelType::all().len();
        for _ in 0..channel_count + 2 {
            pane.handle_key(KeyCode::Down);
        }
        // Continue button is at index == channel_count, so cursor should stop there
        assert_eq!(pane.wizard.cursor, channel_count);
    }

    #[test]
    fn test_channel_setup_space_toggles_channel() {
        let mut pane = pane_at_channel_setup();
        let channels = ChannelType::all();

        // Toggle first channel (Telegram)
        pane.handle_key(KeyCode::Char(' '));
        assert!(
            pane.wizard.selected_channels.contains(&channels[0]),
            "Telegram should be selected after Space"
        );

        // Toggle again to deselect
        pane.handle_key(KeyCode::Char(' '));
        assert!(
            !pane.wizard.selected_channels.contains(&channels[0]),
            "Telegram should be deselected after second Space"
        );
    }

    #[test]
    fn test_channel_setup_space_toggles_different_channels() {
        let mut pane = pane_at_channel_setup();
        let channels = ChannelType::all();

        // Select first channel
        pane.handle_key(KeyCode::Char(' '));
        // Move to second channel
        pane.handle_key(KeyCode::Down);
        // Select second channel
        pane.handle_key(KeyCode::Char(' '));

        assert_eq!(pane.wizard.selected_channels.len(), 2);
        assert!(pane.wizard.selected_channels.contains(&channels[0]));
        assert!(pane.wizard.selected_channels.contains(&channels[1]));
    }

    #[test]
    fn test_channel_setup_enter_on_item_toggles() {
        let mut pane = pane_at_channel_setup();
        let channels = ChannelType::all();

        // Enter on an item should toggle it (same as Space)
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::WizardSelection);
        assert!(pane.wizard.selected_channels.contains(&channels[0]));
    }

    #[test]
    fn test_channel_setup_enter_on_continue_advances() {
        let mut pane = pane_at_channel_setup();
        let channel_count = ChannelType::all().len();

        // Move cursor to Continue button
        for _ in 0..channel_count {
            pane.handle_key(KeyCode::Down);
        }
        assert!(pane.wizard.cursor == channel_count, "Should be on Continue button");

        // Enter on Continue should advance to ToolSummary
        let action = pane.handle_key(KeyCode::Enter);
        assert_eq!(action, MaestroClawAction::WizardAdvanced);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);
        assert_eq!(pane.wizard.cursor, 0, "Cursor should reset after advancing");
    }

    #[test]
    fn test_channel_setup_esc_goes_back() {
        let mut pane = pane_at_channel_setup();
        let channels = ChannelType::all();

        // Select a channel
        pane.handle_key(KeyCode::Char(' '));
        assert!(pane.wizard.selected_channels.contains(&channels[0]));

        // Press Esc
        let action = pane.handle_key(KeyCode::Esc);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ProviderSelection);

        // Cursor should be clamped to provider list range
        let max = pane.wizard.provider_list.len().saturating_sub(1);
        assert!(
            pane.wizard.cursor <= max,
            "Cursor {} should be clamped to provider list len {}",
            pane.wizard.cursor,
            max
        );

        // Selected channels should persist across back navigation
        assert!(
            pane.wizard.selected_channels.contains(&channels[0]),
            "Channel selection should persist when going back"
        );
    }

    #[test]
    fn test_channel_setup_selection_preserved_after_advance() {
        let mut pane = pane_at_channel_setup();
        let channels = ChannelType::all();

        // Select a couple channels
        pane.handle_key(KeyCode::Char(' '));
        pane.handle_key(KeyCode::Down);
        pane.handle_key(KeyCode::Char(' '));

        // Advance to ToolSummary
        let channel_count = ChannelType::all().len();
        for _ in 0..channel_count {
            pane.handle_key(KeyCode::Down);
        }
        pane.handle_key(KeyCode::Enter);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // Selections should persist
        assert_eq!(pane.wizard.selected_channels.len(), 2);
        assert!(pane.wizard.selected_channels.contains(&channels[0]));
        assert!(pane.wizard.selected_channels.contains(&channels[1]));
    }

    // ── Phase 7: Left / BackTab key handling in simple wizard steps ──

    #[test]
    fn test_left_key_goes_back_from_tool_detection() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);

        // Advance to ToolDetection
        pane.handle_key(KeyCode::Enter);
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolDetection);

        // Left should go back to Welcome
        let action = pane.handle_key(KeyCode::Left);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);
    }

    #[test]
    fn test_backtab_key_goes_back_from_tool_summary() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        // Advance through all interactive steps to reach ToolSummary
        pane.handle_key(KeyCode::Enter); // Welcome → ToolDetection
        pane.handle_key(KeyCode::Enter); // ToolDetection → PrimaryToolSelection
        pane.handle_key(KeyCode::Enter); // PrimaryToolSelection → ProviderSelection
        pane.handle_key(KeyCode::Enter); // ProviderSelection → ChannelSetup
        // Advance past all channels to Continue
        for _ in 0..ChannelType::all().len() {
            pane.handle_key(KeyCode::Down);
        }
        pane.handle_key(KeyCode::Enter); // ChannelSetup → ToolSummary
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // BackTab should go back to ChannelSetup
        let action = pane.handle_key(KeyCode::BackTab);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert_eq!(pane.wizard.current_step(), WizardStep::ChannelSetup);
    }

    #[test]
    fn test_left_key_noops_on_welcome() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);

        // Left on Welcome is a no-op (previous_step is Welcome→Welcome)
        let action = pane.handle_key(KeyCode::Left);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::Welcome);
    }

    #[test]
    fn test_left_key_dismisses_from_complete() {
        let mut pane = MaestroClawPane::default();
        pane.activate_wizard();
        // Advance through all steps to Complete
        pane.handle_key(KeyCode::Enter); // Welcome → ToolDetection
        pane.handle_key(KeyCode::Enter); // ToolDetection → PrimaryToolSelection
        pane.handle_key(KeyCode::Enter); // PrimaryToolSelection → ProviderSelection
        pane.handle_key(KeyCode::Enter); // ProviderSelection → ChannelSetup
        for _ in 0..ChannelType::all().len() {
            pane.handle_key(KeyCode::Down);
        }
        pane.handle_key(KeyCode::Enter); // ChannelSetup → ToolSummary
        pane.handle_key(KeyCode::Enter); // ToolSummary → Complete

        assert_eq!(pane.wizard.current_step(), WizardStep::Complete);
        // Wizard stays active on Complete (user must explicitly dismiss via
        // Enter/Left/BackTab; Esc navigates back to ToolSummary).

        // Left on Complete should go back to ToolSummary (not dismiss)
        let action = pane.handle_key(KeyCode::Left);
        assert_eq!(action, MaestroClawAction::WizardBack);
        assert!(pane.is_wizard_active());
        assert_eq!(pane.wizard.current_step(), WizardStep::ToolSummary);

        // Re-activate wizard after completion — should reset to Welcome, not
        // reopen the stale Complete screen.
        pane.activate_wizard();
        assert!(pane.is_wizard_active());
        assert_eq!(
            pane.wizard.current_step(),
            WizardStep::Welcome,
            "activate_wizard after completion should reset to Welcome"
        );
    }
}
