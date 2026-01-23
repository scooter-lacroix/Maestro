//! Analysis tab rendering for Cockpit TUI

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph, Wrap},
    Frame,
    prelude::*,
};

use crate::app::App;
use crate::state::InputMode;

pub fn render_analysis(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // History
            Constraint::Length(3), // Progress / Status
            Constraint::Length(3), // Input Prompt
        ])
        .split(area);

    let theme = app.theme();
    let hub_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🚀 Analysis Command Hub ")
        .title_style(Style::default().fg(theme.accent_alt));

    // History View
    let mut history_lines = vec![
        Line::from(vec![Span::styled(
            " Maestro Analysis Engine v2.0 READY",
            Style::default().fg(Color::Green).bold(),
        )]),
        Line::from(vec![
            Span::styled(
                " Type '/phase1 <path>' to begin. ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                "(Press 'a' to enter Command Hub)",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(""),
    ];

    let examples = vec![
        Line::from(vec![Span::styled(
            " EXAMPLES:",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from("  $ /phase1 . --mode ultra --files 20"),
        Line::from("  $ /phase2 ."),
        Line::from("  $ /phase3 . --focus-files 2"),
        Line::from("  $ /phase4 . --top 10"),
        Line::from("  $ /phase5 ."),
        Line::from(""),
    ];
    history_lines.extend(examples);

    if app.analysis_history.is_empty() {
        history_lines.push(Line::from("  No recent analysis runs."));
    } else {
        for line in &app.analysis_history {
            history_lines.push(Line::from(line.as_str()));
        }
    }

    let history = Paragraph::new(history_lines)
        .block(hub_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(history, chunks[0]);

    // Progress / Status Bar
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let status_text = if app.input_mode == InputMode::AnalysisPrompt {
        " STATUS: Awaiting Command... "
    } else {
        " STATUS: Idle "
    };
    let status = Paragraph::new(status_text)
        .block(status_block)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, chunks[1]);

    // Input Prompt
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(if app.input_mode == InputMode::AnalysisPrompt {
            " ⌨️ Command (Esc/Enter to finish) > "
        } else {
            " Command > "
        })
        .title_style(Style::default().fg(Color::Cyan));

    let input_text = if app.input_mode == InputMode::AnalysisPrompt {
        format!("{}█", app.analysis_input)
    } else {
        app.analysis_input.clone()
    };

    let input = Paragraph::new(input_text).block(input_block).style(
        if app.input_mode == InputMode::AnalysisPrompt {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        },
    );
    frame.render_widget(input, chunks[2]);
}
