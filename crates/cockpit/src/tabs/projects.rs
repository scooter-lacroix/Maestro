//! Projects tab rendering for Cockpit TUI

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, List, ListItem, Paragraph},
    Frame,
    prelude::*,
};

use crate::app::App;

pub fn render_projects(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🚀 Projects ")
        .title_style(Style::default().fg(Color::Cyan));

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    if app.projects.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No projects indexed."),
            Line::from(""),
            Line::from("  Run \"maestro scan\" to find projects."),
        ];
        let para = Paragraph::new(text).block(block);
        frame.render_widget(para, area);
    } else {
        // Project List (Left)
        let items: Vec<ListItem> = app
            .projects
            .iter()
            .map(|p| {
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {} ", "📦"), Style::default()),
                        Span::styled(&p.name, Style::default().fg(Color::Cyan).bold()),
                    ]),
                    Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(&p.path, Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(""),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(30, 30, 50))
                    .fg(Color::Yellow)
                    .bold(),
            )
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, chunks[0], &mut app.project_state);

        // File Preview / "Yazi" Column (Right)
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(if app.preview_focused {
                " 📂 File Explorer (Focused) "
            } else {
                " 📂 File Explorer "
            })
            .border_style(if app.preview_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        if let Some(i) = app.project_state.selected() {
            let project = &app.projects[i];
            let current_path = app
                .project_explorer_path
                .clone()
                .unwrap_or_else(|| project.path.clone());
            let expanded_path = if current_path.starts_with('~') {
                current_path.replacen('~', &std::env::var("HOME").unwrap_or_default(), 1)
            } else {
                current_path.clone()
            };

            // List directory contents
            let mut file_items = Vec::new();

            if let Ok(entries) = std::fs::read_dir(&expanded_path) {
                let mut dir_entries: Vec<_> = entries.flatten().collect();
                dir_entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

                app.explorer_items = dir_entries
                    .iter()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();

                for (idx, entry) in dir_entries.iter().enumerate().take(30) {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.path().is_dir();
                    let icon = if is_dir { "📁" } else { "📄" };
                    let color = if is_dir { Color::Blue } else { Color::White };

                    let style = if app.preview_focused && idx == app.project_explorer_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color)
                    };

                    file_items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("    {} ", icon), style),
                        Span::styled(file_name, style),
                    ])));
                }
                if dir_entries.len() > 30 {
                    file_items.push(ListItem::new(Line::from(vec![Span::styled(
                        format!("    ... and {} more items", dir_entries.len() - 30),
                        Style::default().fg(Color::DarkGray).italic(),
                    )])));
                }
            } else {
                file_items.push(ListItem::new(Span::styled(
                    "  Error reading directory. (Path might not exist or need expansion)",
                    Style::default().fg(Color::Red),
                )));
            }

            let list = List::new(file_items).block(preview_block);
            frame.render_widget(list, chunks[1]);
        } else {
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from("  Select a project to explore its files."),
                Line::from(""),
                Line::from("  Press Enter to open in:"),
                Line::from(vec![Span::styled(
                    format!("  {} ", app.config.editor.to_uppercase()),
                    Style::default().fg(Color::Green).bold(),
                )]),
                Line::from(""),
                Line::from("  (Use 'Space' on installer to change editor)"),
            ])
            .block(preview_block)
            .alignment(Alignment::Center);
            frame.render_widget(para, chunks[1]);
        }
    }
}
