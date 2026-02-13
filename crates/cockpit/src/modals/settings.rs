//! Settings menu modal rendering
//!
//! This module provides the settings menu modal for selecting
//! configuration options like editor and theme.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem};

use crate::state::SettingsMenuKind;
<<<<<<< HEAD
use crate::theme::theme_from_name;
=======
use crate::theme::{theme_from_name, Theme};
>>>>>>> 0cef1ec7 (feat(v2.5-phase5): Extract modal rendering to dedicated modals module)
use crate::app::App;

/// Renders the settings menu modal.
///
/// This modal displays a list of selectable options for settings like:
/// - Preferred Editor (via SettingsMenuKind::Editor)
/// - Theme selection (via SettingsMenuKind::Theme)
///
/// The title and content of the menu change based on the `settings_menu_kind`.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Mutable reference to the application state (needed for ListState)
pub fn render_settings_menu_modal(frame: &mut Frame, app: &mut App) {
    let theme = theme_from_name(&app.config.theme);
    let area = crate::modals::centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let title = match app.settings_menu_kind {
        Some(SettingsMenuKind::Editor) => " Select Preferred Editor ",
        Some(SettingsMenuKind::Theme) => " Select Theme ",
        None => " Select ",
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let items: Vec<ListItem> = app
        .settings_menu_items
        .iter()
        .map(|(_, label)| ListItem::new(label.clone()))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold()
                .add_modifier(ratatui::style::Modifier::UNDERLINED),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.settings_menu_state);
}
