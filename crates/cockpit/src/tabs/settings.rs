//! Settings tab rendering for Cockpit TUI

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::state::SettingsOption;
use crate::theme::{theme_from_name, THEMES};

pub fn render_settings(frame: &mut Frame, app: &App) {
    let theme = theme_from_name(&app.config.theme);
    let area = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚙️ SYSTEM SETTINGS ")
        .border_type(BorderType::Rounded)
        .border_style(ratatui::style::Style::default().fg(theme.accent));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Editor
            Constraint::Length(3), // Theme
            Constraint::Length(3), // Transparent
            Constraint::Length(3), // Install Path
            Constraint::Length(3), // Save button
            Constraint::Min(0),
        ])
        .split(inner_area);

    let editor_style = if app.tab_index == 7 && app.settings_option == SettingsOption::Editor {
        ratatui::style::Style::default().fg(theme.warning).bold()
    } else {
        ratatui::style::Style::default()
    };
    let editor = Paragraph::new(app.config.editor.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 📝 PREFERRED EDITOR ")
            .border_style(editor_style),
    );
    frame.render_widget(editor, chunks[0]);

    let theme_style = if app.tab_index == 7 && app.settings_option == SettingsOption::Theme {
        ratatui::style::Style::default().fg(theme.warning).bold()
    } else {
        ratatui::style::Style::default()
    };
    let theme_name = THEMES
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(app.config.theme.as_str()))
        .map(|(_, label)| *label)
        .unwrap_or("Custom");
    let theme_field = Paragraph::new(theme_name).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 🎨 THEME ")
            .border_style(theme_style),
    );
    frame.render_widget(theme_field, chunks[1]);

    let transparent_style =
        if app.tab_index == 7 && app.settings_option == SettingsOption::Transparent {
            ratatui::style::Style::default().fg(theme.warning).bold()
        } else {
            ratatui::style::Style::default()
        };
    let transparent_text = if app.config.transparent {
        "ON (terminal background visible)"
    } else {
        "OFF (theme background)"
    };
    let transparent = Paragraph::new(transparent_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 🪟 TRANSPARENCY ")
            .border_style(transparent_style),
    );
    frame.render_widget(transparent, chunks[2]);

    let path_style = if app.tab_index == 7 && app.settings_option == SettingsOption::InstallPath {
        ratatui::style::Style::default().fg(theme.warning).bold()
    } else {
        ratatui::style::Style::default()
    };
    let path = Paragraph::new(app.config.install_path.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 📁 MAESTRO INSTALL PATH ")
            .border_style(path_style),
    );
    frame.render_widget(path, chunks[3]);

    let save_style = if app.tab_index == 7 && app.settings_option == SettingsOption::Save {
        ratatui::style::Style::default()
            .bg(theme.success)
            .fg(ratatui::style::Color::Black)
            .bold()
    } else {
        ratatui::style::Style::default().fg(theme.success)
    };
    let save = Paragraph::new(" [ SAVE CONFIGURATION ] ")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(save_style),
        );
    frame.render_widget(save, chunks[4]);

    let help = Paragraph::new("Use ↑/↓ to navigate, Enter to edit selected field. Settings are stored in ~/.config/maestro/config.toml")
        .alignment(Alignment::Center)
        .style(ratatui::style::Style::default().fg(theme.muted));
    frame.render_widget(help, chunks[5]);
}
