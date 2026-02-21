use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::input_modal::centered_rect;
use crate::conductor::model::ConductorState;

pub fn render_project_selector(f: &mut Frame, area: Rect, state: &ConductorState) {
    let selector_area = centered_rect(70, 60, area);
    f.render_widget(Clear, selector_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Switch Project ")
        .border_style(Style::default().fg(Color::Yellow));

    if state.available_projects.is_empty() {
        let text = vec![
            Line::from("No Maestro projects discovered."),
            Line::from("Ensure tracks.md exists in your current dir or active tmux panes."),
        ];
        let p = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(p, selector_area);
        return;
    }

    let items: Vec<ListItem> = state
        .available_projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == state.selected_project_index {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };

            let root_path = p.root_dir.to_string_lossy();
            let tracks_path = p.tracks_path.to_string_lossy();

            ListItem::new(vec![
                Line::from(Span::styled(
                    format!(" {} ", p.name()),
                    style.add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("   Root:   {}", root_path)),
                Line::from(format!("   Tracks: {}", tracks_path)),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items).block(block);

    f.render_widget(list, selector_area);

    // Help message at the bottom of the selector
    let help_area = Rect::new(
        selector_area.x + 1,
        selector_area.y + selector_area.height - 2,
        selector_area.width - 2,
        1,
    );
    let help_text = Paragraph::new("↑/↓: Navigate • Enter: Switch • Esc/P: Close")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help_text, help_area);
}
