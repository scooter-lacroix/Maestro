use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::modals::{Modal, ModalCancelled, ModalResult, TextInputModal};
use super::theme::ConductorTheme;

/// Centered-rect helper shared across modal overlays.
pub(super) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Result of processing a key event in the input modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    /// No meaningful action; the key was ignored.
    None,
    /// User submitted the input text.
    Submit(String),
    /// User cancelled the modal.
    Cancel,
    /// Key was consumed by the modal (cursor move, character typed, etc.).
    Handled,
}

/// A reusable text-input modal overlay for the Conductor TUI.
pub struct InputModal {
    pub title: String,
    pub prompt_text: String,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub visible: bool,
    pub multiline: bool,
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Modal for InputModal {
    type Output = String;

    fn handle_key(&mut self, key: KeyEvent) -> Option<ModalResult<String>> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc => {
                self.visible = false;
                self.clear(); // Clear buffer to prevent stale input on cancellation
                Some(Err(ModalCancelled))
            }
            KeyCode::Enter => {
                if self.multiline && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.insert_char('\n');
                    None
                } else {
                    let text = self.take_input();
                    self.visible = false;
                    Some(Ok(text))
                }
            }
            _ => {
                // Delegate to trait methods for navigation/editing
                if self.handle_navigation_key(key) || self.handle_editing_key(key) {
                    None
                } else {
                    None
                }
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        render_input_modal(f, area, self, theme)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn take_output(&mut self) -> Option<String> {
        if self.visible {
            None
        } else {
            Some(self.take_input())
        }
    }
}

impl TextInputModal for InputModal {
    fn buffer(&self) -> &str {
        &self.input_buffer
    }

    fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    fn set_cursor_pos(&mut self, pos: usize) {
        self.cursor_pos = pos.min(self.input_buffer.len());
    }

    fn insert_char(&mut self, ch: char) {
        self.input_buffer.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
    }

    fn delete_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input_buffer.remove(self.cursor_pos);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input_buffer.len() {
            self.cursor_pos += 1;
        }
    }

    fn clear(&mut self) {
        self.input_buffer.clear();
        self.cursor_pos = 0;
    }
}

// ============================================================================
// Original InputModal Implementation
// ============================================================================

impl InputModal {
    pub fn new(title: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            prompt_text: prompt.into(),
            input_buffer: String::new(),
            cursor_pos: 0,
            visible: false,
            multiline: false,
        }
    }

    /// Process a key event. Only call when `visible` is true.
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        if !self.visible {
            return InputAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.visible = false;
                InputAction::Cancel
            }
            KeyCode::Enter => {
                if self.multiline && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.input_buffer.insert(self.cursor_pos, '\n');
                    self.cursor_pos += 1;
                    InputAction::Handled
                } else {
                    let text = self.take_input();
                    self.visible = false;
                    InputAction::Submit(text)
                }
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input_buffer.remove(self.cursor_pos);
                }
                InputAction::Handled
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                InputAction::Handled
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input_buffer.len() {
                    self.cursor_pos += 1;
                }
                InputAction::Handled
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                InputAction::Handled
            }
            KeyCode::End => {
                self.cursor_pos = self.input_buffer.len();
                InputAction::Handled
            }
            KeyCode::Char(c) => {
                self.input_buffer.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                InputAction::Handled
            }
            _ => InputAction::None,
        }
    }

    /// Drain and return the current input, resetting the buffer and cursor.
    pub fn take_input(&mut self) -> String {
        self.cursor_pos = 0;
        std::mem::take(&mut self.input_buffer)
    }
}

/// Render the input modal as a centered overlay.
///
/// This is a no-op when `modal.visible` is false.
pub fn render_input_modal(f: &mut Frame, area: Rect, modal: &InputModal, theme: &ConductorTheme) {
    if !modal.visible {
        return;
    }

    let modal_area = centered_rect(60, 30, area);
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", modal.title))
        .border_style(Style::default().fg(theme.accent_primary))
        .style(Style::default().bg(theme.bg_secondary));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Split inner area: prompt, input field, footer hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // prompt
            Constraint::Min(1),    // input field
            Constraint::Length(1), // footer hints
        ])
        .split(inner);

    // -- Prompt text --
    let prompt = Paragraph::new(modal.prompt_text.as_str())
        .style(Style::default().fg(theme.fg_secondary))
        .wrap(Wrap { trim: true });
    f.render_widget(prompt, chunks[0]);

    // -- Input field with cursor --
    let before_cursor = &modal.input_buffer[..modal.cursor_pos];
    let _after_cursor = &modal.input_buffer[modal.cursor_pos..];

    let cursor_char = if modal.cursor_pos < modal.input_buffer.len() {
        let ch = modal.input_buffer[modal.cursor_pos..]
            .chars()
            .next()
            .unwrap();
        // Show the character under the cursor with inverted style
        ch.to_string()
    } else {
        "█".to_string()
    };

    let after_display = if modal.cursor_pos < modal.input_buffer.len() {
        // Skip the character that is shown as the cursor
        let skip = modal.input_buffer[modal.cursor_pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        &modal.input_buffer[modal.cursor_pos + skip..]
    } else {
        ""
    };

    let input_line = Line::from(vec![
        Span::styled(before_cursor, Style::default().fg(theme.fg_primary)),
        Span::styled(
            cursor_char,
            Style::default()
                .fg(theme.bg_secondary)
                .bg(theme.fg_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(after_display, Style::default().fg(theme.fg_primary)),
    ]);

    let input_widget = Paragraph::new(input_line)
        .style(Style::default().bg(theme.bg_tertiary))
        .wrap(Wrap { trim: false });
    f.render_widget(input_widget, chunks[1]);

    // -- Footer hints --
    let hint = if modal.multiline {
        "[Ctrl+Enter] Submit  [Esc] Cancel"
    } else {
        "[Enter] Submit  [Esc] Cancel"
    };
    let footer = Paragraph::new(hint)
        .style(Style::default().fg(theme.fg_muted))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}
