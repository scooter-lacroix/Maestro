//! Projects tab rendering for Cockpit TUI

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use leindex_core::memory::models::SessionStatus;
use std::collections::HashMap;

/// Count active (running or waiting) sessions for a given project path
fn count_active_sessions(project_path: &str, sessions: &[crate::state::SessionEntry]) -> usize {
    sessions
        .iter()
        .filter_map(|entry| {
            if let crate::state::SessionEntry::Session(session) = entry {
                // Check if session's project_path matches or is within this project
                let session_path = &session.project_path;
                let matches = session_path == project_path
                    || session_path.starts_with(&format!("{}/", project_path))
                    || project_path.starts_with(&format!("{}/", session_path));
                if matches
                    && (session.status == SessionStatus::Running
                        || session.status == SessionStatus::Waiting)
                {
                    return Some(());
                }
            }
            None
        })
        .count()
}

pub fn render_projects(frame: &mut Frame, area: Rect, app: &mut App) {
    // Calculate total active sessions for the header
    let total_active: usize = app
        .session_entries
        .iter()
        .filter_map(|entry| {
            if let crate::state::SessionEntry::Session(session) = entry {
                if session.status == SessionStatus::Running
                    || session.status == SessionStatus::Waiting
                {
                    return Some(());
                }
            }
            None
        })
        .count();

    let title = if total_active > 0 {
        format!(" 🚀 Projects [●{} Active] ", total_active)
    } else {
        " 🚀 Projects ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
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
        // Pre-compute active session counts for all projects
        let active_session_counts: HashMap<String, usize> = app
            .projects
            .iter()
            .map(|p| {
                (
                    p.path.clone(),
                    count_active_sessions(&p.path, &app.session_entries),
                )
            })
            .collect();

        // Project List (Left)
        let items: Vec<ListItem> = app
            .projects
            .iter()
            .map(|p| {
                let active_count = active_session_counts.get(&p.path).copied().unwrap_or(0);
                let has_active = active_count > 0;

                let session_indicator = if has_active {
                    let color = if active_count == 1 {
                        Color::Green
                    } else {
                        Color::Yellow
                    };
                    Span::styled(
                        format!("●{} ", active_count),
                        Style::default().fg(color).bold(),
                    )
                } else {
                    Span::styled("  ", Style::default())
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {} ", "📦"), Style::default()),
                        Span::styled(&p.name, Style::default().fg(Color::Cyan).bold()),
                        session_indicator,
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
            let active_count = *active_session_counts.get(&project.path).unwrap_or(&0);

            // Show session info in preview if project has active sessions
            if active_count > 0 {
                let active_sessions: Vec<_> = app
                    .session_entries
                    .iter()
                    .filter_map(|entry| {
                        if let crate::state::SessionEntry::Session(session) = entry {
                            let session_path = &session.project_path;
                            let matches = session_path == &project.path
                                || session_path.starts_with(&format!("{}/", &project.path))
                                || project.path.starts_with(&format!("{}/", session_path));
                            if matches
                                && (session.status == SessionStatus::Running
                                    || session.status == SessionStatus::Waiting)
                            {
                                return Some(session);
                            }
                        }
                        None
                    })
                    .collect();

                let mut preview_lines = vec![
                    Line::from(vec![
                        Span::styled(" 📦 ", Style::default().fg(Color::Cyan).bold()),
                        Span::styled(&project.name, Style::default().fg(Color::White).bold()),
                    ]),
                    Line::from(vec![
                        Span::styled(" 📁 ", Style::default()),
                        Span::styled(&project.path, Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        format!(
                            " ● {} Active Session{}",
                            active_count,
                            if active_count == 1 { "" } else { "s" }
                        ),
                        Style::default().fg(Color::Green).bold(),
                    )]),
                    Line::from(""),
                ];

                for session in active_sessions {
                    let (status_icon, status_color) = if session.status == SessionStatus::Running {
                        (" ● ", Color::Green)
                    } else {
                        (" ◒ ", Color::Yellow)
                    };
                    preview_lines.push(Line::from(vec![
                        Span::styled("   ", Style::default()),
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::styled(&session.title, Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!(" [{}]", session.tool.as_deref().unwrap_or("?")),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                    preview_lines.push(Line::from(vec![
                        Span::styled("     ", Style::default()),
                        Span::styled(
                            format!("ID: {}", session.session_id),
                            Style::default().fg(Color::DarkGray).italic(),
                        ),
                    ]));
                    preview_lines.push(Line::from(""));
                }

                preview_lines.push(Line::from(""));
                preview_lines.push(Line::from(vec![
                    Span::styled(" Press ", Style::default().fg(Color::DarkGray)),
                    Span::styled("s", Style::default().fg(Color::Yellow).bold()),
                    Span::styled(
                        " to switch to Sessions tab",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                let para = Paragraph::new(preview_lines).block(preview_block);
                frame.render_widget(para, chunks[1]);
            } else {
                // Original file explorer logic for projects without active sessions
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
            }
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
