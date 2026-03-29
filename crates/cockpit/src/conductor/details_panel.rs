use super::model::{DetailsViewMode, RuntimeLogEntry, RuntimeLogLevel, SelectableItem};
use super::pane::ConductorPane;
use super::theme::ConductorTheme;
use leindex_core::orchestrate::model::{IterationStatus, SessionStatus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn render_details_panel(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let conductor_theme = ConductorTheme::default();
    match pane.details_mode {
        DetailsViewMode::Details => render_details_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Output => render_output_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Prompt => render_prompt_view(frame, area, pane, &conductor_theme, theme),
        DetailsViewMode::Parallel => render_output_view(frame, area, pane, &conductor_theme, theme),
    }
}

fn render_details_view(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    conductor_theme: &ConductorTheme,
    theme: &crate::theme::Theme,
) {
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
        Some(items[pane.selected_index.min(items.len() - 1)].clone())
    };

    let upcoming_tasks = summarize_upcoming_tasks(&items, pane.state.current_task.as_deref(), 4);
    let recent_notes = summarize_runtime_logs(&pane.state.runtime_logs, 4);

    let details_text = if let Some(item) = selected_item {
        match item {
            SelectableItem::Track { id, .. } => {
                let tool_name = pane
                    .state
                    .active_agent
                    .as_ref()
                    .map(|a| a.tool.clone())
                    .unwrap_or_else(|| "claude".to_string());
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
                        Span::styled(
                            "Track: ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(id, Style::default().bold().fg(conductor_theme.fg_primary)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Mode:  ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            format!("{:?}", pane.loop_mode),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Status:",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            format!(" {:?}", pane.session_status),
                            Style::default().fg(match pane.session_status {
                                SessionStatus::Running => conductor_theme.status_success,
                                SessionStatus::Paused => conductor_theme.status_warning,
                                SessionStatus::Completed => conductor_theme.status_info,
                                SessionStatus::Failed | SessionStatus::Interrupted => {
                                    conductor_theme.status_error
                                }
                                _ => conductor_theme.fg_muted,
                            }),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Loop:  ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            format!("Iteration {}", pane.state.current_iteration),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Agent Activity:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )),
                ];

                if let Some(agent) = &pane.state.active_agent {
                    details.push(Line::from(vec![
                        Span::styled("  Tool:  ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(
                            &agent.tool,
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]));
                    if let Some(model) = &agent.model {
                        details.push(Line::from(vec![
                            Span::styled(
                                "  Model: ",
                                Style::default().fg(conductor_theme.fg_muted),
                            ),
                            Span::styled(model, Style::default().fg(conductor_theme.fg_secondary)),
                        ]));
                    }
                    details.push(Line::from(vec![
                        Span::styled("  Since: ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(
                            agent.since.format("%H:%M:%S").to_string(),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]));
                } else {
                    details.push(Line::from(Span::styled(
                        "  No active agent",
                        Style::default().fg(conductor_theme.fg_muted),
                    )));
                }

                if let Some(rl) = &pane.state.rate_limit {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled(
                            "  ⚠️ RATE LIMITED",
                            Style::default().fg(conductor_theme.status_error).bold(),
                        ),
                        Span::styled(
                            format!(" (Retry #{})", rl.retry_count),
                            Style::default().fg(conductor_theme.fg_muted),
                        ),
                    ]));
                    if let Some(backoff) = rl.backoff_until {
                        let remaining = backoff
                            .signed_duration_since(chrono::Utc::now())
                            .num_seconds();
                        if remaining > 0 {
                            details.push(Line::from(vec![
                                Span::styled(
                                    "  Backoff: ",
                                    Style::default().fg(conductor_theme.fg_muted),
                                ),
                                Span::styled(
                                    format!("{}s", remaining),
                                    Style::default().fg(theme.warning),
                                ),
                            ]));
                        }
                    }
                }

                details.extend(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Commands:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )),
                    Line::from(vec![
                        Span::styled(
                            "  [s] Start: ",
                            Style::default().fg(conductor_theme.fg_muted),
                        ),
                        Span::styled(start_cmd, Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "  [p] Pause:  ",
                            Style::default().fg(conductor_theme.fg_muted),
                        ),
                        Span::styled(pause_cmd, Style::default().fg(conductor_theme.fg_secondary)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "  [r] Resume: ",
                            Style::default().fg(conductor_theme.fg_muted),
                        ),
                        Span::styled(
                            resume_cmd,
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                ]);

                let progress = if pane.state.total_tasks == 0 {
                    0.0
                } else {
                    pane.state.tasks_completed as f64 / pane.state.total_tasks as f64
                };
                let bar_width = 18usize;
                let filled = ((bar_width as f64) * progress).round() as usize;
                let progress_bar = format!(
                    "{}{}",
                    "█".repeat(filled.min(bar_width)),
                    "░".repeat(bar_width.saturating_sub(filled.min(bar_width)))
                );
                details.extend(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Ralph Loop:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )),
                    Line::from(vec![
                        Span::styled("  Progress ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(progress_bar, Style::default().fg(theme.accent)),
                        Span::styled(
                            format!(" {:>3.0}%", progress * 100.0),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Loops ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(
                            format!("{}", pane.state.current_iteration),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                        Span::styled(
                            "  Active Task ",
                            Style::default().fg(conductor_theme.fg_muted),
                        ),
                        Span::styled(
                            pane.state
                                .current_task
                                .clone()
                                .unwrap_or_else(|| "idle".to_string()),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                ]);

                if !upcoming_tasks.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Task Queue:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    details.extend(upcoming_tasks.clone());
                }

                if !recent_notes.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Recent Loop Notes:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    details.extend(recent_notes.clone());
                }

                // Iteration Summary Section
                if let Some(last_log) = pane.state.iteration_logs.last() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Last Iteration Summary:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    details.push(Line::from(vec![
                        Span::styled(
                            format!("  Iter {}: ", last_log.iteration),
                            Style::default().fg(conductor_theme.fg_muted),
                        ),
                        Span::styled(
                            format!("{:?}", last_log.status),
                            Style::default().fg(match last_log.status {
                                IterationStatus::Completed => conductor_theme.status_success,
                                IterationStatus::Failed => conductor_theme.status_error,
                                _ => conductor_theme.fg_secondary,
                            }),
                        ),
                    ]));

                    if !last_log.output.is_empty() {
                        let summary = if last_log.output.len() > 200 {
                            format!("{}...", &last_log.output[..200].replace('\n', " "))
                        } else {
                            last_log.output.replace('\n', " ")
                        };
                        details.push(Line::from(vec![
                            Span::styled(
                                "  Output: ",
                                Style::default().fg(conductor_theme.fg_muted),
                            ),
                            Span::styled(
                                summary,
                                Style::default().fg(conductor_theme.fg_secondary).italic(),
                            ),
                        ]));
                    }
                }

                // Memories Section
                if !pane.state.track_memories.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Recent Memories:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    for memory in &pane.state.track_memories {
                        let prefix = format!(" • [{}] ", memory.category);
                        details.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(theme.accent)),
                            Span::styled(
                                &memory.content,
                                Style::default().fg(conductor_theme.fg_secondary),
                            ),
                        ]));
                    }
                }

                details
            }
            SelectableItem::Task {
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
                        Span::styled(
                            "Task:  ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            title,
                            Style::default().bold().fg(conductor_theme.fg_primary),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "ID:    ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(id.clone(), Style::default().fg(conductor_theme.fg_muted)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Status:",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            format!(" {:?}", status),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Ready: ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            if is_actionable {
                                "actionable"
                            } else if is_blocked {
                                "blocked"
                            } else {
                                "waiting"
                            },
                            Style::default().fg(if is_actionable {
                                conductor_theme.status_success
                            } else if is_blocked {
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
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
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
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
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
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
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
                                dep.task_id.clone(),
                                Style::default().fg(conductor_theme.fg_primary),
                            ),
                        ]));
                    }
                }

                let progress = if pane.state.total_tasks == 0 {
                    0.0
                } else {
                    pane.state.tasks_completed as f64 / pane.state.total_tasks as f64
                };
                let bar_width = 18usize;
                let filled = ((bar_width as f64) * progress).round() as usize;
                let progress_bar = format!(
                    "{}{}",
                    "█".repeat(filled.min(bar_width)),
                    "░".repeat(bar_width.saturating_sub(filled.min(bar_width)))
                );
                details.extend(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Ralph Loop:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )),
                    Line::from(vec![
                        Span::styled("  Progress ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(progress_bar, Style::default().fg(theme.accent)),
                        Span::styled(
                            format!(" {:>3.0}%", progress * 100.0),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Loops ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(
                            format!("{}", pane.state.current_iteration),
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                        Span::styled("  Active ", Style::default().fg(conductor_theme.fg_muted)),
                        Span::styled(
                            if pane.state.current_task.as_deref() == Some(id.as_str()) {
                                "yes".to_string()
                            } else {
                                "no".to_string()
                            },
                            Style::default().fg(conductor_theme.fg_secondary),
                        ),
                    ]),
                ]);

                if !upcoming_tasks.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Task Queue:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    details.extend(upcoming_tasks);
                }

                if !recent_notes.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Recent Loop Notes:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    details.extend(recent_notes);
                }

                details
            }
        }
    } else {
        vec![Line::from(Span::styled(
            "No item selected.",
            Style::default().fg(conductor_theme.fg_muted),
        ))]
    };

    let paragraph = Paragraph::new(details_text)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_output_view(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    conductor_theme: &ConductorTheme,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Live Output ")
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let output_text: Vec<Line> = pane
        .iteration_output
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                line.as_str(),
                Style::default().fg(conductor_theme.fg_secondary),
            ))
        })
        .collect();

    let output = Paragraph::new(output_text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((pane.output_scroll, 0));

    frame.render_widget(output, area);
}

fn summarize_upcoming_tasks(
    items: &[SelectableItem],
    current_task: Option<&str>,
    max_items: usize,
) -> Vec<Line<'static>> {
    items
        .iter()
        .filter_map(|item| match item {
            SelectableItem::Task {
                id,
                title,
                status,
                is_actionable,
                ..
            } if *status != leindex_core::orchestrate::model::TrackStatus::Completed => {
                let marker = if current_task == Some(id.as_str()) {
                    "►"
                } else if *is_actionable {
                    "•"
                } else {
                    "○"
                };
                let text = format!("  {} {} ({:?})", marker, title, status);
                Some(Line::from(text))
            }
            _ => None,
        })
        .take(max_items)
        .collect()
}

fn summarize_runtime_logs(logs: &[RuntimeLogEntry], max_items: usize) -> Vec<Line<'static>> {
    logs.iter()
        .rev()
        .take(max_items)
        .rev()
        .map(|entry| {
            let prefix = match entry.level {
                RuntimeLogLevel::Info => "[info]",
                RuntimeLogLevel::Success => "[ok]",
                RuntimeLogLevel::Warning => "[warn]",
                RuntimeLogLevel::Error => "[err]",
            };
            let task = entry.task_id.as_deref().unwrap_or("track");
            let mut summary = entry.summary.replace('\n', " ");
            if summary.len() > 92 {
                summary.truncate(89);
                summary.push_str("...");
            }
            Line::from(format!("  {} {} {}", prefix, task, summary))
        })
        .collect()
}

fn render_prompt_view(
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
                    Line::from(Span::styled(
                        "Goal:",
                        Style::default().fg(theme.accent).underlined(),
                    )),
                    Line::from(Span::styled(
                        "Implement this task using the current context.",
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Context Injection:",
                        Style::default().fg(theme.accent).underlined(),
                    )),
                    Line::from(Span::styled(
                        "• Project analysis (ast, callgraph)",
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(Span::styled(
                        "• Recent iteration history",
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(Span::styled(
                        "• Relevant memories from database",
                        Style::default().fg(theme.fg),
                    )),
                ]
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
