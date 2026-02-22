//! Command Palette for Maestro Cockpit
//!
//! Provides hub-and-spoke navigation with fuzzy search for quick access to
//! all tabs, capabilities, and actions.

mod search;

pub use search::{fuzzy_match, search_items};

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Command palette entry
#[derive(Clone, Debug)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub shortcut: String,
    pub category: CommandCategory,
    pub description: String,
}

/// Command categories for grouping
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandCategory {
    Recent,
    Tabs,
    Capabilities,
    Analysis,
    Actions,
}

impl CommandCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Tabs => "Tabs",
            Self::Capabilities => "Capabilities",
            Self::Analysis => "Analysis",
            Self::Actions => "Actions",
        }
    }
}

/// Command palette state
#[derive(Clone, Debug)]
pub struct CommandPalette {
    /// Whether the palette is visible
    pub is_visible: bool,
    /// Search query
    pub query: String,
    /// All available commands
    pub commands: Vec<Command>,
    /// Filtered commands based on query
    pub filtered: Vec<Command>,
    /// List state for selection
    pub list_state: ListState,
    /// Recently used commands (MRU)
    pub recent: Vec<String>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new() -> Self {
        let commands = Self::build_commands();
        let filtered = commands.clone();

        Self {
            is_visible: false,
            query: String::new(),
            commands,
            filtered,
            list_state: ListState::default(),
            recent: Vec::new(),
        }
    }

    /// Build the list of available commands
    fn build_commands() -> Vec<Command> {
        vec![
            // Tabs
            Command {
                id: "tab.hub".into(),
                label: "Hub (Dashboard)".into(),
                shortcut: "Alt+1".into(),
                category: CommandCategory::Tabs,
                description: "Go to Hub dashboard".into(),
            },
            Command {
                id: "tab.sessions".into(),
                label: "Sessions".into(),
                shortcut: "Alt+2".into(),
                category: CommandCategory::Tabs,
                description: "Manage tmux sessions".into(),
            },
            Command {
                id: "tab.conductor".into(),
                label: "Conductor".into(),
                shortcut: "Alt+3".into(),
                category: CommandCategory::Tabs,
                description: "Track/Task execution".into(),
            },
            Command {
                id: "tab.capabilities".into(),
                label: "Capabilities".into(),
                shortcut: "Alt+4".into(),
                category: CommandCategory::Tabs,
                description: "Phase 3-5 capabilities".into(),
            },
            Command {
                id: "tab.settings".into(),
                label: "Settings".into(),
                shortcut: "Alt+5".into(),
                category: CommandCategory::Tabs,
                description: "Configuration and preferences".into(),
            },
            // Capabilities
            Command {
                id: "cap.cron".into(),
                label: "Cron Jobs".into(),
                shortcut: "c1".into(),
                category: CommandCategory::Capabilities,
                description: "Scheduled jobs management".into(),
            },
            Command {
                id: "cap.mcp".into(),
                label: "MCP Servers".into(),
                shortcut: "c2".into(),
                category: CommandCategory::Capabilities,
                description: "MCP provider management".into(),
            },
            Command {
                id: "cap.sandbox".into(),
                label: "Sandbox".into(),
                shortcut: "c3".into(),
                category: CommandCategory::Capabilities,
                description: "Security policy and runtimes".into(),
            },
            Command {
                id: "cap.channels".into(),
                label: "Channels".into(),
                shortcut: "c4".into(),
                category: CommandCategory::Capabilities,
                description: "Telegram, Discord, Slack".into(),
            },
            Command {
                id: "cap.gateway".into(),
                label: "Web Gateway".into(),
                shortcut: "c5".into(),
                category: CommandCategory::Capabilities,
                description: "HTTP/SSE/WebSocket server".into(),
            },
            // Analysis
            Command {
                id: "analysis.leindex".into(),
                label: "LeIndex Search".into(),
                shortcut: "a1".into(),
                category: CommandCategory::Analysis,
                description: "Search code with LeIndex".into(),
            },
            Command {
                id: "analysis.tldr".into(),
                label: "TLDR Analysis".into(),
                shortcut: "a2".into(),
                category: CommandCategory::Analysis,
                description: "5-layer code analysis".into(),
            },
            // Actions
            Command {
                id: "action.new_session".into(),
                label: "New Session".into(),
                shortcut: "n".into(),
                category: CommandCategory::Actions,
                description: "Create a new session".into(),
            },
            Command {
                id: "action.search".into(),
                label: "Quick Search".into(),
                shortcut: "/".into(),
                category: CommandCategory::Actions,
                description: "Context-aware search".into(),
            },
            Command {
                id: "action.help".into(),
                label: "Help".into(),
                shortcut: "?".into(),
                category: CommandCategory::Actions,
                description: "Show help and keybindings".into(),
            },
            Command {
                id: "action.quit".into(),
                label: "Quit".into(),
                shortcut: "q".into(),
                category: CommandCategory::Actions,
                description: "Exit Maestro Cockpit".into(),
            },
        ]
    }

    /// Show the command palette
    pub fn show(&mut self) {
        self.is_visible = true;
        self.query.clear();
        self.update_filtered();
        self.list_state.select(Some(0));
    }

    /// Hide the command palette
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Toggle the command palette
    pub fn toggle(&mut self) {
        if self.is_visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Handle character input for search
    pub fn handle_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filtered();
    }

    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        self.query.pop();
        self.update_filtered();
    }

    /// Update filtered commands based on query
    fn update_filtered(&mut self) {
        if self.query.is_empty() {
            // Show recent commands first, then all others
            let mut recent_cmds: Vec<Command> = Vec::new();
            let mut other_cmds: Vec<Command> = Vec::new();

            for cmd in &self.commands {
                if self.recent.contains(&cmd.id) {
                    recent_cmds.push(cmd.clone());
                } else {
                    other_cmds.push(cmd.clone());
                }
            }

            self.filtered = [recent_cmds, other_cmds].concat();
        } else {
            // Fuzzy search with ranking using search_items
            let query = self.query.to_lowercase();

            // Search by label (primary) and description (secondary)
            let label_results = search_items(&self.commands, &query, |cmd| &cmd.label);
            let desc_results = search_items(&self.commands, &query, |cmd| &cmd.description);

            // Combine results, prioritizing label matches
            let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut ranked: Vec<Command> = Vec::new();

            // Add label matches first (higher priority)
            for (idx, _) in label_results {
                let cmd = &self.commands[idx];
                if seen_ids.insert(cmd.id.clone()) {
                    ranked.push(cmd.clone());
                }
            }

            // Add description matches (lower priority)
            for (idx, _) in desc_results {
                let cmd = &self.commands[idx];
                if seen_ids.insert(cmd.id.clone()) {
                    ranked.push(cmd.clone());
                }
            }

            // Also include substring matches for shortcuts
            for cmd in &self.commands {
                if cmd.shortcut.to_lowercase().contains(&query) && seen_ids.insert(cmd.id.clone()) {
                    ranked.push(cmd.clone());
                }
            }

            self.filtered = ranked;
        }

        // Reset selection
        if !self.filtered.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let current = self.list_state.selected().unwrap_or(0);
        if current > 0 {
            self.list_state.select(Some(current - 1));
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let current = self.list_state.selected().unwrap_or(0);
        if current < self.filtered.len().saturating_sub(1) {
            self.list_state.select(Some(current + 1));
        }
    }

    /// Get selected command
    pub fn selected(&self) -> Option<&Command> {
        let idx = self.list_state.selected()?;
        self.filtered.get(idx)
    }

    /// Execute selected command (returns command ID)
    pub fn execute(&mut self) -> Option<String> {
        let cmd = self.selected()?.clone();
        // Add to recent
        if !self.recent.contains(&cmd.id) {
            self.recent.insert(0, cmd.id.clone());
            // Keep only last 5 recent
            if self.recent.len() > 5 {
                self.recent.truncate(5);
            }
        }
        self.hide();
        Some(cmd.id)
    }

    /// Render the command palette
    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        if !self.is_visible {
            return;
        }

        let area = frame.area();
        let popup_area = centered_rect(50, 60, area);

        // Clear the area
        frame.render_widget(Clear, popup_area);

        // Create block
        let block = Block::default()
            .title(" Command Palette ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Layout: search input, command list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .split(inner);

        // Render search input
        let input = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("> ", Style::default().fg(theme.accent)),
                Span::styled(&self.query, Style::default().fg(theme.fg)),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.muted)),
        );
        frame.render_widget(input, chunks[0]);

        // Render command list grouped by category
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let is_selected = self.list_state.selected() == Some(i);
                let style = if is_selected {
                    Style::default().fg(theme.bg).bg(theme.accent)
                } else {
                    Style::default().fg(theme.fg)
                };

                let shortcut_style = if is_selected {
                    Style::default()
                        .fg(theme.bg)
                        .bg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.muted)
                };

                let line = Line::from(vec![
                    Span::styled(format!("{:10} ", cmd.shortcut), shortcut_style),
                    Span::styled(&cmd.label, style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).highlight_style(Style::default().bg(theme.accent).fg(theme.bg));
        frame.render_stateful_widget(list, chunks[1], &mut self.list_state.clone());
    }
}

/// Create a centered rectangle
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_palette_default() {
        let palette = CommandPalette::default();
        assert!(!palette.is_visible());
        assert!(palette.query.is_empty());
        assert!(!palette.commands.is_empty());
    }

    #[test]
    fn test_command_palette_show_hide() {
        let mut palette = CommandPalette::new();
        palette.show();
        assert!(palette.is_visible());

        palette.hide();
        assert!(!palette.is_visible());
    }

    #[test]
    fn test_command_palette_toggle() {
        let mut palette = CommandPalette::new();
        palette.toggle();
        assert!(palette.is_visible());

        palette.toggle();
        assert!(!palette.is_visible());
    }

    #[test]
    fn test_command_palette_search() {
        let mut palette = CommandPalette::new();
        palette.show();

        palette.handle_char('c');
        palette.handle_char('r');
        palette.handle_char('o');
        palette.handle_char('n');

        assert_eq!(palette.query, "cron");
        assert!(!palette.filtered.is_empty());

        // Should find "Cron Jobs" command
        let has_cron = palette.filtered.iter().any(|c| c.id == "cap.cron");
        assert!(has_cron);
    }

    #[test]
    fn test_command_palette_navigation() {
        let mut palette = CommandPalette::new();
        palette.show();

        assert_eq!(palette.list_state.selected(), Some(0));

        palette.move_down();
        assert_eq!(palette.list_state.selected(), Some(1));

        palette.move_up();
        assert_eq!(palette.list_state.selected(), Some(0));
    }

    #[test]
    fn test_command_palette_backspace() {
        let mut palette = CommandPalette::new();
        palette.show();

        palette.handle_char('t');
        palette.handle_char('e');
        palette.handle_char('s');
        palette.handle_char('t');
        assert_eq!(palette.query, "test");

        palette.handle_backspace();
        assert_eq!(palette.query, "tes");
    }

    #[test]
    fn test_command_palette_execute() {
        let mut palette = CommandPalette::new();
        palette.show();
        palette.list_state.select(Some(0));

        let cmd_id = palette.execute();
        assert!(cmd_id.is_some());
        assert!(!palette.is_visible());
        assert!(!palette.recent.is_empty());
    }

    #[test]
    fn test_command_palette_recent() {
        let mut palette = CommandPalette::new();
        palette.show();
        palette.list_state.select(Some(0));
        palette.execute();
        palette.show();

        // Recent command should be first
        assert!(!palette.filtered.is_empty());
    }

    #[test]
    fn test_command_categories() {
        let palette = CommandPalette::new();

        let tabs: Vec<_> = palette
            .commands
            .iter()
            .filter(|c| c.category == CommandCategory::Tabs)
            .collect();
        assert!(!tabs.is_empty());

        let caps: Vec<_> = palette
            .commands
            .iter()
            .filter(|c| c.category == CommandCategory::Capabilities)
            .collect();
        assert!(!caps.is_empty());
    }

    #[test]
    fn test_command_shortcuts() {
        let palette = CommandPalette::new();

        // Check capability shortcuts
        let cron = palette.commands.iter().find(|c| c.id == "cap.cron");
        assert!(cron.is_some());
        assert_eq!(cron.unwrap().shortcut, "c1");

        let mcp = palette.commands.iter().find(|c| c.id == "cap.mcp");
        assert!(mcp.is_some());
        assert_eq!(mcp.unwrap().shortcut, "c2");
    }

    #[test]
    fn test_selected() {
        let mut palette = CommandPalette::new();
        palette.show();

        let selected = palette.selected();
        assert!(selected.is_some());

        palette.list_state.select(None);
        let selected = palette.selected();
        assert!(selected.is_none());
    }
}
