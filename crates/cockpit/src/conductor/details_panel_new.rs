use super::model::DetailsViewMode;
use super::pane::ConductorPane;
use super::theme::ConductorTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

fn render_prompt_view_fresh(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    _conductor_theme: &ConductorTheme,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" LLM Prompt Preview ")
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let items = pane.get_selectable_items();
    let selected_item = if items.is_empty() {
        None
    } else {
        Some(&items[pane.selected_index.min(items.len() - 1)])
    };

    let text = if let Some(item) = selected_item {
        match item {
            super::model::SelectableItem::Track { id, .. } => {
                vec![
                    Line::from(vec![
                        Span::styled("Track: ", Style::default().fg(theme.accent)),
                        Span::styled(id, Style::default().bold()),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "No prompt preview available for tracks. Select a task.",
                        Style::default().fg(theme.muted).italic(),
                    )),
                ]
            }
            super::model::SelectableItem::Task { id, title, .. } => {
                // Try to get full task data for better preview
                let full_task = pane.find_task_by_id(id);

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("Task: ", Style::default().fg(theme.accent)),
                        Span::styled(title, Style::default().bold()),
                    ]),
                    Line::from(vec![
                        Span::styled("ID:   ", Style::default().fg(theme.muted)),
                        Span::styled(id, Style::default().fg(theme.muted)),
                    ]),
                ];

                // Show task description if available
                if let Some(ref task) = full_task {
                    if !task.description.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled("Description:", Style::default().fg(theme.accent).underlined()),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", task.description), Style::default().fg(theme.fg)),
                        ]));
                    }
                }

                // Always show what context gets injected
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Context Injection:", Style::default().fg(theme.accent).underlined()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("• ", Style::default().fg(theme.fg)),
                    Span::styled("Project analysis (ast, callgraph)", Style::default().fg(theme.fg)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("• ", Style::default().fg(theme.fg)),
                    Span::styled("Recent iteration history", Style::default().fg(theme.fg)),
                ]));

                // Show memory count if available
                if !pane.state.track_memories.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("• ", Style::default().fg(theme.fg)),
                        Span::styled(
                            format!("{} memories from database", pane.state.track_memories.len()),
                            Style::default().fg(theme.fg),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("• ", Style::default().fg(theme.fg)),
                        Span::styled("Relevant memories from database", Style::default().fg(theme.fg)),
                    ]));
                }

                // Show token estimate
                let estimated_tokens = 2000 + (full_task.as_ref().map_or(0, |t| t.description.len() * 2));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Estimated tokens: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("~{}", estimated_tokens), Style::default().fg(theme.accent)),
                ]));

                lines
            }
        }
    } else {
        vec![Line::from(Span::styled(
            "No item selected.",
            Style::default().fg(theme.muted),
        ))]
    };

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
