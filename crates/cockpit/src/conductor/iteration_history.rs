use super::pane::ConductorPane;
use leindex_core::orchestrate::model::IterationStatus;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

pub fn render_iteration_history(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(if pane.iteration_history_focused {
            " Iteration History [focused: Enter opens, Esc closes] "
        } else {
            " Iteration History [Shift+I to focus] "
        })
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
            Line::from(Span::styled(
                "  No history available yet.",
                Style::default().fg(theme.muted),
            )),
        ];
        frame.render_widget(Paragraph::new(text), inner_area);
        return;
    }

    let items: Vec<ListItem> = pane
        .state
        .iteration_logs
        .iter()
        .rev()
        .map(|log| {
            let (status_symbol, status_color) = match log.status {
                IterationStatus::Running => ("↻", Color::Yellow),
                IterationStatus::Completed => ("✔", Color::Green),
                IterationStatus::Failed => ("✘", Color::Red),
                IterationStatus::Skipped => ("⊖", Color::Gray),
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_symbol),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("Iter {}", log.iteration),
                    Style::default().fg(theme.fg).bold(),
                ),
                Span::styled(
                    format!(": {}", log.task_id),
                    Style::default().fg(theme.muted),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(if pane.iteration_history_focused {
                Color::Rgb(35, 55, 90)
            } else {
                Color::Rgb(35, 35, 35)
            })
            .fg(Color::White)
            .bold(),
    );
    let mut state = ListState::default();
    state.select(Some(
        pane.selected_iteration_log
            .min(pane.state.iteration_logs.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(list, inner_area, &mut state);
}

pub fn render_iteration_popup(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let popup = centered_rect(78, 76, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Iteration Detail ")
        .border_style(Style::default().fg(theme.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let Some(log) = pane.selected_iteration_log() else {
        frame.render_widget(
            Paragraph::new("No iteration selected.").style(Style::default().fg(theme.muted)),
            inner,
        );
        return;
    };

    let status_color = match log.status {
        IterationStatus::Running => Color::Yellow,
        IterationStatus::Completed => Color::Green,
        IterationStatus::Failed => Color::Red,
        IterationStatus::Skipped => Color::Gray,
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Iteration ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                log.iteration.to_string(),
                Style::default().fg(Color::White).bold(),
            ),
            Span::styled("  Task ", Style::default().fg(Color::DarkGray)),
            Span::styled(&log.task_id, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Status ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:?}", log.status),
                Style::default().fg(status_color),
            ),
            Span::styled("  Duration ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} ms", log.duration_ms),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Started ", Style::default().fg(Color::DarkGray)),
            Span::styled(&log.started_at, Style::default().fg(Color::White)),
        ]),
    ];

    if let Some(completed_at) = &log.completed_at {
        lines.push(Line::from(vec![
            Span::styled("Completed ", Style::default().fg(Color::DarkGray)),
            Span::styled(completed_at, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Task Loop Snapshot:",
        Style::default().fg(theme.accent).bold(),
    )));
    lines.push(Line::from(vec![
        Span::styled("Current task ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            pane.state
                .current_task
                .clone()
                .unwrap_or_else(|| "idle".to_string()),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("  Current loop ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            pane.state.current_iteration.to_string(),
            Style::default().fg(Color::White),
        ),
    ]));

    let related_logs: Vec<_> = pane
        .state
        .runtime_logs
        .iter()
        .filter(|entry| entry.task_id.as_deref() == Some(log.task_id.as_str()))
        .rev()
        .take(4)
        .collect();
    if related_logs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No structured runtime notes recorded for this iteration yet.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for entry in related_logs.into_iter().rev() {
            let mut summary = entry.summary.replace('\n', " ");
            if summary.len() > 88 {
                summary.truncate(85);
                summary.push_str("...");
            }
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent)),
                Span::styled(summary, Style::default().fg(Color::White)),
            ]));
        }
    }

    if let Some(error) = &log.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Error:",
            Style::default().fg(Color::Red).bold(),
        )));
        lines.push(Line::from(Span::styled(
            error,
            Style::default().fg(Color::White),
        )));
    }

    if !log.output.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Output:",
            Style::default().fg(theme.accent).bold(),
        )));
        for line in log.output.lines() {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::White),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Esc or Enter to close.",
        Style::default().fg(Color::DarkGray),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
