use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use leindex_core::orchestrate::model::{IterationStatus, SessionStatus};
use super::pane::ConductorPane;
use super::model::DetailsViewMode;
use super::theme::ConductorTheme;

pub fn render_details_panel(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, theme: &crate::theme::Theme) {
    let conductor_theme = ConductorTheme::default();
    match pane.details_mode {
        DetailsViewMode::Details => render_details_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Output => render_output_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Prompt => render_prompt_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Parallel => render_output_view(frame, area, pane, &conductor_theme, theme),
    }
}

fn render_details_view(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, conductor_theme: &ConductorTheme, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Task Details & Commands ")
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

    let details_text = if let Some(item) = selected_item {
        match item {
            super::model::SelectableItem::Track { id, .. } => {
                let tool_name = pane.state.active_agent.as_ref().map(|a| a.tool.clone()).unwrap_or_else(|| "claude".to_string());
                let start_cmd = pane
                    .get_start_command(Some(&tool_name), false, false)
                    .map(|cmd| cmd.to_string())
                    .unwrap_or_else(|| "// No track selected".to_string());
                let pause_cmd = pane
                    .get_pause_command()
                    .map(|cmd| cmd.to_string())
                    .unwrap_or_else(|| "// No track selected".to_string());
                let resume_cmd = pane
                    .get_resume_command()
                    .map(|cmd| cmd.to_string())
                    .unwrap_or_else(|| "// No track selected".to_string());
                
                let mut details = vec![
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
                    Line::from(vec![
                        Span::styled("Loop:  ", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(format!("Iteration {}", pane.state.current_iteration), Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Agent Activity:", Style::default().fg(conductor_theme.accent_primary).underlined())),
                ];

                if let Some(agent) = &pane.state.active_agent {
                    details.push(Line::from(vec![
                        Span::styled("  Tool:  ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(&agent.tool, Style::default().fg(conductor_theme.fg_secondary)),
                    ]));
                    if let Some(model) = &agent.model {
                        details.push(Line::from(vec![
                            Span::styled("  Model: ", Style::default().fg(conductor_theme.fg_muted)),
                            Span::styled(model, Style::default().fg(conductor_theme.fg_secondary)),
                        ]));
                    }
                    details.push(Line::from(vec![
                        Span::styled("  Since: ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(agent.since.format("%H:%M:%S").to_string(), Style::default().fg(conductor_theme.fg_secondary)),
                    ]));
                } else {
                    details.push(Line::from(Span::styled("  No active agent", Style::default().fg(conductor_theme.fg_muted))));
                }

                if let Some(rl) = &pane.state.rate_limit {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled("  ⚠️ RATE LIMITED", Style::default().fg(conductor_theme.status_error).bold()),
                        Span::styled(format!(" (Retry #{})", rl.retry_count), Style::default().fg(conductor_theme.fg_muted)),
                    ]));
                    if let Some(backoff) = rl.backoff_until {
                        let remaining = backoff.signed_duration_since(chrono::Utc::now()).num_seconds();
                        if remaining > 0 {
                            details.push(Line::from(vec![
                                Span::styled("  Backoff: ", Style::default().fg(conductor_theme.fg_muted)),
                                Span::styled(format!("{}s", remaining), Style::default().fg(theme.warning)),
                            ]));
                        }
                    }
                }

                details.extend(vec![
                    Line::from(""),
                    Line::from(Span::styled("Commands:", Style::default().fg(conductor_theme.accent_primary).underlined())),
                    Line::from(vec![
                        Span::styled("  [s] Start: ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(start_cmd, Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled("  [p] Pause:  ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(pause_cmd, Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled("  [r] Resume: ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(resume_cmd, Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                ]);

                // Iteration Summary Section
                if let Some(last_log) = pane.state.iteration_logs.last() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled("Last Iteration Summary:", Style::default().fg(conductor_theme.accent_primary).underlined())));
                    details.push(Line::from(vec![
                        Span::styled(format!("  Iter {}: ", last_log.iteration), Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(format!("{:?}", last_log.status), Style::default().fg(match last_log.status {
                            IterationStatus::Completed => conductor_theme.status_success,
                            IterationStatus::Failed => conductor_theme.status_error,
                            _ => conductor_theme.fg_secondary,
                        })),
                    ]));
                    
                    if !last_log.output.is_empty() {
                        let summary = if last_log.output.len() > 200 {
                            format!("{}...", &last_log.output[..200].replace('\n', " "))
                        } else {
                            last_log.output.replace('\n', " ")
                        };
                        details.push(Line::from(vec![
                            Span::styled("  Output: ", Style::default().fg(conductor_theme.fg_muted)),
                            Span::styled(summary, Style::default().fg(conductor_theme.fg_secondary).italic()),
                        ]));
                    }
                }

                // Memories Section
                if !pane.state.track_memories.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled("Recent Memories:", Style::default().fg(conductor_theme.accent_primary).underlined())));
                    for memory in &pane.state.track_memories {
                        let prefix = format!(" • [{}] ", memory.category);
                        details.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(theme.accent)),
                            Span::styled(&memory.content, Style::default().fg(conductor_theme.fg_secondary)),
                        ]));
                    }
                }

                details
            }
            super::model::SelectableItem::Task {
                title,
                id,
                status,
                description,
                notes,
                is_blocked,
                is_actionable,
                dependencies,
                dependency_statuses,
                ..
            } => {
                let mut details = vec![
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
                    Line::from(vec![
                        Span::styled("Ready: ", Style::default().fg(conductor_theme.accent_secondary)),
                        Span::styled(
                            if *is_actionable { "actionable" } else if *is_blocked { "blocked" } else { "waiting" },
                            Style::default().fg(if *is_actionable {
                                conductor_theme.status_success
                            } else if *is_blocked {
                                conductor_theme.status_error
                            } else {
                                conductor_theme.fg_secondary
                            }),
                        ),
                    ]),
                ];

                if !description.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Description:",
                        Style::default().fg(conductor_theme.accent_primary).underlined(),
                    )));
                    details.push(Line::from(Span::styled(
                        description,
                        Style::default().fg(conductor_theme.fg_secondary),
                    )));
                }

                if !notes.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Notes:",
                        Style::default().fg(conductor_theme.accent_primary).underlined(),
                    )));
                    details.push(Line::from(Span::styled(
                        notes,
                        Style::default().fg(conductor_theme.fg_secondary).italic(),
                    )));
                }

                if !dependencies.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Dependencies:",
                        Style::default().fg(conductor_theme.accent_primary).underlined(),
                    )));

                    for (idx, dep) in dependencies.iter().enumerate() {
                        let icon = match dependency_statuses.get(idx) {
                            Some(crate::conductor::model::DependencyStatus::Completed) => "✓",
                            Some(crate::conductor::model::DependencyStatus::Blocked) => "⊘",
                            Some(crate::conductor::model::DependencyStatus::Pending) => "○",
                            _ => "?",
                        };
                        details.push(Line::from(vec![
                            Span::styled(
                                format!("  [{}] ", icon),
                                Style::default().fg(conductor_theme.fg_secondary),
                            ),
                            Span::styled(
                                &dep.task_id,
                                Style::default().fg(conductor_theme.fg_primary),
                            )
                        ]));
                    }
                }

                details
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
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

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

fn render_prompt_view(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, _conductor_theme: &ConductorTheme, theme: &crate::theme::Theme) {
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
                    Line::from(Span::styled("No prompt preview available for tracks. Select a task.", Style::default().fg(theme.muted).italic())),
                ]
            }
            super::model::SelectableItem::Task { id, title, .. } => {
                vec![
                    Line::from(vec![
                        Span::styled("Task: ", Style::default().fg(theme.accent)),
                        Span::styled(title, Style::default().bold()),
                    ]),
                    Line::from(vec![
                        Span::styled("ID:   ", Style::default().fg(theme.muted)),
                        Span::styled(id, Style::default().fg(theme.muted)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Goal:", Style::default().fg(theme.accent).underlined())),
                    Line::from(Span::styled("Implement this task using the current context.", Style::default().fg(theme.fg))),
                    Line::from(""),
                    Line::from(Span::styled("Context Injection:", Style::default().fg(theme.accent).underlined())),
                    Line::from(Span::styled("• Project analysis (ast, callgraph)", Style::default().fg(theme.fg))),
                    Line::from(Span::styled("• Recent iteration history", Style::default().fg(theme.fg))),
                    Line::from(Span::styled("• Relevant memories from database", Style::default().fg(theme.fg))),
                ]
            }
        }
    } else {
        vec![Line::from(Span::styled("No item selected.", Style::default().fg(theme.muted)))]
    };

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
