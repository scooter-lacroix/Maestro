use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::modals::{ListSelectorModal, Modal, ModalCancelled, ModalResult};
use super::theme::ConductorTheme;

/// A single item in the selector list.
pub struct SelectorItem {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
}

/// Result of handling a key event in the selector modal.
pub enum SelectorAction {
    None,
    Selected(String),
    Cancel,
}

/// A reusable list selector modal overlay.
pub struct SelectorModal {
    pub title: String,
    pub items: Vec<SelectorItem>,
    pub selected: usize,
    pub visible: bool,
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Modal for SelectorModal {
    type Output = String;

    fn handle_key(&mut self, key: KeyEvent) -> Option<ModalResult<String>> {
        if !self.visible || self.items.is_empty() {
            return None;
        }

        // Use default navigation handling from trait
        if let Some(result) = self.handle_navigation_key(key) {
            return Some(result);
        }

        match key.code {
            KeyCode::Enter => {
                let value = self.items.get(self.selected).map(|item| item.value.clone()).unwrap_or_default();
                self.visible = false;
                Some(Ok(value))
            }
            KeyCode::Esc => {
                self.visible = false;
                Some(Err(ModalCancelled))
            }
            _ => None,
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        render_selector_modal(f, area, self, theme)
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
            self.items.get(self.selected).map(|item| item.value.clone())
        }
    }
}

impl ListSelectorModal for SelectorModal {
    type Item = SelectorItem;

    fn items(&self) -> &[SelectorItem] {
        &self.items
    }

    fn selected_index(&self) -> usize {
        self.selected
    }

    fn set_selected_index(&mut self, index: usize) {
        self.selected = if self.items.is_empty() {
            0
        } else {
            index.min(self.items.len() - 1)
        };
    }

    fn move_selection_up(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.items.len() - 1;
        }
    }

    fn move_selection_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    fn selected_item(&self) -> Option<&SelectorItem> {
        self.items.get(self.selected)
    }
}

// ============================================================================
// Original SelectorModal Implementation
// ============================================================================

impl SelectorModal {
    /// Create a selector from `(label, value)` pairs.
    pub fn new(title: &str, items: Vec<(&str, &str)>) -> Self {
        let items = items
            .into_iter()
            .map(|(label, value)| SelectorItem {
                label: label.to_string(),
                value: value.to_string(),
                description: None,
            })
            .collect();
        Self {
            title: title.to_string(),
            items,
            selected: 0,
            visible: true,
        }
    }

    /// Create a selector from `(label, value, description)` tuples.
    pub fn new_with_descriptions(title: &str, items: Vec<(&str, &str, &str)>) -> Self {
        let items = items
            .into_iter()
            .map(|(label, value, desc)| SelectorItem {
                label: label.to_string(),
                value: value.to_string(),
                description: Some(desc.to_string()),
            })
            .collect();
        Self {
            title: title.to_string(),
            items,
            selected: 0,
            visible: true,
        }
    }

    /// Handle a key event, returning the resulting action.
    pub fn handle_key(&mut self, key: KeyEvent) -> SelectorAction {
        if !self.visible || self.items.is_empty() {
            return SelectorAction::None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = self.items.len() - 1;
                }
                SelectorAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                } else {
                    self.selected = 0;
                }
                SelectorAction::None
            }
            KeyCode::Enter => {
                let value = self.items[self.selected].value.clone();
                self.visible = false;
                SelectorAction::Selected(value)
            }
            KeyCode::Esc => {
                self.visible = false;
                SelectorAction::Cancel
            }
            _ => SelectorAction::None,
        }
    }

    /// Get the value of the currently highlighted item.
    pub fn selected_value(&self) -> Option<&str> {
        self.items.get(self.selected).map(|item| item.value.as_str())
    }
}

/// Render the selector modal as a centered overlay.
pub fn render_selector_modal(
    f: &mut Frame,
    area: Rect,
    modal: &SelectorModal,
    theme: &ConductorTheme,
) {
    if !modal.visible {
        return;
    }

    let selector_area = centered_rect(50, 50, area);
    f.render_widget(Clear, selector_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", modal.title))
        .border_style(Style::default().fg(theme.accent_primary));

    if modal.items.is_empty() {
        let text = vec![Line::from("No items available.")];
        let p = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, selector_area);
        return;
    }

    let items: Vec<ListItem> = modal
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == modal.selected;
            let style = if is_selected {
                Style::default()
                    .fg(theme.fg_primary)
                    .bg(theme.bg_highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_secondary)
            };

            let mut lines = vec![Line::from(Span::styled(
                format!(" {} ", item.label),
                style,
            ))];

            if let Some(ref desc) = item.description {
                let desc_style = if is_selected {
                    Style::default()
                        .fg(theme.fg_muted)
                        .bg(theme.bg_highlight)
                } else {
                    Style::default().fg(theme.fg_dim)
                };
                lines.push(Line::from(Span::styled(
                    format!("   {}", desc),
                    desc_style,
                )));
            }

            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, selector_area);

    // Footer hint
    let help_area = Rect::new(
        selector_area.x + 1,
        selector_area.y + selector_area.height - 2,
        selector_area.width - 2,
        1,
    );
    let help_text =
        Paragraph::new("↑/↓: Navigate • Enter: Select • Esc: Cancel")
            .style(Style::default().fg(theme.fg_dim))
            .alignment(Alignment::Center);
    f.render_widget(help_text, help_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
