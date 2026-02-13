//! Modal trait definitions and shared infrastructure for Conductor TUI.
//!
//! Provides trait-based abstraction for modal overlays enabling:
//! - Polymorphic modal handling in ConductorPane
//! - Shared rendering and key handling infrastructure
//! - Type-safe action results per modal type

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders},
    Frame,
};

use super::theme::ConductorTheme;

/// Result of modal interaction - returned when modal is submitted or cancelled
pub type ModalResult<T> = Result<T, ModalCancelled>;

/// Error type indicating user cancelled the modal (via Esc)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalCancelled;

/// Core modal trait - all modals implement this for basic interaction
///
/// # Example
/// ```ignore
/// struct MyModal { visible: bool, value: Option<String> }
///
/// impl Modal for MyModal {
///     type Output = String;
///
///     fn handle_key(&mut self, key: KeyEvent) -> Option<ModalResult<String>> {
///         match key.code {
///             KeyCode::Enter => Some(Ok(self.value.take().unwrap())),
///             KeyCode::Esc => Some(Err(ModalCancelled)),
///             _ => None,
///         }
///     }
///
///     fn render(&self, frame: &mut Frame, area: Rect, theme: &ConductorTheme) {
///         // Render modal as centered overlay
///     }
///
///     fn is_visible(&self) -> bool { self.visible }
///     fn show(&mut self) { self.visible = true; }
///     fn hide(&mut self) { self.visible = false; }
///
///     fn take_output(&mut self) -> Option<String> {
///         if self.visible { None } else { self.value.take() }
///     }
/// }
/// }
/// ```
pub trait Modal {
    /// The value type returned when this modal is submitted successfully
    type Output;

    /// Process a key event. Returns Some when modal concludes (submit/cancel).
    /// Returns None when key was handled but modal remains open.
    ///
    /// # Returns
    /// - `Some(Ok(value))` - User submitted modal with output value
    /// - `Some(Err(ModalCancelled))` - User cancelled modal via Esc
    /// - `None` - Key handled but modal remains open
    fn handle_key(&mut self, key: KeyEvent) -> Option<ModalResult<Self::Output>>;

    /// Render modal as a centered overlay.
    /// No-op when `self.is_visible()` returns false.
    fn render(&self, frame: &mut Frame, area: Rect, theme: &ConductorTheme);

    /// Check if modal is currently visible/active
    fn is_visible(&self) -> bool;

    /// Show modal (sets visible = true)
    fn show(&mut self);

    /// Hide modal (sets visible = false)
    fn hide(&mut self);

    /// Take and return submitted value, resetting internal state.
    /// Returns None if modal is still visible (no output available).
    fn take_output(&mut self) -> Option<Self::Output>;
}

/// Trait for text input modals with cursor management
///
/// Provides methods for manipulating text input buffers,
/// including cursor positioning, character insertion/deletion,
/// and text navigation.
pub trait TextInputModal: Modal {
    /// Get current input buffer contents
    fn buffer(&self) -> &str;

    /// Get current cursor position (byte offset)
    fn cursor_pos(&self) -> usize;

    /// Set cursor position (clamped to buffer length)
    fn set_cursor_pos(&mut self, pos: usize);

    /// Insert character at cursor position
    fn insert_char(&mut self, ch: char);

    /// Delete character before cursor (backspace)
    fn delete_before_cursor(&mut self);

    /// Move cursor left one position
    fn move_cursor_left(&mut self);

    /// Move cursor right one position
    fn move_cursor_right(&mut self);

    /// Clear input buffer and reset cursor
    fn clear(&mut self);

    /// Handle navigation key events for cursor movement.
    /// Returns true if key was handled, false if not.
    ///
    /// # Default Implementation
    /// Handles: Left, Right, Home, End arrows
    fn handle_navigation_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left => {
                self.move_cursor_left();
                true
            }
            KeyCode::Right => {
                self.move_cursor_right();
                true
            }
            KeyCode::Home => {
                self.set_cursor_pos(0);
                true
            }
            KeyCode::End => {
                self.set_cursor_pos(self.buffer().len());
                true
            }
            _ => false,
        }
    }

    /// Handle editing key events for text modification.
    /// Returns true if key was handled, false if not.
    ///
    /// # Default Implementation
    /// Handles: Backspace, Char(c)
    fn handle_editing_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Backspace => {
                self.delete_before_cursor();
                true
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                true
            }
            _ => false,
        }
    }
}

/// Trait for list selector modals with navigation
///
/// Provides methods for managing item lists and selection,
/// including moving selection up/down and retrieving selected items.
pub trait ListSelectorModal: Modal {
    /// Item type stored in the selector
    type Item;

    /// Get items in the selector
    fn items(&self) -> &[Self::Item];

    /// Get currently selected index
    fn selected_index(&self) -> usize;

    /// Set selected index (clamped to valid range)
    fn set_selected_index(&mut self, index: usize);

    /// Move selection up one position.
    /// Wraps to bottom if at top item.
    fn move_selection_up(&mut self);

    /// Move selection down one position.
    /// Wraps to top if at bottom item.
    fn move_selection_down(&mut self);

    /// Get currently selected item
    fn selected_item(&self) -> Option<&Self::Item>;

    /// Handle navigation key events for list selection.
    /// Returns Some(output) if selection submitted, None otherwise.
    ///
    /// # Default Implementation
    /// Handles: Up, Down, j/k keys
    fn handle_navigation_key(&mut self, key: KeyEvent) -> Option<ModalResult<Self::Output>> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.move_selection_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.move_selection_down();
                None
            }
            _ => None,
        }
    }
}

/// Re-export KeyCode for use in trait impls
pub use crossterm::event::KeyCode;

/// Helper: Create centered rectangle for modal overlay
///
/// Calculates a centered rectangle within the given area
/// based on percentage dimensions.
///
/// # Arguments
/// * `percent_x` - Width percentage (0-100)
/// * `percent_y` - Height percentage (0-100)
/// * `area` - Parent area to center within
///
/// # Example
/// ```ignore
/// let modal_area = centered_modal_rect(60, 30, parent_area);
/// ```
pub fn centered_modal_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Helper: Render modal frame with title and border
///
/// Creates a styled block widget suitable for modal overlays.
pub fn render_modal_frame<'a>(title: &'a str, theme: &ConductorTheme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(Line::from(format!(" {} ", title)))
        .border_style(Style::default().fg(theme.accent_primary))
}

/// Helper: Render modal footer hints
///
/// Renders centered footer text with navigation hints.
pub fn render_modal_footer(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    theme: &ConductorTheme,
) {
    use ratatui::widgets::Paragraph;

    let footer = Paragraph::new(text)
        .style(Style::default().fg(theme.fg_dim))
        .alignment(Alignment::Center);
    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_modal_rect() {
        let area = Rect::new(0, 0, 100, 30);
        let centered = centered_modal_rect(50, 50, area);

        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 7);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 15);
    }

    #[test]
    fn test_modal_cancelled_is_debug() {
        let cancelled = ModalCancelled;
        assert_eq!(format!("{:?}", cancelled), "ModalCancelled");
    }

    #[test]
    fn test_modal_result_variants() {
        let ok: ModalResult<String> = Ok("value".to_string());
        let err: ModalResult<String> = Err(ModalCancelled);

        assert!(ok.is_ok());
        assert!(err.is_err());
    }
}
