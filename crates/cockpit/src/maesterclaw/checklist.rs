//! Checklist widget for MaestroClaw
//!
//! This module provides a checklist widget with toggleable items,
//! supporting pre-selection and navigation with a Continue button.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::collections::HashSet;

/// Interactive checklist widget with toggleable items
#[derive(Debug, Clone)]
pub struct Checklist {
    /// List of items in the checklist
    pub items: Vec<String>,
    /// Set of selected item indices
    pub selected: HashSet<usize>,
    /// Current cursor position (0..=items.len() where items.len() is the Continue button)
    pub cursor: usize,
    /// Title displayed above the checklist
    pub title: String,
}

impl Checklist {
    /// Create a new checklist with the given title and items
    ///
    /// # Arguments
    /// * `title` - The title displayed above the checklist
    /// * `items` - The list of items to display
    ///
    /// # Returns
    /// A new Checklist with an empty selection set and cursor at position 0
    pub fn new(title: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            items,
            selected: HashSet::new(),
            cursor: 0,
            title: title.into(),
        }
    }

    /// Create a new checklist with pre-selected items
    ///
    /// # Arguments
    /// * `title` - The title displayed above the checklist
    /// * `items` - The list of items to display
    /// * `pre_selected` - Set of indices that should be pre-selected
    ///
    /// # Returns
    /// A new Checklist with the specified items and pre-selected indices
    pub fn with_selected(
        title: impl Into<String>,
        items: Vec<String>,
        pre_selected: HashSet<usize>,
    ) -> Self {
        Self {
            items,
            selected: pre_selected,
            cursor: 0,
            title: title.into(),
        }
    }

    /// Toggle the selection state of the item at the current cursor position
    ///
    /// If the cursor is on the Continue button (past the last item), this does nothing.
    pub fn toggle_current(&mut self) {
        if self.cursor < self.items.len() {
            if self.selected.contains(&self.cursor) {
                self.selected.remove(&self.cursor);
            } else {
                self.selected.insert(self.cursor);
            }
        }
    }

    /// Move the cursor up to the previous item
    ///
    /// The cursor stops at position 0 (the first item).
    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the cursor down to the next item or Continue button
    ///
    /// The cursor stops at position `items.len()` which represents the Continue button.
    pub fn move_down(&mut self) {
        if self.cursor < self.items.len() {
            self.cursor += 1;
        }
    }

    /// Check if the cursor is currently on the Continue button
    ///
    /// # Returns
    /// `true` if cursor is at position `items.len()`, `false` otherwise
    pub fn is_on_continue(&self) -> bool {
        self.cursor == self.items.len()
    }

    /// Get the current selection as a vector of selected item strings
    ///
    /// # Returns
    /// A vector containing the text of all selected items, in order
    pub fn get_selected_items(&self) -> Vec<String> {
        let mut indices: Vec<_> = self.selected.iter().copied().collect();
        indices.sort();
        indices
            .into_iter()
            .filter_map(|idx| self.items.get(idx).cloned())
            .collect()
    }

    /// Check if a specific item index is selected
    ///
    /// # Arguments
    /// * `idx` - The index to check
    ///
    /// # Returns
    /// `true` if the item at the given index is selected
    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected.contains(&idx)
    }

    /// Render the checklist to the given frame within the specified area
    ///
    /// # Arguments
    /// * `frame` - The ratatui Frame to render to
    /// * `area` - The rectangular area to render within
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Create the block with title and border
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let mut list_items = Vec::new();

        // Render each checklist item
        for (idx, item) in self.items.iter().enumerate() {
            let is_cursor = idx == self.cursor;
            let is_selected = self.selected.contains(&idx);

            // Build the line for this item
            let mut spans = Vec::new();

            // Arrow indicator (→ for cursor, spaces otherwise)
            let arrow = if is_cursor { "→ " } else { "  " };
            spans.push(Span::styled(
                arrow,
                Style::default().fg(Color::Green),
            ));

            // Checkbox with color based on selection state
            let checkbox = if is_selected { "[✓]" } else { "[ ]" };
            let checkbox_style = Style::default().fg(if is_selected {
                Color::Green
            } else {
                Color::DarkGray
            });
            spans.push(Span::styled(checkbox, checkbox_style));
            spans.push(Span::raw(" "));

            // Item text with style based on cursor and selection state
            let text_style = if is_selected && is_cursor {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(Color::Cyan)
            } else if is_cursor {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            spans.push(Span::styled(item, text_style));

            list_items.push(ListItem::new(Line::from(spans)));
        }

        // Add blank line before Continue button
        list_items.push(ListItem::new(Line::from("")));

        // Add Continue button with arrow indicator matching item pattern
        let is_on_continue = self.is_on_continue();
        let continue_text = Span::styled(
            "Continue →",
            if is_on_continue {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        );
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                if is_on_continue { "→ " } else { "  " },
                Style::default().fg(Color::Green),
            ),
            continue_text,
        ])));

        // Create and render the list
        let list = List::new(list_items).block(block);
        frame.render_widget(list, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_checklist() {
        let checklist = Checklist::new("Test", vec!["Item 1".to_string(), "Item 2".to_string()]);
        assert_eq!(checklist.items.len(), 2);
        assert!(checklist.selected.is_empty());
        assert_eq!(checklist.cursor, 0);
        assert_eq!(checklist.title, "Test");
    }

    #[test]
    fn test_with_selected() {
        let mut pre_selected = HashSet::new();
        pre_selected.insert(1);
        let checklist = Checklist::with_selected(
            "Test",
            vec!["Item 1".to_string(), "Item 2".to_string()],
            pre_selected,
        );
        assert_eq!(checklist.selected.len(), 1);
        assert!(checklist.is_selected(1));
        assert!(!checklist.is_selected(0));
    }

    #[test]
    fn test_toggle_current() {
        let mut checklist = Checklist::new("Test", vec!["Item 1".to_string(), "Item 2".to_string()]);
        checklist.toggle_current();
        assert!(checklist.is_selected(0));
        checklist.toggle_current();
        assert!(!checklist.is_selected(0));
    }

    #[test]
    fn test_move_up() {
        let mut checklist = Checklist::new("Test", vec!["Item 1".to_string(), "Item 2".to_string()]);
        checklist.move_down();
        checklist.move_down();
        checklist.move_up();
        assert_eq!(checklist.cursor, 1);
        checklist.move_up();
        assert_eq!(checklist.cursor, 0);
        checklist.move_up();
        assert_eq!(checklist.cursor, 0); // stays at 0
    }

    #[test]
    fn test_move_down() {
        let mut checklist = Checklist::new("Test", vec!["Item 1".to_string(), "Item 2".to_string()]);
        checklist.move_down();
        assert_eq!(checklist.cursor, 1);
        checklist.move_down();
        assert_eq!(checklist.cursor, 2); // On Continue button
        checklist.move_down();
        assert_eq!(checklist.cursor, 2); // stays at 2
    }

    #[test]
    fn test_is_on_continue() {
        let mut checklist = Checklist::new("Test", vec!["Item 1".to_string()]);
        assert!(!checklist.is_on_continue());
        checklist.move_down();
        assert!(checklist.is_on_continue());
    }

    #[test]
    fn test_toggle_on_continue_does_nothing() {
        let mut checklist = Checklist::new("Test", vec!["Item 1".to_string()]);
        checklist.move_down(); // Move to Continue button
        checklist.toggle_current();
        assert!(checklist.selected.is_empty());
    }

    #[test]
    fn test_get_selected_items() {
        let mut checklist = Checklist::new("Test", vec![
            "Item 1".to_string(),
            "Item 2".to_string(),
            "Item 3".to_string(),
        ]);
        checklist.selected.insert(0);
        checklist.selected.insert(2);
        let selected = checklist.get_selected_items();
        assert_eq!(selected.len(), 2);
        // Note: order may vary due to HashSet, but both items should be present
        assert!(selected.contains(&"Item 1".to_string()));
        assert!(selected.contains(&"Item 3".to_string()));
    }
}
