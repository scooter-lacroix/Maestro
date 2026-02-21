use super::model::DetailsViewMode;
use super::pane::ConductorPane;
use super::theme::ConductorTheme;
use leindex_core::orchestrate::model::{IterationStatus, SessionStatus, TrackStatus};
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
        DetailsViewMode::Parallel => {
            render_parallel_view(frame, area, pane, &conductor_theme, theme)
        }
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
        })
        .style(Style::default().bg(theme.panel_bg));

    let items = pane.get_selectable_items();
    let selected_item = if items.is_empty() {
        None
    } else {
        Some(&items[pane.selected_index.min(items.len() - 1)])
    };

    let details_text = if let Some(item) = selected_item {
        match item {
            super::model::SelectableItem::Track { id, .. } => {
                let tool_name = pane
                    .state
                    .active_agent
                    .as_ref()
                    .map(|a| a.tool.clone())
                    .unwrap_or_else(|| "claude".to_string());
                let start_cmd = pane
                    .get_start_command(Some(&tool_name), false, false)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "// No track selected".to_string());
                let pause_cmd = pane
                    .get_pause_command()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "// No track selected".to_string());
                let resume_cmd = pane
                    .get_resume_command()
                    .map(|c| c.to_string())
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
                                SessionStatus::Running | SessionStatus::Pausing => {
                                    conductor_theme.status_success
                                }
                                SessionStatus::Paused => conductor_theme.status_warning,
                                SessionStatus::Completed => conductor_theme.status_info,
                                SessionStatus::Failed
                                | SessionStatus::Interrupted
                                | SessionStatus::Stopping => conductor_theme.status_error,
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
                        Span::styled(id, Style::default().fg(conductor_theme.fg_muted)),
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
                ];

                // Add actionable/blocked indicators
                if *is_blocked {
                    details.push(Line::from(vec![
                        Span::styled(
                            "State: ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled("⊘ BLOCKED", Style::default().fg(theme.error).bold()),
                    ]));
                } else if *is_actionable && matches!(status, TrackStatus::Pending) {
                    details.push(Line::from(vec![
                        Span::styled(
                            "State: ",
                            Style::default().fg(conductor_theme.accent_secondary),
                        ),
                        Span::styled(
                            "● ACTIONABLE",
                            Style::default().fg(conductor_theme.task_active).bold(),
                        ),
                    ]));
                }

                // Add description if available
                if !description.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Description:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    // Wrap long descriptions
                    for line in description.lines() {
                        details.push(Line::from(vec![
                            Span::styled("  ", Style::default().fg(conductor_theme.fg_muted)),
                            Span::styled(line, Style::default().fg(conductor_theme.fg_secondary)),
                        ]));
                    }
                }

                // Add notes if available
                if !notes.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(Span::styled(
                        "Notes:",
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .underlined(),
                    )));
                    for line in notes.lines() {
                        details.push(Line::from(vec![
                            Span::styled("  ", Style::default().fg(conductor_theme.fg_muted)),
                            Span::styled(
                                line,
                                Style::default().fg(conductor_theme.fg_secondary).italic(),
                            ),
                        ]));
                    }
                }

                // Dependencies section
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
                                &dep.task_id,
                                Style::default().fg(conductor_theme.fg_primary),
                            ),
                        ]));
                    }
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
            super::model::SelectableItem::Task {
                id,
                title,
                description,
                notes,
                is_blocked,
                is_actionable,
                dependencies,
                dependency_statuses,
                ..
            } => {
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

                // Add state indicator
                if *is_blocked {
                    lines.push(Line::from(vec![
                        Span::styled("State: ", Style::default().fg(theme.muted)),
                        Span::styled("⊘ BLOCKED", Style::default().fg(theme.error)),
                    ]));
                } else if *is_actionable {
                    lines.push(Line::from(vec![
                        Span::styled("State: ", Style::default().fg(theme.muted)),
                        Span::styled("● ACTIONABLE", Style::default().fg(theme.accent)),
                    ]));
                }

                // Goal section - use description if available
                lines.push(Line::from(""));
                if !description.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "Goal:",
                        Style::default().fg(theme.accent).underlined(),
                    )));
                    for line in description.lines().take(10) {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(theme.fg),
                        )));
                    }
                    if description.lines().count() > 10 {
                        lines.push(Line::from(Span::styled(
                            "  ... (truncated)",
                            Style::default().fg(theme.muted).italic(),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Goal:",
                        Style::default().fg(theme.accent).underlined(),
                    )));
                    lines.push(Line::from(Span::styled(
                        "  (No description provided)",
                        Style::default().fg(theme.muted).italic(),
                    )));
                }

                // Notes section if available
                if !notes.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Notes:",
                        Style::default().fg(theme.accent).underlined(),
                    )));
                    for line in notes.lines().take(5) {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(theme.fg).italic(),
                        )));
                    }
                }

                // Dependencies section
                if !dependencies.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Dependencies:",
                        Style::default().fg(theme.accent).underlined(),
                    )));
                    for (idx, dep) in dependencies.iter().enumerate() {
                        let status_icon = match dependency_statuses.get(idx) {
                            Some(super::model::DependencyStatus::Completed) => "✓",
                            Some(super::model::DependencyStatus::Blocked) => "⊘",
                            Some(super::model::DependencyStatus::Pending) => "○",
                            None => "?",
                        };
                        let status_color = match dependency_statuses.get(idx) {
                            Some(super::model::DependencyStatus::Completed) => theme.accent,
                            Some(super::model::DependencyStatus::Blocked) => theme.error,
                            _ => theme.muted,
                        };
                        lines.push(Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(status_icon, Style::default().fg(status_color)),
                            Span::styled(
                                format!(" {}", dep.task_id),
                                Style::default().fg(theme.fg),
                            ),
                        ]));
                    }
                }

                // Context injection section
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Context Injection:",
                    Style::default().fg(theme.accent).underlined(),
                )));
                lines.push(Line::from(Span::styled(
                    "• Project analysis (ast, callgraph)",
                    Style::default().fg(theme.fg),
                )));
                lines.push(Line::from(Span::styled(
                    "• Recent iteration history",
                    Style::default().fg(theme.fg),
                )));

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
                    lines.push(Line::from(Span::styled(
                        "• Relevant memories from database",
                        Style::default().fg(theme.fg),
                    )));
                }

                // Token estimate
                let estimated_tokens = 2000 + (description.len() + notes.len()) / 4; // Rough estimate: ~4 chars per token
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Estimated tokens: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!("~{}", estimated_tokens),
                        Style::default().fg(theme.accent),
                    ),
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

fn render_parallel_view(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    conductor_theme: &ConductorTheme,
    theme: &crate::theme::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Parallel Execution ")
        .border_style(if pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Split into workers section (top) and merge queue (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // Workers
            Constraint::Percentage(40), // Merge Queue
        ])
        .split(inner_area);

    // Render workers section
    let workers_block = Block::default()
        .borders(Borders::TOP)
        .title(" Workers ")
        .border_style(Style::default().fg(conductor_theme.fg_muted));

    let mut workers_lines = vec![
        Line::from(Span::styled(
            "Parallel execution status and worker states",
            Style::default().fg(conductor_theme.fg_muted).italic(),
        )),
        Line::from(""),
    ];

    // Check if we have parallel view data
    if let Some(ref group_info) = pane.parallel_view.group_info {
        // Group status header
        let status_color = match group_info.status {
            leindex_core::orchestrate::model::ParallelStatus::Running => {
                conductor_theme.status_success
            }
            leindex_core::orchestrate::model::ParallelStatus::Merging => {
                conductor_theme.status_warning
            }
            leindex_core::orchestrate::model::ParallelStatus::Complete => {
                conductor_theme.status_info
            }
            leindex_core::orchestrate::model::ParallelStatus::Paused => {
                conductor_theme.status_warning
            }
            leindex_core::orchestrate::model::ParallelStatus::Idle => conductor_theme.fg_muted,
        };

        workers_lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(conductor_theme.fg_muted)),
            Span::styled(
                format!("{:?}", group_info.status),
                Style::default().fg(status_color).bold(),
            ),
        ]));

        workers_lines.push(Line::from(vec![
            Span::styled("Group ID: ", Style::default().fg(conductor_theme.fg_muted)),
            Span::styled(
                &group_info.group_id,
                Style::default().fg(conductor_theme.fg_secondary),
            ),
        ]));

        workers_lines.push(Line::from(""));
        workers_lines.push(Line::from(Span::styled(
            "Active Workers:",
            Style::default()
                .fg(conductor_theme.accent_primary)
                .underlined(),
        )));

        // Render each worker
        for worker in &group_info.workers {
            let status_icon = super::parallel_view::ParallelView::get_status_icon(&worker.status);
            let status_color = match worker.status {
                leindex_core::orchestrate::model::WorkerStatus::Idle => conductor_theme.fg_muted,
                leindex_core::orchestrate::model::WorkerStatus::Working => {
                    conductor_theme.status_success
                }
                leindex_core::orchestrate::model::WorkerStatus::Waiting => {
                    conductor_theme.status_warning
                }
                leindex_core::orchestrate::model::WorkerStatus::Complete => {
                    conductor_theme.status_info
                }
                leindex_core::orchestrate::model::WorkerStatus::Error => {
                    conductor_theme.status_error
                }
            };

            let is_selected = pane.parallel_view.selected_worker.as_ref() == Some(&worker.id);
            let selector = if is_selected { "▶" } else { " " };

            workers_lines.push(Line::from(vec![
                Span::styled(
                    format!("{} {} ", selector, status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("{}: ", worker.id),
                    Style::default().fg(conductor_theme.fg_primary),
                ),
                Span::styled(
                    format!("{:.0}%", worker.progress * 100.0),
                    Style::default().fg(conductor_theme.fg_secondary),
                ),
            ]));

            // Show latest output line for the worker
            if let Some(last_output) = worker.output.last() {
                let truncated = if last_output.chars().count() > 60 {
                    format!(
                        "  └─ {}...",
                        last_output.chars().take(57).collect::<String>()
                    )
                } else {
                    format!("  └─ {}", last_output)
                };
                workers_lines.push(Line::from(Span::styled(
                    truncated,
                    Style::default().fg(conductor_theme.fg_muted).italic(),
                )));
            }
        }
    } else {
        workers_lines.push(Line::from(Span::styled(
            "No parallel execution active.",
            Style::default().fg(conductor_theme.fg_muted).italic(),
        )));
        workers_lines.push(Line::from(""));
        workers_lines.push(Line::from(Span::styled(
            "Start a parallel track to see worker status here.",
            Style::default().fg(conductor_theme.fg_muted),
        )));
    }

    let workers_para = Paragraph::new(workers_lines)
        .block(workers_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(workers_para, chunks[0]);

    // Render merge queue section
    let merge_block = Block::default()
        .borders(Borders::TOP)
        .title(" Merge Queue ")
        .border_style(Style::default().fg(conductor_theme.fg_muted));

    let mut merge_lines = vec![];

    if let Some(ref group_info) = pane.parallel_view.group_info {
        if group_info.merge_queue.is_empty() {
            merge_lines.push(Line::from(Span::styled(
                "No pending merges",
                Style::default().fg(conductor_theme.fg_muted).italic(),
            )));
        } else {
            for entry in &group_info.merge_queue {
                let status_icon = match entry.status {
                    leindex_core::orchestrate::model::MergeStatus::Waiting => "○",
                    leindex_core::orchestrate::model::MergeStatus::Merging => "◐",
                    leindex_core::orchestrate::model::MergeStatus::Conflicted => "⚠",
                    leindex_core::orchestrate::model::MergeStatus::Complete => "●",
                };
                let status_color = match entry.status {
                    leindex_core::orchestrate::model::MergeStatus::Waiting => {
                        conductor_theme.fg_muted
                    }
                    leindex_core::orchestrate::model::MergeStatus::Merging => {
                        conductor_theme.status_warning
                    }
                    leindex_core::orchestrate::model::MergeStatus::Conflicted => {
                        conductor_theme.status_error
                    }
                    leindex_core::orchestrate::model::MergeStatus::Complete => {
                        conductor_theme.status_success
                    }
                };

                merge_lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(
                        format!("{} → {}", entry.worker_id, entry.task_id),
                        Style::default().fg(conductor_theme.fg_primary),
                    ),
                ]));

                // Show conflicts if any
                if !entry.conflicts.is_empty() {
                    for conflict in &entry.conflicts {
                        merge_lines.push(Line::from(vec![
                            Span::styled(
                                "    ⚠ ",
                                Style::default().fg(conductor_theme.status_error),
                            ),
                            Span::styled(
                                &conflict.file,
                                Style::default().fg(conductor_theme.fg_secondary),
                            ),
                        ]));
                    }
                }
            }
        }
    } else {
        merge_lines.push(Line::from(Span::styled(
            "No merge queue",
            Style::default().fg(conductor_theme.fg_muted).italic(),
        )));
    }

    let merge_para = Paragraph::new(merge_lines)
        .block(merge_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(merge_para, chunks[1]);
}
