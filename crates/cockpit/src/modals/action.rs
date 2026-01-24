//! Action confirmation modal rendering
//!
//! This module provides the action modal, which is used for confirming
//! destructive or important actions like renaming, forking, killing sessions,
//! and permanent deletions.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::state::InputMode;
use crate::app::App;

/// Renders the action confirmation modal.
///
/// This modal displays different prompts based on the current `InputMode`:
/// - `RenameGroup`: Prompts for a new group name
/// - `ForkSession`: Prompts for a fork name
/// - `KillConfirm`: Confirms session termination
/// - `DeleteConfirm`: Confirms permanent deletion (with extra warning)
/// - `NewSessionTitle`: Prompts for a new session title
/// - `NewGroupTitle`: Prompts for a new group name
/// - `MoveToGroup`: Prompts for a target group path
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_action_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let (title, prompt, value) = match app.input_mode {
        InputMode::RenameGroup => (" Rename Group ", "New Name:", Some(&app.rename_buffer)),
        InputMode::ForkSession => (" Fork Session ", "Fork Name:", Some(&app.rename_buffer)),
        InputMode::KillConfirm => (" Kill Session ", "Are you sure? (y/n)", None),
        InputMode::DeleteConfirm => (
            " Permanent Delete ",
            "Are you sure you want to PERMANENTLY delete? (y/n)",
            None,
        ),
        InputMode::NewSessionTitle => (
            " New Session ",
            "Enter Title:",
            Some(&app.new_session_title),
        ),
        InputMode::NewGroupTitle => (" New Group ", "Group Name:", Some(&app.rename_buffer)),
        InputMode::MoveToGroup => (" Move to Group ", "Target Path:", Some(&app.rename_buffer)),
        _ => ("", "", None),
    };

    let theme = app.theme();
    let title_style = match app.input_mode {
        InputMode::KillConfirm | InputMode::DeleteConfirm => {
            Style::default().fg(theme.error).bold()
        }
        _ => Style::default().fg(theme.warning).bold(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(title)
        .title_style(title_style);

    let content = if let Some(v) = value {
        format!("{}\n\n> {}", prompt, v)
    } else {
        prompt.to_string()
    };

    let para = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}
