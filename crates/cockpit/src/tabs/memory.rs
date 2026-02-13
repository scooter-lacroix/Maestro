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

    // Check if we're in memory creation mode
    let is_creating = app.input_mode == InputMode::NewMemoryContent
        || app.input_mode == InputMode::NewMemoryCategory;

    let (search_area, content_area, input_area) = if is_creating {
        // Split: search (1 line), content input (3 lines), memories (rest)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(area);
        (chunks[0], None, Some(chunks[1]))
    } else {
        // Original layout: search (3 lines), memories (rest)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);
        (chunks[0], Some(chunks[1]), None)
    };

    // Render search bar
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🔎 Memory Search (Ctrl+F clear, r refresh, n new) ")
        .title_style(Style::default().fg(theme.accent));

    let search_text = if app.input_mode == InputMode::MemorySearch {
        format!("{}█", app.memory_query)
    } else {
        app.memory_query.clone()
    };
    frame.render_widget(Paragraph::new(search_text).block(search_block), search_area);

    // Render memory creation input if active
    if let Some(input_area) = input_area {
        let input_title = if app.input_mode == InputMode::NewMemoryContent {
            " 📝 New Memory Content (Enter to continue, Esc to cancel) "
        } else {
            " 🏷️  Category (general, knowledge, preference, spec, fact, pattern, decision, context, temp, observation) "
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(input_title)
            .title_style(Style::default().fg(theme.accent_alt))
            .border_style(Style::default().fg(theme.accent));

        let input_text = if app.input_mode == InputMode::NewMemoryContent {
            format!("{}█", app.new_memory_content)
        } else {
            format!("{}█", app.new_memory_category)
        };

        let input_paragraph = Paragraph::new(input_text)
            .block(input_block)
            .style(Style::default().fg(Color::White));
        frame.render_widget(input_paragraph, input_area);
    }

    // Render memories list
    if let Some(list_area) = content_area {
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
                Line::from("  Tip: press 'n' to create a new memory."),
            ];
            let para = Paragraph::new(text).block(list_block);
            frame.render_widget(para, list_area);
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
        frame.render_stateful_widget(list, list_area, &mut app.memory_state);
    }
}
