//! Session-related modal rendering
//!
//! This module provides modals for session management, including the
//! session hub modal and the quick session switcher.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};
use leindex_core::memory::models::McpStatus;
use leindex_core::memory::SessionStatus;

use crate::state::HubFocus;
use crate::theme::Theme;
use crate::app::App;

/// Renders the session hub modal.
///
/// This modal provides advanced session management capabilities including:
/// - Renaming sessions
/// - Managing group assignments
/// - Previewing pane history
/// - Searching within pane content
///
/// The modal has multiple focusable areas controlled by `HubFocus`.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_session_hub_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" SESSION HUB Control ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(10, 10, 15)));
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // RENAME
            Constraint::Length(3), // GROUP
            Constraint::Min(0),    // PREVIEW
            Constraint::Length(3), // SEARCH
        ])
        .split(area);

    // Rename Box
    let rename_style = if app.hub_focus == HubFocus::Rename {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default()
    };
    let rename_title = if app.hub_focus == HubFocus::Rename {
        ">> RENAME (Enter to Commit) "
    } else {
        " RENAME "
    };
    let rename = Paragraph::new(app.rename_buffer.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(rename_title)
            .border_style(rename_style),
    );
    frame.render_widget(rename, chunks[0]);

    // Group Box
    let group_style = if app.hub_focus == HubFocus::Group {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default()
    };
    let group_title = if app.hub_focus == HubFocus::Group {
        ">> GROUP ASSIGNMENT (Enter to change) "
    } else {
        " GROUP ASSIGNMENT "
    };
    let group = Paragraph::new("Current: /default (Press 'm' to Move)").block(
        Block::default()
            .borders(Borders::ALL)
            .title(group_title)
            .border_style(group_style),
    );
    frame.render_widget(group, chunks[1]);

    // Search Results / Pane Preview
    let preview = Paragraph::new(app.session_preview_content.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" PANE HISTORY PREVIEW / SEARCH RESULTS "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, chunks[2]);

    // Search Input
    let search_style = if app.hub_focus == HubFocus::Search {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default()
    };
    let search_title = if app.hub_focus == HubFocus::Search {
        ">> SEARCH IN PANE (Type to filter) "
    } else {
        " SEARCH IN PANE "
    };
    let search_content = if app.hub_focus == HubFocus::Search {
        format!("{}_", app.hub_search_buffer)
    } else {
        app.hub_search_buffer.clone()
    };
    let search_input = Paragraph::new(search_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(search_title)
            .border_style(search_style),
    );
    frame.render_widget(search_input, chunks[3]);
}

/// Renders the quick session switcher modal.
///
/// This modal provides a fast way to switch between active sessions.
/// It displays all sessions with their status, title, and tool.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Mutable reference to the application state (needed for ListState)
pub fn render_switcher_modal(frame: &mut Frame, app: &mut App) {
    let area = crate::modals::centered_rect(50, 40, frame.area());
    let theme = app.theme();
    let block = Block::default()
        .title(" Quick Session Switcher ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    if app.sessions.is_empty() {
        let text = vec![Line::from("  No active sessions.")];
        let para = Paragraph::new(text).block(block);
        frame.render_widget(Clear, area);
        frame.render_widget(para, area);
    } else {
        let items: Vec<ListItem> = app
            .sessions
            .iter()
            .map(|s| {
                let status_color =
                    if s.status == SessionStatus::Running {
                        Color::Green
                    } else {
                        Color::Gray
                    };
                ListItem::new(vec![Line::from(vec![
                    Span::styled(" * ", Style::default().fg(status_color)),
                    Span::styled(&s.title, Style::default().bold().fg(Color::White)),
                    Span::styled(
                        format!(" [{}]", s.tool.as_deref().unwrap_or("?")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])])
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .bold(),
            )
            .highlight_symbol(">> ");

        frame.render_widget(Clear, area);
        frame.render_stateful_widget(list, area, &mut app.switcher_state);
    }
}
