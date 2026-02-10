//! Wizard modal rendering for multi-step workflows
//!
//! This module provides wizard-style modals for creating new projects,
//! groups, tracks, and sessions. These modals guide users through
//! multi-step input processes.

use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::App;
use crate::state::InputMode;

/// Renders the new project wizard modal.
///
/// This wizard guides users through creating a new project in three steps:
/// 1. Project Name
/// 2. Target Path
/// 3. Initial Tool (None/claude/gemini)
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_new_project_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);

    let step = match app.input_mode {
        InputMode::NewProjectName => 1,
        InputMode::NewProjectPath => 2,
        InputMode::NewProjectTool => 3,
        _ => 1,
    };

    let block = Block::default()
        .title(format!(" NEW PROJECT WIZARD (Step {} of 3) ", step))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(15, 10, 20)));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Path
            Constraint::Length(3), // Tool
            Constraint::Min(0),    // Help/Hint
        ])
        .split(area);

    let name_style = if step == 1 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name = Paragraph::new(app.new_project_name.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 1. PROJECT NAME ")
            .border_style(name_style),
    );
    frame.render_widget(name, chunks[0]);

    let path_style = if step == 2 {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let path = Paragraph::new(app.new_project_path.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 2. TARGET PATH (Enter for current) ")
            .border_style(path_style),
    );
    frame.render_widget(path, chunks[1]);

    let tool_style = if step == 3 {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let tool = Paragraph::new(app.new_project_tool.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 3. INITIAL TOOL (None/claude/gemini) ")
            .border_style(tool_style),
    );
    frame.render_widget(tool, chunks[2]);

    let hint = Paragraph::new("Press 'Enter' to confirm step, 'Esc' to cancel\n\nThis will run /maestro:setup in the target directory.")
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[3]);
    frame.render_widget(block, area);
}

/// Renders the group management wizard modal.
///
/// This wizard handles both creating new groups and renaming existing ones.
/// It guides users through two steps:
/// 1. Group Name
/// 2. Category (e.g. Work, Personal, Research)
///
/// The title changes based on whether the operation is a create or rename.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_group_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);

    let step = match app.input_mode {
        InputMode::NewGroupTitle | InputMode::RenameGroup => 1,
        InputMode::NewGroupCategory | InputMode::RenameGroupCategory => 2,
        _ => 1,
    };

    let title = if matches!(
        app.input_mode,
        InputMode::RenameGroup | InputMode::RenameGroupCategory
    ) {
        " RENAME GROUP WIZARD "
    } else {
        " NEW GROUP WIZARD "
    };

    let block = Block::default()
        .title(format!(" {} (Step {} of 2) ", title, step))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(10, 20, 15)));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Category
            Constraint::Min(0),    // Help/Hint
        ])
        .split(area);

    let name_style = if step == 1 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name = Paragraph::new(app.rename_buffer.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 1. GROUP NAME ")
            .border_style(name_style),
    );
    frame.render_widget(name, chunks[0]);

    let cat_style = if step == 2 {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let cat = Paragraph::new(app.new_group_category.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 2. CATEGORY (e.g. Work, Personal, Research) ")
            .border_style(cat_style),
    );
    frame.render_widget(cat, chunks[1]);

    let hint = Paragraph::new("Tab to switch fields, Enter: next/save, Esc to cancel\n\nGroups help you organize your coding sessions.")
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
    frame.render_widget(block, area);
}

/// Renders the new track wizard modal.
///
/// This wizard guides users through creating a new orchestration track
/// in two steps:
/// 1. Track Title
/// 2. Track Type (Master Track or Direct Track)
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_new_track_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(60, 30, frame.area());
    frame.render_widget(Clear, area);

    let step = match app.input_mode {
        InputMode::NewTrackTitle => 1,
        InputMode::NewTrackType => 2,
        _ => 1,
    };

    let block = Block::default()
        .title(format!(" NEW TRACK WIZARD (Step {} of 2) ", step))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(10, 15, 20)));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Type
            Constraint::Min(0),    // Help/Hint
        ])
        .split(area);

    let title_style = if step == 1 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = Paragraph::new(app.new_track_title.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 1. TRACK TITLE ")
            .border_style(title_style),
    );
    frame.render_widget(title, chunks[0]);

    let type_style = if step == 2 {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let type_text = if app.new_track_is_master {
        "[X] Master Track  [ ] Direct Track"
    } else {
        "[ ] Master Track  [X] Direct Track"
    };
    let track_type = Paragraph::new(type_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 2. TRACK TYPE (Space to toggle) ")
            .border_style(type_style),
    );
    frame.render_widget(track_type, chunks[1]);

    let hint = Paragraph::new("Press 'Enter' to confirm, 'Esc' to cancel\n\nThis will run /maestro:newTrack in the project.")
        .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
    frame.render_widget(block, area);
}

/// Renders the new session wizard modal.
///
/// This wizard guides users through creating a new coding session
/// with three fields:
/// - Session Title
/// - Project Path
/// - Tool selection (cycleable)
///
/// Users can navigate between fields using Tab and confirm with Enter.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_input_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(60, 20, frame.area());
    let block = Block::default()
        .title(" New Session Wizard ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let mut text = vec![Line::from("")];

    // Title Field
    let title_style = if app.input_mode == InputMode::NewSessionTitle {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default()
    };
    text.push(Line::from(vec![
        Span::styled("  Session Title: ", title_style),
        Span::raw(&app.new_session_title),
        if app.input_mode == InputMode::NewSessionTitle {
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK))
        } else {
            Span::raw("")
        },
    ]));

    // Path Field
    let path_style = if app.input_mode == InputMode::NewSessionPath {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default()
    };
    text.push(Line::from(vec![
        Span::styled("  Project Path:  ", path_style),
        Span::raw(&app.new_session_path),
        if app.input_mode == InputMode::NewSessionPath {
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK))
        } else {
            Span::raw("")
        },
    ]));

    // Tool Field
    let tool_style = if app.input_mode == InputMode::NewSessionTool {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default()
    };
    text.push(Line::from(vec![
        Span::styled("  Tool (Cycle):  ", tool_style),
        Span::styled(
            &app.new_session_tool,
            Style::default().fg(Color::Cyan).bold(),
        ),
        if app.input_mode == InputMode::NewSessionTool {
            Span::raw(" (Press any key to cycle)")
        } else {
            Span::raw("")
        },
    ]));

    text.push(Line::from(""));
    text.push(Line::from("  [Enter] Next/Confirm  [Esc] Cancel"));

    let para = Paragraph::new(text).block(block);
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}
