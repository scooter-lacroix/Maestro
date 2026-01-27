use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use leindex_core::orchestrate::model::IterationStatus;
use super::pane::ConductorPane;

pub fn render_iteration_history(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Iteration History ")
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if pane.state.iteration_logs.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled("  No history available yet.", Style::default().fg(theme.muted))),
        ];
        frame.render_widget(Paragraph::new(text), inner_area);
        return;
    }

    let items: Vec<ListItem> = pane.state.iteration_logs.iter().rev().map(|log| {
        let (status_symbol, status_color) = match log.status {
            IterationStatus::Running => ("↻", Color::Yellow),
            IterationStatus::Completed => ("✔", Color::Green),
            IterationStatus::Failed => ("✘", Color::Red),
            IterationStatus::Skipped => ("⊖", Color::Gray),
        };

        let line = Line::from(vec![
            Span::styled(format!(" {} ", status_symbol), Style::default().fg(status_color)),
            Span::styled(format!("Iter {}", log.iteration), Style::default().fg(theme.fg).bold()),
            Span::styled(format!(": {}", log.task_id), Style::default().fg(theme.muted)),
        ]);
        ListItem::new(line)
    }).collect();

    let list = List::new(items);
    frame.render_widget(list, inner_area);
}
