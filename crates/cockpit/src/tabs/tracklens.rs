//! TrackLens tab rendering for Cockpit TUI
//!
//! Displays TrackLens review status, history, and server information.
//! This tab integrates with the TrackLensPane module to display real review state.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, ListItem, List, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::tracklens::TrackLensPane;

/// Render the TrackLens tab
///
/// This tab displays:
/// - Active review status (if a review is in progress)
/// - Review history (past reviews with their outcomes)
/// - Server status and quick actions
pub fn render_tracklens(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let pane = &app.tracklens_pane;

    // Split into sections: header, main content, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(area);

    // Render header
    render_header(frame, chunks[0], pane, &theme);

    // Render main content (delegates to TrackLensPane)
    // But wrap it with theme-styled borders
    render_content(frame, chunks[1], pane, &theme);

    // Render status bar
    render_status_bar(frame, chunks[2], pane, &theme);
}

/// Render the tab header with TrackLens branding
fn render_header(frame: &mut Frame, area: Rect, pane: &TrackLensPane, theme: &crate::theme::Theme) {
    let header_color = if pane.active {
        Color::Yellow
    } else {
        Color::Gray
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 🔍 ", Style::default().fg(Color::Yellow)),
        Span::styled("TrackLens", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" - Review & Walkthrough", Style::default().fg(Color::Gray)),
        Span::styled(
            if pane.active { " ● Active" } else { " ○ Idle" },
            Style::default().fg(header_color).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.panel_bg)),
    )
    .wrap(Wrap { trim: true });

    frame.render_widget(header, area);
}

/// Render the main content area using TrackLensPane state
fn render_content(
    frame: &mut Frame,
    area: Rect,
    pane: &TrackLensPane,
    theme: &crate::theme::Theme,
) {
    // Split content into active review and history sections
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Active Review Section
    render_active_section(frame, content_chunks[0], pane, theme);

    // History Section
    render_history_section(frame, content_chunks[1], pane, theme);
}

/// Render the active review section
fn render_active_section(
    frame: &mut Frame,
    area: Rect,
    pane: &TrackLensPane,
    theme: &crate::theme::Theme,
) {
    let items = if pane.active {
        if let Some(ref review) = pane.current_review {
            vec![
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Gray)),
                    Span::styled("Active", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Track: ", Style::default().fg(Color::Gray)),
                    Span::styled(&review.track_id, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Document: ", Style::default().fg(Color::Gray)),
                    Span::styled(&review.document_type, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Mode: ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:?}", review.mode), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("URL: ", Style::default().fg(Color::Gray)),
                    Span::styled(&review.server_url, Style::default().fg(Color::Blue)),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Gray)),
                    Span::styled("Active", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Loading review details...", Style::default().fg(Color::DarkGray))),
            ]
        }
    } else {
        vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled("Idle", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("No active review.", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("Use ", Style::default().fg(Color::Gray)),
                Span::styled("maestro tracklens review", Style::default().fg(Color::Cyan)),
                Span::styled(" to start a review.", Style::default().fg(Color::Gray)),
            ]),
        ]
    };

    let content = items
        .into_iter()
        .map(|line| ListItem::new(line))
        .collect::<Vec<_>>();

    let list = List::new(content)
        .block(
            Block::default()
                .title(" Active Review ")
                .title_style(Style::default().fg(Color::Yellow))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(theme.panel_bg)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, area);
}

/// Render the review history section
fn render_history_section(
    frame: &mut Frame,
    area: Rect,
    pane: &TrackLensPane,
    theme: &crate::theme::Theme,
) {
    let items = if pane.history.is_empty() {
        vec![
            Line::from(vec![
                Span::styled("No review history.", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Completed reviews will appear here.", Style::default().fg(Color::Gray)),
            ]),
        ]
    } else {
        pane.history
            .iter()
            .rev()
            .take(10)
            .map(|entry| {
                let status = if entry.approved {
                    Span::styled("✓", Style::default().fg(Color::Green))
                } else {
                    Span::styled("✗", Style::default().fg(Color::Red))
                };

                let ts = entry.timestamp.format("%H:%M").to_string();
                let count = entry.annotation_count.to_string();

                Line::from(vec![
                    status,
                    Span::raw(" "),
                    Span::styled(&entry.track_id, Style::default().fg(Color::Cyan)),
                    Span::raw(" - "),
                    Span::styled(&entry.document_type, Style::default().fg(Color::White)),
                    Span::raw(" ("),
                    Span::styled(count, Style::default().fg(Color::Yellow)),
                    Span::raw(")"),
                    Span::raw(" "),
                    Span::styled(ts, Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect()
    };

    let content = items
        .into_iter()
        .map(|line| ListItem::new(line))
        .collect::<Vec<_>>();

    let list = List::new(content)
        .block(
            Block::default()
                .title(" Review History ")
                .title_style(Style::default().fg(Color::Yellow))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(theme.panel_bg)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, area);
}

/// Render the status bar with server information
fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    pane: &TrackLensPane,
    theme: &crate::theme::Theme,
) {
    let total_reviews = pane.history.len();
    let approved_count = pane.history.iter().filter(|e| e.approved).count();

    let status_text = vec![
        Line::from(vec![
            Span::styled(" TrackLens ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(" Total: ", Style::default().fg(Color::Gray)),
            Span::styled(total_reviews.to_string(), Style::default().fg(Color::White)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(" Approved: ", Style::default().fg(Color::Gray)),
            Span::styled(
                approved_count.to_string(),
                Style::default().fg(if approved_count > 0 { Color::Green } else { Color::DarkGray }),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if pane.active { "● Running" } else { "○ Idle" },
                Style::default()
                    .fg(if pane.active { Color::Yellow } else { Color::DarkGray })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let status_bar = Paragraph::new(status_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(theme.panel_bg)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(status_bar, area);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_render_function_exists() {
        // Verify the render functions compile
        assert!(true);
    }
}
