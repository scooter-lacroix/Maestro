use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use super::pane::ConductorPane;
use super::model::DetailsViewMode;
use super::theme::ConductorTheme;
use leindex_core::orchestrate::model::SessionStatus;

pub fn render_details_panel(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, theme: &crate::theme::Theme) {
    let conductor_theme = ConductorTheme::default();
    match pane.details_mode {
        DetailsViewMode::Details => render_details_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Output => render_output_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Prompt => render_prompt_view(frame, area, pane, &conductor_theme, theme),
    }
}

fn render_details_view(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, conductor_theme: &ConductorTheme, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Task Details & Commands ")
        .border_style(Style::default().fg(theme.accent));

    let items = pane.get_selectable_items();
    let selected_item = if items.is_empty() {
        None
    } else {
        Some(&items[pane.selected_index.min(items.len() - 1)])
    };

    let details_text = if let Some(item) = selected_item {
        match item {
            super::model::SelectableItem::Track { id, .. } => {
                let start_cmd = pane.get_start_command(Some("claude"), false, false);
                vec![
                    Line::from(vec![
                        Span::styled("Track: ", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(id, Style::default().bold().fg(conductor_theme.fg_primary)),
                    ]),
                    Line::from(vec![
                        Span::styled("Mode:  ", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(format!("{:?}", pane.loop_mode), Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled("Status:", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(
                            format!(" {:?}", pane.session_status),
                            Style::default().fg(match pane.session_status {
                                SessionStatus::Running => conductor_theme.status_success,
                                SessionStatus::Paused => conductor_theme.status_warning,
                                SessionStatus::Completed => conductor_theme.status_info,
                                SessionStatus::Failed | SessionStatus::Interrupted => conductor_theme.status_error,
                                _ => conductor_theme.fg_muted,
                            })
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Commands:", Style::default().fg(conductor_theme.accent_primary).underlined())),
                    Line::from(vec![
                        Span::styled("  [s] Start: ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(start_cmd, Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled("  [p] Pause:  ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(pane.get_pause_command(), Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled("  [r] Resume: ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(pane.get_resume_command(), Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                ]
            }
            super::model::SelectableItem::Task { title, id, status, .. } => {
                vec![
                    Line::from(vec![
                        Span::styled("Task:  ", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(title, Style::default().bold().fg(conductor_theme.fg_primary)),
                    ]),
                    Line::from(vec![
                        Span::styled("ID:    ", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(id, Style::default().fg(conductor_theme.fg_muted)),
                    ]),
                    Line::from(vec![
                        Span::styled("Status:", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(format!(" {:?}", status), Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                ]
            }
        }
    } else {
        vec![Line::from(Span::styled("No item selected.", Style::default().fg(conductor_theme.fg_muted)))]
    };

    let paragraph = Paragraph::new(details_text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_output_view(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, conductor_theme: &ConductorTheme, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Live Output ")
        .border_style(Style::default().fg(theme.accent));

    let output_text: Vec<Line> = pane.iteration_output
        .iter()
        .map(|line| Line::from(Span::styled(line.as_str(), Style::default().fg(conductor_theme.fg_secondary))))
        .collect();

    let output = Paragraph::new(output_text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((pane.output_scroll, 0));

    frame.render_widget(output, area);
}

fn render_prompt_view(frame: &mut Frame, area: Rect, _pane: &mut ConductorPane, conductor_theme: &ConductorTheme, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" LLM Prompt Preview ")
        .border_style(Style::default().fg(theme.accent));

    let text = vec![
        Line::from(Span::styled("Prompt preview not yet implemented.", Style::default().fg(conductor_theme.fg_muted))),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
