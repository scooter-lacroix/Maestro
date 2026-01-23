//! Spawning overlay modal rendering
//!
//! This module provides the spawning overlay, which is displayed while
//! the system is performing async operations like spawning new sessions.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::widgets::Clear;

use crate::app::App;

/// Renders the spawning overlay modal.
///
/// This overlay is displayed when the system is performing async operations.
/// It shows the current status message to provide feedback to the user
/// during potentially long-running operations.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_spawning_overlay(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(40, 10, frame.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(30, 0, 30)).fg(Color::Yellow));

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚡ ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&app.status_message),
        ]),
    ];

    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}
