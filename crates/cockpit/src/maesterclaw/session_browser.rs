//! Session browser component for MaestroClaw.
//!
//! Provides a filtered, navigable list of sessions with preview information.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// A session entry displayed in the browser.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub source: String,
    pub last_active: String,
    pub turn_count: usize,
}

/// Session browser with filtering and navigation.
#[derive(Debug, Clone)]
pub struct SessionBrowser {
    sessions: Vec<SessionEntry>,
    filter_text: String,
    filtered_indices: Vec<usize>,
    cursor: usize,
    scroll_offset: usize,
    active: bool,
}

impl Default for SessionBrowser {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            filter_text: String::new(),
            filtered_indices: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            active: false,
        }
    }
}

impl SessionBrowser {
    /// Creates a new session browser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads sessions into the browser, resetting the filter and cursor.
    pub fn load(&mut self, sessions: Vec<SessionEntry>) {
        self.sessions = sessions;
        self.filter_text.clear();
        self.cursor = 0;
        self.scroll_offset = 0;
        self.refilter();
    }

    /// Rebuilds the filtered indices based on current filter text.
    fn refilter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_indices = (0..self.sessions.len()).collect();
        } else {
            let filter_lower = self.filter_text.to_lowercase();
            self.filtered_indices = self
                .sessions
                .iter()
                .enumerate()
                .filter(|(_, session)| {
                    session.title.to_lowercase().contains(&filter_lower)
                        || session.preview.to_lowercase().contains(&filter_lower)
                        || session.id.to_lowercase().contains(&filter_lower)
                        || session.source.to_lowercase().contains(&filter_lower)
                })
                .map(|(i, _)| i)
                .collect();
        }
        // Clamp cursor to valid range
        if !self.filtered_indices.is_empty() {
            self.cursor = self.cursor.min(self.filtered_indices.len() - 1);
        } else {
            self.cursor = 0;
        }
    }

    /// Handles a character input, appending to the filter.
    pub fn on_char(&mut self, c: char) {
        self.filter_text.push(c);
        self.refilter();
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    /// Handles backspace, removing the last character from the filter.
    pub fn on_backspace(&mut self) {
        self.filter_text.pop();
        self.refilter();
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    /// Moves the cursor up in the filtered list.
    pub fn move_up(&mut self) {
        if !self.filtered_indices.is_empty() && self.cursor > 0 {
            self.cursor -= 1;
            // Adjust scroll_offset if cursor moved above visible area
            if self.cursor < self.scroll_offset {
                self.scroll_offset = self.cursor;
            }
        }
    }

    /// Moves the cursor down in the filtered list.
    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.cursor = self.cursor.saturating_add(1).min(self.filtered_indices.len() - 1);
            // Adjust scroll_offset if cursor moved below visible area
            // (This will be handled in render based on available height)
        }
    }

    /// Returns the ID of the selected session, if any.
    pub fn selected_session_id(&self) -> Option<&str> {
        self.filtered_indices
            .get(self.cursor)
            .and_then(|&idx| self.sessions.get(idx))
            .map(|session| session.id.as_str())
    }

    /// Sets whether the browser is active.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Returns whether the browser is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the current filter text (for testing filter reachability).
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    /// Renders the session browser to the frame.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Layout: Header (3 lines), Session list (Min 3), Footer (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(3),    // Session list
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header section
        self.render_header(frame, chunks[0]);

        // Render session list
        self.render_session_list(frame, chunks[1]);

        // Render footer
        self.render_footer(frame, chunks[2]);
    }

    /// Renders the header section with filter input or title.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let header_text = if self.filter_text.is_empty() {
            // Show title with complete key hints when filter is empty
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "  Browse Sessions",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " — ↑↓ navigate  Enter select  Type to filter  Esc quit",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ]
        } else {
            // Show filter when active
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("  Filter: {}█", self.filter_text),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]
        };

        let paragraph = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(paragraph, area);
    }

    /// Renders the session list with selection indicators.
    fn render_session_list(&self, frame: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }

        if self.filtered_indices.is_empty() {
            let no_sessions = Paragraph::new(vec![Line::from(vec![
                Span::styled("No sessions match the filter.", Style::default().fg(Color::DarkGray)),
            ])]);
            frame.render_widget(no_sessions, area);
            return;
        }

        // Calculate visible range based on scroll_offset and available height
        let visible_height = area.height as usize;
        let max_visible = visible_height.max(1);

        // Update scroll_offset if cursor is outside visible range
        let mut scroll_offset = self.scroll_offset;
        if self.cursor >= scroll_offset + max_visible {
            scroll_offset = self.cursor.saturating_sub(max_visible - 1);
        } else if self.cursor < scroll_offset {
            scroll_offset = self.cursor;
        }

        let visible_indices: Vec<_> = self.filtered_indices
            .iter()
            .skip(scroll_offset)
            .take(max_visible)
            .collect();

        let items: Vec<ListItem> = visible_indices
            .iter()
            .enumerate()
            .map(|(relative_pos, &&idx)| {
                let session = &self.sessions[idx];
                let absolute_pos = scroll_offset + relative_pos;
                let is_selected = absolute_pos == self.cursor;

                let indicator = if is_selected { " → " } else { "   " };

                // Title fallback chain: title → preview → id
                let display_title = if !session.title.is_empty() {
                    &session.title
                } else if !session.preview.is_empty() {
                    &session.preview
                } else {
                    &session.id
                };
                let title = truncate(display_title, 40);
                let last_active = truncate(&session.last_active, 10);
                let source = truncate(&session.source, 6);
                let id_display = truncate(&session.id, 18);

                let style = if is_selected {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(indicator, style),
                    Span::styled(title, style),
                    Span::raw("  "),
                    Span::styled(last_active, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(source, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(id_display, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, area);
    }

    /// Renders the footer with session count information.
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let total = self.sessions.len();
        let filtered = self.filtered_indices.len();
        let pos = if filtered > 0 {
            self.cursor + 1
        } else {
            0
        };
        let footer_text = if filtered < total {
            format!(
                "  {}/{} sessions (filtered from {})",
                pos, filtered, total
            )
        } else {
            format!("  {}/{} sessions", pos, filtered)
        };

        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled(footer_text, Style::default().fg(Color::DarkGray)),
        ]));

        frame.render_widget(paragraph, area);
    }
}

/// Truncates a string to the specified maximum length, adding ellipsis if needed.
/// Uses char-based slicing to safely handle multi-byte UTF-8 characters.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_session(id: &str, title: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            title: title.to_string(),
            preview: "test preview".to_string(),
            source: "test".to_string(),
            last_active: "2m ago".to_string(),
            turn_count: 5,
        }
    }

    #[test]
    fn test_default_creates_empty_browser() {
        let browser = SessionBrowser::default();
        assert!(browser.sessions.is_empty());
        assert!(browser.filter_text.is_empty());
        assert_eq!(browser.cursor, 0);
        assert!(browser.filtered_indices.is_empty());
    }

    #[test]
    fn test_load_populates_sessions() {
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("1", "First Session"),
            make_test_session("2", "Second Session"),
        ];
        browser.load(sessions);
        assert_eq!(browser.sessions.len(), 2);
        assert_eq!(browser.filtered_indices.len(), 2);
        assert_eq!(browser.cursor, 0);
    }

    #[test]
    fn test_filter_filters_sessions() {
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("1", "Apple Session"),
            make_test_session("2", "Banana Session"),
            make_test_session("3", "Apricot Session"),
        ];
        browser.load(sessions);
        browser.on_char('a');
        browser.on_char('p');
        assert_eq!(browser.filtered_indices.len(), 2); // Apple, Apricot
    }

    #[test]
    fn test_backspace_removes_filter_char() {
        let mut browser = SessionBrowser::new();
        let sessions = vec![make_test_session("1", "Test Session")];
        browser.load(sessions);
        browser.on_char('t');
        browser.on_char('e');
        assert_eq!(browser.filter_text, "te");
        browser.on_backspace();
        assert_eq!(browser.filter_text, "t");
    }

    #[test]
    fn test_navigation() {
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("1", "First"),
            make_test_session("2", "Second"),
            make_test_session("3", "Third"),
        ];
        browser.load(sessions);

        // Move down
        browser.move_down();
        assert_eq!(browser.cursor, 1);
        assert_eq!(browser.selected_session_id(), Some("2"));

        // Move down again
        browser.move_down();
        assert_eq!(browser.cursor, 2);

        // Can't move past end
        browser.move_down();
        assert_eq!(browser.cursor, 2);

        // Move up
        browser.move_up();
        assert_eq!(browser.cursor, 1);

        // Can't move before start
        browser.cursor = 0;
        browser.move_up();
        assert_eq!(browser.cursor, 0);
    }

    #[test]
    fn test_selected_session_id() {
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("id-1", "First"),
            make_test_session("id-2", "Second"),
        ];
        browser.load(sessions);
        assert_eq!(browser.selected_session_id(), Some("id-1"));

        browser.move_down();
        assert_eq!(browser.selected_session_id(), Some("id-2"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("a", 1), "a");
        assert_eq!(truncate("ab", 1), "...");
    }

    #[test]
    fn test_filter_resets_cursor() {
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("1", "Alpha"),
            make_test_session("2", "Beta"),
            make_test_session("3", "Gamma"),
        ];
        browser.load(sessions.clone());
        browser.move_down();
        browser.move_down();
        assert_eq!(browser.cursor, 2);

        // Typing in filter should reset cursor
        browser.on_char('a');
        assert_eq!(browser.cursor, 0);
        assert_eq!(browser.scroll_offset, 0);
    }

    #[test]
    fn test_empty_selection() {
        let browser = SessionBrowser::new();
        assert_eq!(browser.selected_session_id(), None);
    }

    #[test]
    fn test_title_fallback_to_preview() {
        // Session with empty title should be findable via preview text
        let mut browser = SessionBrowser::new();
        let sessions = vec![SessionEntry {
            id: "session-1".to_string(),
            title: String::new(),
            preview: "working on authentication".to_string(),
            source: "claude".to_string(),
            last_active: "5m ago".to_string(),
            turn_count: 3,
        }];
        browser.load(sessions);

        // Filter by preview content should still find the session
        browser.on_char('a');
        browser.on_char('u');
        browser.on_char('t');
        browser.on_char('h');
        assert_eq!(browser.filtered_indices.len(), 1);
        assert_eq!(browser.selected_session_id(), Some("session-1"));
    }

    #[test]
    fn test_title_fallback_to_id() {
        // Session with empty title and preview should be findable via id
        let mut browser = SessionBrowser::new();
        let sessions = vec![SessionEntry {
            id: "abc-123".to_string(),
            title: String::new(),
            preview: String::new(),
            source: "claude".to_string(),
            last_active: "1m ago".to_string(),
            turn_count: 1,
        }];
        browser.load(sessions);

        // Filter by id content should find the session
        browser.on_char('a');
        browser.on_char('b');
        browser.on_char('c');
        assert_eq!(browser.filtered_indices.len(), 1);
        assert_eq!(browser.selected_session_id(), Some("abc-123"));
    }

    #[test]
    fn test_filter_by_source() {
        // Session should be findable via source field
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("1", "Fix Bug"),
            make_test_session("2", "Add Feature"),
        ];
        browser.load(sessions);

        // Filter by source "test" should find both
        browser.on_char('t');
        browser.on_char('e');
        browser.on_char('s');
        browser.on_char('t');
        assert_eq!(browser.filtered_indices.len(), 2);
    }

    /// Helper: extract text content of a single line from the render buffer.
    fn buffer_line(backend: &ratatui::backend::TestBackend, y: u16) -> String {
        let buf = backend.buffer();
        let area = buf.area();
        let mut s = String::new();
        for x in area.left()..area.right() {
            s.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
        }
        s.trim_end().to_string()
    }

    #[test]
    fn test_render_footer_shows_zero_match_counts() {
        // When a filter produces zero results, the rendered footer must show
        // "0/0 sessions (filtered from N)" — not just "No sessions".
        let mut browser = SessionBrowser::new();
        browser.load(vec![
            make_test_session("1", "Alpha"),
            make_test_session("2", "Beta"),
            make_test_session("3", "Gamma"),
        ]);
        browser.on_char('z');
        browser.on_char('z');
        browser.on_char('z');
        assert!(browser.filtered_indices.is_empty());

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                browser.render(f, f.area());
            })
            .unwrap();

        // Footer is the last line (y=9 for height 10)
        let footer_text = buffer_line(terminal.backend(), 9);
        assert_eq!(
            footer_text,
            "  0/0 sessions (filtered from 3)",
            "footer should show zero-match counts"
        );
    }

    #[test]
    fn test_render_footer_shows_unfiltered_counts() {
        let mut browser = SessionBrowser::new();
        browser.load(vec![
            make_test_session("1", "One"),
            make_test_session("2", "Two"),
            make_test_session("3", "Three"),
        ]);

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                browser.render(f, f.area());
            })
            .unwrap();

        let footer_text = buffer_line(terminal.backend(), 9);
        assert_eq!(
            footer_text,
            "  1/3 sessions",
            "footer should show unfiltered counts"
        );
    }

    #[test]
    fn test_render_footer_shows_filtered_with_matches() {
        // 3 sessions, filter to 2 matches → footer: "1/2 sessions (filtered from 3)"
        let mut browser = SessionBrowser::new();
        browser.load(vec![
            make_test_session("1", "Alpha Session"),
            make_test_session("2", "Beta Session"),
            make_test_session("3", "Gamma Session"),
        ]);
        browser.on_char('s');
        browser.on_char('e');
        browser.on_char('s');
        // "ses" matches "Alpha Session" and "Beta Session" and "Gamma Session"
        // (all titles contain "Session" which starts with "ses")
        assert_eq!(browser.filtered_indices.len(), 3);
        // Narrow to just "beta": filter "bet" → only "Beta Session"
        browser.filter_text.clear();
        browser.refilter();
        browser.cursor = 0;
        browser.scroll_offset = 0;
        browser.on_char('b');
        browser.on_char('e');
        browser.on_char('t');
        assert_eq!(browser.filtered_indices.len(), 1);

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                browser.render(f, f.area());
            })
            .unwrap();

        let footer_text = buffer_line(terminal.backend(), 9);
        assert_eq!(
            footer_text,
            "  1/1 sessions (filtered from 3)",
            "footer should show filtered match counts"
        );
    }

    #[test]
    fn test_render_all_visible_rows_used() {
        // With 5 sessions in a 9-row area (3 header + 5 list + 1 footer),
        // all 5 sessions should occupy the list rows — no row silently dropped.
        let mut browser = SessionBrowser::new();
        let sessions: Vec<SessionEntry> = (0..5)
            .map(|i| SessionEntry {
                id: format!("s-{}", i),
                title: format!("Session {}", i),
                preview: "preview".to_string(),
                source: "test".to_string(),
                last_active: "1m ago".to_string(),
                turn_count: 1,
            })
            .collect();
        browser.load(sessions);

        let backend = ratatui::backend::TestBackend::new(80, 9);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                browser.render(f, f.area());
            })
            .unwrap();

        // List area: y=3..=7 (5 rows). Last session should be at y=7.
        let last_list_line = buffer_line(terminal.backend(), 7);
        assert!(
            last_list_line.contains("Session 4"),
            "5th session should be visible in last list row, got: '{}'",
            last_list_line
        );
    }

    #[test]
    fn test_render_partial_fit_no_wasted_rows() {
        // With 1 session in a 10-row area (3 header + 6 list + 1 footer),
        // only y=3 should have session content; y=4 must be empty.
        let mut browser = SessionBrowser::new();
        browser.load(vec![make_test_session("1", "Only")]);

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                browser.render(f, f.area());
            })
            .unwrap();

        let first_list_line = buffer_line(terminal.backend(), 3);
        assert!(
            first_list_line.contains("Only"),
            "session should be in first list row, got: '{}'",
            first_list_line
        );

        let second_list_line = buffer_line(terminal.backend(), 4);
        assert!(
            second_list_line.is_empty(),
            "second list row should be empty with only 1 session, got: '{}'",
            second_list_line
        );
    }

    #[test]
    fn test_enter_on_session_returns_id() {
        // Integration path: selecting a session and immediately getting its ID.
        // This verifies the data flow that the app.rs handler relies on.
        let mut browser = SessionBrowser::new();
        let sessions = vec![
            make_test_session("sess-aaa", "First"),
            make_test_session("sess-bbb", "Second"),
            make_test_session("sess-ccc", "Third"),
        ];
        browser.load(sessions);

        // Navigate to second session
        browser.move_down();
        assert_eq!(browser.selected_session_id(), Some("sess-bbb"));

        // Navigate to third
        browser.move_down();
        assert_eq!(browser.selected_session_id(), Some("sess-ccc"));
    }

    #[test]
    fn test_enter_returns_none_on_empty_browser() {
        let browser = SessionBrowser::new();
        assert_eq!(browser.selected_session_id(), None);
    }
}
