//! Memory Browser Overlay for Conductor TUI
//!
//! Provides an overlay to browse, search, filter, and manage memories
//! associated with the current track.

use crate::conductor::modals::{Modal, ModalCancelled, ModalResult, TextInputModal};
use crate::conductor::selector_modal::SelectorModal;
use crate::conductor::theme::ConductorTheme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use leindex_core::memory::models::{Memory, MemoryCategory};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Memory Browser Overlay
pub struct MemoryBrowser {
    /// List of memories being browsed
    pub memories: Vec<Memory>,
    /// Selected memory index
    pub selected: usize,
    /// Search query for filtering
    pub search_query: String,
    /// Category filter (None = all)
    pub category_filter: Option<MemoryCategory>,
    /// Current page for pagination
    pub page: usize,
    /// Page size for pagination
    pub page_size: usize,
    /// Whether to browser is visible
    pub visible: bool,
    /// Whether search input is focused
    pub search_focused: bool,
    /// Search input modal
    pub search_modal: TextInputModal,
    /// Category selector modal
    pub category_modal: SelectorModal,
    /// Store memory modal
    pub store_modal: TextInputModal,
    /// Delete confirmation modal
    pub delete_modal: SelectorModal,
    /// Selected category for new memory
    pub selected_category: MemoryCategory,
}

impl Default for MemoryBrowser {
    fn default() -> Self {
        Self {
            memories: Vec::new(),
            selected: 0,
            search_query: String::new(),
            category_filter: None,
            page: 0,
            page_size: 50,
            visible: false,
            search_focused: false,
            search_modal: TextInputModal::new("Search Memories", "Enter search query:"),
            category_modal: SelectorModal {
                title: "Select Category".to_string(),
                items: vec![
                    "All Categories".to_string(),
                    "General".to_string(),
                    "Knowledge".to_string(),
                    "Preferences".to_string(),
                    "Specifications".to_string(),
                    "Fact".to_string(),
                    "Pattern".to_string(),
                    "Decision".to_string(),
                    "Context".to_string(),
                    "Temporary".to_string(),
                    "Observation".to_string(),
                ],
                selected: 0,
                visible: false,
            },
            store_modal: TextInputModal::new("New Memory", "Enter memory content:"),
            delete_modal: SelectorModal {
                title: "Delete Memory".to_string(),
                items: vec![
                    "Yes, delete this memory".to_string(),
                    "No, cancel".to_string(),
                ],
                selected: 0,
                visible: false,
            },
            selected_category: MemoryCategory::General,
        }
    }
}

impl MemoryBrowser {
    /// Show to memory browser overlay
    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
        self.page = 0;
    }

    /// Hide to memory browser overlay
    pub fn hide(&mut self) {
        self.visible = false;
        self.search_focused = false;
        self.search_modal.hide();
        self.category_modal.hide();
        self.store_modal.hide();
        self.delete_modal.hide();
    }

    /// Check if browser is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update to memory list
    pub fn update_memories(&mut self, memories: Vec<Memory>) {
        self.memories = memories;
        self.selected = 0;
        self.page = 0;
    }

    /// Get filtered memories based on search and category filter
    pub fn filtered_memories(&self) -> Vec<&Memory> {
        self.memories
            .iter()
            .filter(|m| {
                // Apply search filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    let content = m.content.to_lowercase();
                    if !content.contains(&query) {
                        return false;
                    }
                }

                // Apply category filter
                if let Some(filter_cat) = self.category_filter {
                    if m.category != filter_cat {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get current page of memories
    pub fn current_page_memories(&self) -> Vec<&Memory> {
        let filtered = self.filtered_memories();
        let start = self.page * self.page_size;
        let end = (start + self.page_size).min(filtered.len());

        if start < filtered.len() {
            filtered[start..end].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Move selection up
    pub fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn move_selection_down(&mut self) {
        let page_memories = self.current_page_memories();
        if self.selected < page_memories.len() - 1 {
            self.selected += 1;
        }
    }

    /// Go to previous page
    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
            self.selected = 0;
        }
    }

    /// Go to next page
    pub fn next_page(&mut self) {
        let filtered_len = self.filtered_memories().len();
        let max_page = (filtered_len / self.page_size).max(1) - 1;
        if self.page < max_page {
            self.page += 1;
            self.selected = 0;
        }
    }

    /// Get currently selected memory
    pub fn selected_memory(&self) -> Option<&Memory> {
        let page_memories = self.current_page_memories();
        page_memories.get(self.selected)
    }

    /// Handle key events for to memory browser
    pub fn handle_key(&mut self, key: KeyEvent) -> MemoryBrowserAction {
        // Handle active modals first
        if self.category_modal.is_visible() {
            if let Some(result) = self.category_modal.handle_key(key) {
                return match result {
                    Ok(idx) => {
                        if idx == 0 {
                            self.category_filter = None;
                        } else if let Some(cat) = self.index_to_category(idx) {
                            self.category_filter = Some(cat);
                        }
                        MemoryBrowserAction::CategorySelected
                    }
                    Err(ModalCancelled) => MemoryBrowserAction::CategoryCancelled,
                };
            }
            return MemoryBrowserAction::Handled;
        }

        if self.search_modal.is_visible() {
            if let Some(result) = self.search_modal.handle_key(key) {
                return match result {
                    Ok(query) => {
                        self.search_query = query;
                        self.page = 0;
                        self.selected = 0;
                        MemoryBrowserAction::SearchSubmitted
                    }
                    Err(ModalCancelled) => {
                        self.search_focused = false;
                        MemoryBrowserAction::SearchCancelled
                    }
                };
            }
            return MemoryBrowserAction::Handled;
        }

        if self.store_modal.is_visible() {
            if let Some(result) = self.store_modal.handle_key(key) {
                return match result {
                    Ok(content) => MemoryBrowserAction::StoreMemory {
                        content,
                        category: self.selected_category,
                    },
                    Err(ModalCancelled) => MemoryBrowserAction::StoreCancelled,
                };
            }
            return MemoryBrowserAction::Handled;
        }

        if self.delete_modal.is_visible() {
            if let Some(result) = self.delete_modal.handle_key(key) {
                return match result {
                    Ok(idx) => {
                        if idx == 0 {
                            MemoryBrowserAction::DeleteConfirmed
                        } else {
                            MemoryBrowserAction::DeleteCancelled
                        }
                    }
                    Err(ModalCancelled) => MemoryBrowserAction::DeleteCancelled,
                };
            }
            return MemoryBrowserAction::Handled;
        }

        // Handle browser key events
        if !self.visible {
            return MemoryBrowserAction::Ignored;
        }

        match key.code {
            // Escape: hide browser
            KeyCode::Esc => {
                self.hide();
                MemoryBrowserAction::Close
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if self.search_focused {
                    MemoryBrowserAction::Handled
                } else {
                    self.move_selection_up();
                    MemoryBrowserAction::Handled
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.search_focused {
                    MemoryBrowserAction::Handled
                } else {
                    self.move_selection_down();
                    MemoryBrowserAction::Handled
                }
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                self.prev_page();
                MemoryBrowserAction::Handled
            }
            KeyCode::PageDown | KeyCode::Char('d') => {
                if self.search_focused {
                    MemoryBrowserAction::Handled
                } else {
                    self.next_page();
                    MemoryBrowserAction::Handled
                }
            }

            // Focus search input
            KeyCode::Char('/') => {
                self.search_focused = true;
                self.search_modal.show();
                MemoryBrowserAction::SearchFocused
            }

            // Open store modal
            KeyCode::Char('n') => {
                self.store_modal.show();
                self.store_modal.clear();
                MemoryBrowserAction::StoreOpened
            }

            // Open category filter
            KeyCode::Char('c') => {
                self.category_modal.show();
                MemoryBrowserAction::CategoryOpened
            }

            // Delete selected memory
            KeyCode::Delete | KeyCode::Backspace => {
                if self.selected_memory().is_some() {
                    self.delete_modal.show();
                    MemoryBrowserAction::DeleteOpened
                } else {
                    MemoryBrowserAction::Handled
                }
            }

            // Toggle search focus
            KeyCode::Tab => {
                self.search_focused = !self.search_focused;
                if self.search_focused {
                    self.search_modal.show();
                }
                MemoryBrowserAction::SearchFocused
            }

            _ => MemoryBrowserAction::Ignored,
        }
    }

    /// Convert category index to MemoryCategory enum
    fn index_to_category(&self, idx: usize) -> Option<MemoryCategory> {
        match idx {
            0 => None, // All Categories
            1 => Some(MemoryCategory::General),
            2 => Some(MemoryCategory::Knowledge),
            3 => Some(MemoryCategory::Preference),
            4 => Some(MemoryCategory::Specification),
            5 => Some(MemoryCategory::Fact),
            6 => Some(MemoryCategory::Pattern),
            7 => Some(MemoryCategory::Decision),
            8 => Some(MemoryCategory::Context),
            9 => Some(MemoryCategory::Temporary),
            10 => Some(MemoryCategory::Observation),
            _ => None,
        }
    }

    /// Render to memory browser overlay
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        // Render active modals first
        if self.category_modal.is_visible() {
            self.category_modal.render(f, area, theme);
            return;
        }

        if self.search_modal.is_visible() {
            self.search_modal.render(f, area, theme);
            return;
        }

        if self.store_modal.is_visible() {
            self.store_modal.render(f, area, theme);
            return;
        }

        if self.delete_modal.is_visible() {
            self.render_delete_modal(f, area, theme);
            return;
        }

        if !self.visible {
            return;
        }

        // Calculate browser area (centered overlay)
        let browser_area = centered_rect(80, 70, area);

        // Clear area
        f.render_widget(Clear, browser_area);

        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_color))
            .title(Span::styled(
                "Memory Browser",
                Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
            ))
            .title_style(Style::default().fg(theme.accent_secondary));

        // Calculate header area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with filters
                Constraint::Min(0), // Memory list
                Constraint::Length(1), // Footer with navigation hints
            ])
            .split(block.inner(browser_area));

        let header_area = chunks[0];
        let list_area = chunks[1];
        let footer_area = chunks[2];

        // Render header with search and category filter
        self.render_header(f, header_area, theme);

        // Render memory list
        self.render_list(f, list_area, theme);

        // Render footer
        self.render_footer(f, footer_area, theme);

        // Draw block around everything
        f.render_widget(block, browser_area);
    }

    fn render_header(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Search info
                Constraint::Percentage(50), // Category info
            ])
            .split(area);

        let search_area = chunks[0];
        let category_area = chunks[1];

        // Search indicator
        let search_text = if self.search_query.is_empty() {
            format!("[/] Search: <all>")
        } else {
            format!("[/] Search: {}", self.search_query)
        };

        let search_style = if self.search_focused {
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_secondary)
        };

        let search_para = Paragraph::new(search_text)
            .style(search_style)
            .alignment(Alignment::Left);

        f.render_widget(search_para, search_area);

        // Category filter indicator
        let category_text = if let Some(cat) = self.category_filter {
            format!("[c] Category: {}", cat)
        } else {
            "[c] Category: All".to_string()
        };

        let category_para = Paragraph::new(category_text)
            .style(Style::default().fg(theme.fg_secondary))
            .alignment(Alignment::Right);

        f.render_widget(category_para, category_area);
    }

    fn render_list(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        let filtered = self.filtered_memories();
        let total_count = filtered.len();

        if total_count == 0 {
            let text = if self.search_query.is_empty() && self.category_filter.is_none() {
                "No memories available"
            } else {
                "No memories match filters"
            };

            let para = Paragraph::new(text)
                .style(Style::default().fg(theme.fg_muted))
                .alignment(Alignment::Center);

            f.render_widget(para, area.centered(|rect| {
                Rect::new(
                    rect.x,
                    rect.y + 1,
                    rect.width.saturating_sub(2),
                    rect.height.saturating_sub(2),
                )
            }));

            return;
        }

        let page_memories = self.current_page_memories();
        let max_page = (total_count / self.page_size).max(1) - 1;
        let page_info = format!("Page {}/{} ({} total)", self.page + 1, max_page + 1, total_count);

        let items: Vec<ListItem> = page_memories
            .iter()
            .enumerate()
            .map(|(idx, memory)| {
                let is_selected = idx == self.selected;
                let category_icon = category_to_icon(memory.category);
                let preview = preview_content(&memory.content, 60);

                let text = format!(
                    "{} {} | {}",
                    category_icon,
                    memory.category,
                    preview
                );

                if is_selected {
                    ListItem::new(text).style(Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD))
                } else {
                    ListItem::new(text)
                }
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border_color)));

        let list_area_inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        f.render_widget(list, list_area_inner[0]);

        // Page info at bottom
        let page_para = Paragraph::new(page_info)
            .style(Style::default().fg(theme.fg_muted))
            .alignment(Alignment::Right);

        f.render_widget(page_para, list_area_inner[1]);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        let hints = vec![
            "[↑/k] Up [↓/j] Down [PgUp/u] Prev Pg [PgDown/d] Next",
            "[/] Focus Search [c] Category Filter [n] New Memory",
            "[Del] Delete [Esc] Close",
        ];

        let hint_lines: Vec<Line> = hints
            .iter()
            .map(|h| Line::from(vec![Span::styled(*h, Style::default().fg(theme.fg_secondary))]))
            .collect();

        let para = Paragraph::new(hint_lines.join("\n"))
            .style(Style::default().fg(theme.fg_secondary))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(para, area);
    }

    fn render_delete_modal(&self, f: &mut Frame, area: Rect, theme: &ConductorTheme) {
        let modal_area = centered_rect(60, 15, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_color))
            .title(Span::styled(
                "Delete Memory",
                Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
            ));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Min(0), // Content
                Constraint::Length(1), // Spacer
            ])
            .split(block.inner(modal_area));

        // Show memory preview if one is selected
        if let Some(memory) = self.selected_memory() {
            let preview = Paragraph::new(format!("Memory:\n{}", memory.content))
                .style(Style::default().fg(theme.fg_secondary))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left);

            f.render_widget(preview, chunks[1]);
        } else {
            let text = Paragraph::new("No memory selected")
                .style(Style::default().fg(theme.fg_muted))
                .alignment(Alignment::Center);

            f.render_widget(text, chunks[1]);
        }

        f.render_widget(block, modal_area);
    }
}

/// Action resulting from key event in to memory browser
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryBrowserAction {
    /// Key was handled, no special action
    Handled,
    /// Key was ignored (browser not visible or not applicable)
    Ignored,
    /// Browser should close
    Close,
    /// Search modal opened/focused
    SearchFocused,
    /// Search submitted
    SearchSubmitted,
    /// Search cancelled
    SearchCancelled,
    /// Category filter opened
    CategoryOpened,
    /// Category selected
    CategorySelected,
    /// Category cancelled
    CategoryCancelled,
    /// Store modal opened
    StoreOpened,
    /// Store to memory with content and category
    StoreMemory { content: String, category: MemoryCategory },
    /// Store cancelled
    StoreCancelled,
    /// Delete confirmation opened
    DeleteOpened,
    /// Delete confirmed
    DeleteConfirmed,
    /// Delete cancelled
    DeleteCancelled,
}

/// Get icon for to memory category
fn category_to_icon(category: MemoryCategory) -> &'static str {
    match category {
        MemoryCategory::General => "📝",
        MemoryCategory::Knowledge => "📚",
        MemoryCategory::Preference => "⚙️",
        MemoryCategory::Specification => "📋",
        MemoryCategory::Fact => "✅",
        MemoryCategory::Pattern => "🔄",
        MemoryCategory::Decision => "💡",
        MemoryCategory::Context => "📍",
        MemoryCategory::Temporary => "⏱️",
        MemoryCategory::Observation => "👁️",
    }
}

/// Preview content to a maximum length
fn preview_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        format!("{}...", &content[..max_len])
    }
}

/// Helper to create centered rect
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
