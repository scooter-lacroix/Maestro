//! Analysis tab rendering for Cockpit TUI
//!
//! Enhanced with LeIndex 5-phase analysis integration

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph, Wrap},
    Frame,
    prelude::*,
};

use crate::app::App;
use crate::state::{AnalysisMode, InputMode};

pub fn render_analysis(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with mode indicator
            Constraint::Length(5), // Quick actions display
            Constraint::Length(3), // Phase buttons
            Constraint::Min(0),    // History
            Constraint::Length(3), // Input Prompt
        ])
        .split(area);

    let theme = app.theme();

    // Header with mode indicator
    let mode_text = match app.analysis_mode {
        AnalysisMode::Ultra => "ULTRA (Exploration)",
        AnalysisMode::Balanced => "BALANCED (Implementation)",
    };
    let mode_color = match app.analysis_mode {
        AnalysisMode::Ultra => Color::Cyan,
        AnalysisMode::Balanced => Color::Green,
    };

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" 🚀 Analysis Command Hub    Mode: {} ", mode_text))
        .title_style(Style::default().fg(mode_color).bold());

    frame.render_widget(header_block, chunks[0]);

    // Quick Action Buttons Display
    let quick_actions = vec![
        Line::from(vec![
            Span::styled(" QUICK WORKFLOWS: ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("[F]", Style::default().fg(Color::Green)),
            Span::styled(" Fast  ", Style::default().fg(Color::Gray)),
            Span::styled("[I]", Style::default().fg(Color::Green)),
            Span::styled(" Implementation  ", Style::default().fg(Color::Gray)),
            Span::styled("[M]", Style::default().fg(Color::Green)),
            Span::styled(" Toggle Mode", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(" PHASES: ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("[1]", Style::default().fg(Color::Blue)),
            Span::styled(" Scan  ", Style::default().fg(Color::Gray)),
            Span::styled("[2]", Style::default().fg(Color::Blue)),
            Span::styled(" Deps  ", Style::default().fg(Color::Gray)),
            Span::styled("[3]", Style::default().fg(Color::Blue)),
            Span::styled(" Logic  ", Style::default().fg(Color::Gray)),
            Span::styled("[4]", Style::default().fg(Color::Blue)),
            Span::styled(" Data  ", Style::default().fg(Color::Gray)),
            Span::styled("[5]", Style::default().fg(Color::Blue)),
            Span::styled(" Opt  ", Style::default().fg(Color::Gray)),
            Span::styled("[B]", Style::default().fg(Color::Magenta)),
            Span::styled(" Bundle", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
    ];

    let actions_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let actions_paragraph = Paragraph::new(quick_actions).block(actions_block);
    frame.render_widget(actions_paragraph, chunks[1]);

    // Context Bundle Info
    let bundle_line = Line::from(vec![
        Span::styled(
            " Context Bundle: ",
            Style::default().fg(Color::Yellow).bold(),
        ),
        Span::styled(
            match app.analysis_mode {
                AnalysisMode::Ultra => "Ultra ~2K tokens (exploration)",
                AnalysisMode::Balanced => "Balanced ~27K tokens (implementation-ready)",
            },
            Style::default().fg(Color::Gray).italic(),
        ),
    ]);

    let bundle_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let bundle_paragraph = Paragraph::new(bundle_line).block(bundle_block);
    frame.render_widget(bundle_paragraph, chunks[2]);

    // History View
    let mut history_lines = vec![];

    if app.analysis_history.is_empty() {
        history_lines.push(Line::from(vec![Span::styled(
            " No analysis history yet.",
            Style::default().fg(Color::DarkGray).italic(),
        )]));
        history_lines.push(Line::from(""));
        history_lines.push(Line::from(vec![
            Span::styled(" Quick Start:", Style::default().fg(Color::Yellow)),
        ]));
        history_lines.push(Line::from("   Press [F] for fast orientation (ultra mode)"));
        history_lines.push(Line::from("   Press [I] for implementation-ready (balanced mode)"));
        history_lines.push(Line::from("   Press [1-5] for individual phases"));
        history_lines.push(Line::from("   Press [M] to toggle between Ultra/Balanced modes"));
        history_lines.push(Line::from(""));
        history_lines.push(Line::from(vec![
            Span::styled(" Examples:", Style::default().fg(Color::Yellow)),
        ]));
        history_lines.push(Line::from("   /phase1 . --mode ultra --files 20"));
        history_lines.push(Line::from("   /phase1 . --mode balanced --files 50"));
        history_lines.push(Line::from("   /phase2 . (dependency map)"));
        history_lines.push(Line::from("   /phase3 . (logic flow)"));
        history_lines.push(Line::from("   /phase4 . (data flow)"));
        history_lines.push(Line::from("   /phase5 . (optimization)"));
    } else {
        // Show most recent history entries first (reversed)
        for entry in app.analysis_history.iter().rev().take(10) {
            history_lines.push(Line::from(vec![
                Span::styled(
                    format!(" • {}", entry),
                    Style::default().fg(Color::Gray),
                ),
            ]));
            history_lines.push(Line::from(""));
        }
    }

    let history_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" History ({} shown, max 20) ", app.analysis_history.len()))
        .title_style(Style::default().fg(theme.accent_alt));

    let history = Paragraph::new(history_lines)
        .block(history_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(history, chunks[3]);

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
    frame.render_widget(input, chunks[4]);
}
