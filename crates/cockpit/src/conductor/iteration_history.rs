use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};
use super::pane::ConductorPane;

pub fn render_iteration_history(frame: &mut Frame, area: Rect, _pane: &mut ConductorPane, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Iteration History ")
        .border_style(Style::default().fg(theme.muted));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let items = vec![
        ListItem::new("No history available yet."),
    ];

    let list = List::new(items);
    frame.render_widget(list, inner_area);
}
