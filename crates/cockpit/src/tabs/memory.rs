//! Memory tab rendering for Cockpit TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use crate::state::InputMode;

pub fn render_memory(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🔎 Memory Search (Ctrl+F, Ctrl+L clear, r refresh) ")
        .title_style(Style::default().fg(theme.accent));

    let search_text = if app.input_mode == InputMode::MemorySearch {
        format!("{}█", app.memory_query)
    } else {
        app.memory_query.clone()
    };
    frame.render_widget(Paragraph::new(search_text).block(search_block), chunks[0]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🧠 Memory Results ")
        .title_style(Style::default().fg(theme.accent_alt));

    if app.memories.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No memories found."),
            Line::from(""),
            Line::from("  Tip: press 'r' to import system-wide memories."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = app
        .memories
        .iter()
        .map(|m| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", m.category),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(m.content.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, chunks[1], &mut app.memory_state);
}
