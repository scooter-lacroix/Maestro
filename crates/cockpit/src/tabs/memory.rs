//! Memory tab rendering for Cockpit TUI
//!
//! Provides expandable memory entries with full metadata display,
//! vector visualization, and agent access tracking.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::maesterclaw::clamp_flash;
use crate::state::InputMode;
use std::collections::BTreeMap;

/// Maximum content preview length in collapsed view
const PREVIEW_LEN: usize = 60;

/// Character width for detail panel
const DETAIL_PANEL_WIDTH_PERCENT: u16 = 45;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemoryRelationKind {
    NexusLink,
    SessionLink,
    TrackLink,
    ProjectLink,
    AgentLink,
    TagLink,
    CategoryLink,
    Other,
}

fn relation_kind_label(kind: MemoryRelationKind) -> &'static str {
    match kind {
        MemoryRelationKind::NexusLink => "nexus link",
        MemoryRelationKind::SessionLink => "session link",
        MemoryRelationKind::TrackLink => "track link",
        MemoryRelationKind::ProjectLink => "project link",
        MemoryRelationKind::AgentLink => "agent link",
        MemoryRelationKind::TagLink => "tag link",
        MemoryRelationKind::CategoryLink => "category link",
        MemoryRelationKind::Other => "other",
    }
}

fn relation_style(kind: MemoryRelationKind) -> (Color, &'static str) {
    match kind {
        MemoryRelationKind::NexusLink => (Color::Magenta, "N"),
        MemoryRelationKind::SessionLink => (Color::Cyan, "S"),
        MemoryRelationKind::TrackLink => (Color::Blue, "T"),
        MemoryRelationKind::ProjectLink => (Color::Green, "P"),
        MemoryRelationKind::AgentLink => (Color::Yellow, "A"),
        MemoryRelationKind::TagLink => (Color::LightBlue, "#"),
        MemoryRelationKind::CategoryLink => (Color::LightYellow, "C"),
        MemoryRelationKind::Other => (Color::DarkGray, "."),
    }
}

fn classify_relation(
    selected: &crate::state::MemoryInfo,
    memory: &crate::state::MemoryInfo,
) -> MemoryRelationKind {
    if selected.id == memory.id {
        return MemoryRelationKind::NexusLink;
    }

    if selected.related_memory_ids.contains(&memory.id)
        || memory.related_memory_ids.contains(&selected.id)
    {
        return MemoryRelationKind::NexusLink;
    }

    if selected.session_id.is_some() && selected.session_id == memory.session_id {
        return MemoryRelationKind::SessionLink;
    }

    if selected.track_id.is_some() && selected.track_id == memory.track_id {
        return MemoryRelationKind::TrackLink;
    }

    if selected.project_id.is_some() && selected.project_id == memory.project_id {
        return MemoryRelationKind::ProjectLink;
    }

    if !selected.stored_by.trim().is_empty()
        && selected.stored_by.eq_ignore_ascii_case(&memory.stored_by)
        && !matches!(
            selected.stored_by.to_ascii_lowercase().as_str(),
            "unknown" | "nexus" | "maestro"
        )
    {
        return MemoryRelationKind::AgentLink;
    }

    if selected
        .tags
        .iter()
        .any(|tag| memory.tags.iter().any(|candidate| candidate == tag))
    {
        return MemoryRelationKind::TagLink;
    }

    if selected.category == memory.category {
        return MemoryRelationKind::CategoryLink;
    }

    MemoryRelationKind::Other
}

fn relation_rank(kind: MemoryRelationKind) -> u8 {
    match kind {
        MemoryRelationKind::NexusLink => 100,
        MemoryRelationKind::SessionLink => 90,
        MemoryRelationKind::TrackLink => 80,
        MemoryRelationKind::ProjectLink => 70,
        MemoryRelationKind::AgentLink => 60,
        MemoryRelationKind::TagLink => 40,
        MemoryRelationKind::CategoryLink => 30,
        MemoryRelationKind::Other => 10,
    }
}

pub fn graph_navigation_targets(app: &App) -> Vec<usize> {
    if app.memories.is_empty() {
        return Vec::new();
    }

    let selected_idx = app
        .memory_state
        .selected()
        .unwrap_or(0)
        .min(app.memories.len().saturating_sub(1));
    let selected = &app.memories[selected_idx];
    let mut ranked = Vec::new();

    for (idx, memory) in app.memories.iter().enumerate() {
        if idx == selected_idx {
            continue;
        }
        let kind = classify_relation(selected, memory);
        ranked.push((idx, relation_rank(kind), kind));
    }

    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| app.memories[a.0].category.cmp(&app.memories[b.0].category))
            .then_with(|| app.memories[a.0].content.cmp(&app.memories[b.0].content))
    });

    let mut targets = vec![selected_idx];
    targets.extend(ranked.into_iter().map(|(idx, _, _)| idx));
    targets
}

pub fn render_memory(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    // Check if we're in memory creation mode
    let is_creating = app.input_mode == InputMode::NewMemoryContent
        || app.input_mode == InputMode::NewMemoryCategory;

    // Check if we have suggestions to show
    let has_suggestions = !app.hot_cache.is_empty();

    // Determine layout based on whether we're viewing details or creating
    let (search_area, hint_area, content_area, input_area, detail_area) = if is_creating {
        // Split: search (1 line), content input (3 lines), memories (rest)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(area);
        (chunks[0], None, Some(chunks[2]), Some(chunks[1]), None)
    } else if app.input_mode == InputMode::MemoryDetail
        || app.input_mode == InputMode::MemoryDetailFocus
        || app.memory_graph_enabled
    {
        // Two-pane layout: list (left), detail (right)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100 - DETAIL_PANEL_WIDTH_PERCENT),
                Constraint::Percentage(DETAIL_PANEL_WIDTH_PERCENT),
            ])
            .split(chunks[1]);

        (
            chunks[0],
            None,
            Some(main_chunks[0]),
            None,
            Some(main_chunks[1]),
        )
    } else {
        // Original layout with optional suggestion hint: search (3 lines), hint (optional, 2 lines), memories (rest)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                if has_suggestions {
                    Constraint::Length(2)
                } else {
                    Constraint::Length(0)
                },
                Constraint::Min(0),
            ])
            .split(area);
        (
            chunks[0],
            if has_suggestions {
                Some(chunks[1])
            } else {
                None
            },
            Some(chunks[2]),
            None,
            None,
        )
    };

    // Render search bar
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Memory Search (Ctrl+F search, Ctrl+L clear, r refresh, n new, Enter details, g graph) ")
        .title_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.panel_bg));

    let search_text = if app.input_mode == InputMode::MemorySearch {
        format!("{}|", app.memory_query)
    } else {
        app.memory_query.clone()
    };
    frame.render_widget(Paragraph::new(search_text).block(search_block), search_area);

    // Render suggestion hints if available
    if let Some(hint_area) = hint_area {
        render_suggestion_hints(frame, hint_area, app);
    }

    // Render memory creation input if active
    if let Some(input_area) = input_area {
        let input_title = if app.input_mode == InputMode::NewMemoryContent {
            " New Memory Content (Enter to continue, Esc to cancel) "
        } else {
            " Category (general, knowledge, preference, spec, fact, pattern, decision, context, temp, observation) "
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(input_title)
            .title_style(Style::default().fg(theme.accent_alt))
            .border_style(Style::default().fg(theme.accent));

        let input_text = if app.input_mode == InputMode::NewMemoryContent {
            format!("{}|", app.new_memory_content)
        } else {
            format!("{}|", app.new_memory_category)
        };

        let input_paragraph = Paragraph::new(input_text)
            .block(input_block)
            .style(Style::default().fg(Color::White));
        frame.render_widget(input_paragraph, input_area);
    }

    // Render memories list
    if let Some(list_area) = content_area {
        render_memory_list(frame, list_area, app);
    }

    // Render detail panel if active
    if let Some(detail_area) = detail_area {
        render_memory_detail(frame, detail_area, app);
    }
}

/// Render the memory list with expandable entries
fn render_memory_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(
            " Memory Results ({}) ",
            if app.input_mode == InputMode::MemoryDetailFocus {
                "graph focused, Tab returns to list"
            } else if app.memory_graph_enabled {
                "Tab focuses graph, g hides graph"
            } else {
                "Enter to view details"
            }
        ))
        .title_style(Style::default().fg(theme.accent_alt))
        .style(Style::default().bg(theme.panel_bg));

    if app.memories.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No memories found."),
            Line::from(""),
            Line::from("  Tip: press 'r' to import system-wide memories."),
            Line::from("  Tip: press 'n' to create a new memory."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = app
        .memories
        .iter()
        .map(|m| {
            // Create expand icon
            let expand_icon = if m.is_expanded { " v " } else { " > " };

            // Create preview content (truncated if too long)
            let preview = if m.content.chars().count() > PREVIEW_LEN {
                format!("{}...", m.content.chars().take(PREVIEW_LEN).collect::<String>())
            } else {
                m.content.clone()
            };

            // Create category badge with color
            let (category_color, category_icon) = category_style(&m.category);

            // Create importance indicator
            let importance_indicator = match m.importance.as_str() {
                "critical" => " [!]",
                "high" => " [*]",
                _ => "",
            };

            // Build the line based on expansion state
            if m.is_expanded {
                // Expanded view: show more details
                let mut lines = vec![Line::from(vec![
                    Span::styled(expand_icon, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("[{}{}] ", category_icon, m.category),
                        Style::default().fg(category_color).bold(),
                    ),
                    Span::styled(preview, Style::default().fg(Color::White)),
                    Span::styled(importance_indicator, Style::default().fg(Color::Red)),
                ])];

                // Add summary line if available
                if let Some(ref summary) = m.summary {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Summary: {}", summary),
                            Style::default().fg(Color::DarkGray).italic(),
                        ),
                    ]));
                }

                // Add metadata line
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(
                        format!(
                            "Created: {} | Access: {} times",
                            m.created_at, m.access_count
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                // Add tags if present
                if !m.tags.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Tags: {}", m.tags.join(", ")),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]));
                }

                if !m.stored_by.trim().is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Agent: {}", m.stored_by),
                            Style::default().fg(Color::Magenta),
                        ),
                    ]));
                }

                if let Some(state) = m.nexus_runtime_state.as_deref() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Runtime: {}", state),
                            Style::default().fg(Color::Green),
                        ),
                    ]));
                }

                ListItem::new(lines)
            } else {
                // Collapsed view: single line
                ListItem::new(Line::from(vec![
                    Span::styled(expand_icon, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("[{}{}] ", category_icon, m.category),
                        Style::default().fg(category_color),
                    ),
                    Span::styled(preview, Style::default().fg(Color::White)),
                    Span::styled(importance_indicator, Style::default().fg(Color::Red)),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, area, &mut app.memory_state);
}

/// Render the detail panel with full metadata and vector visualization
fn render_memory_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    // Get selected memory
    let selected_idx = app.memory_state.selected().unwrap_or(0);
    let memory = app.memories.get(selected_idx);

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Memory Details ")
        .title_style(Style::default().fg(theme.accent_alt))
        .style(Style::default().bg(theme.panel_bg));

    let inner_area = detail_block.inner(area);

    if let Some(m) = memory {
        // Split detail area into content and visualization
        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // Content and metadata
                Constraint::Length(10), // Vector visualization or relationship graph
            ])
            .split(inner_area);

        // Render content and metadata
        render_detail_content(frame, detail_chunks[0], m, &theme);

        if app.memory_graph_enabled {
            render_memory_graph(frame, detail_chunks[1], app, &theme);
        } else {
            render_vector_visualization(frame, detail_chunks[1], m, &theme);
        }
    } else {
        let text = Paragraph::new("No memory selected")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(text, inner_area);
    }

    frame.render_widget(detail_block, area);
}

/// Render the content and metadata section of the detail panel
fn render_detail_content(
    frame: &mut Frame,
    area: Rect,
    memory: &crate::state::MemoryInfo,
    theme: &crate::theme::Theme,
) {
    let (category_color, category_icon) = category_style(&memory.category);

    let mut lines = vec![
        // Header with category and importance
        Line::from(vec![
            Span::styled(
                format!("[{}{}] ", category_icon, memory.category),
                Style::default().fg(category_color).bold(),
            ),
            Span::styled(
                format!("[{}]", memory.importance),
                Style::default().fg(importance_color(&memory.importance)),
            ),
            if let Some(score) = memory.similarity_score {
                Span::styled(
                    format!(" [sim: {:.2}]", score),
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        // Full content (wrapped)
        Line::from(Span::styled(
            "Content:",
            Style::default().fg(theme.accent).bold(),
        )),
    ];

    // Wrap content to fit width
    let content_lines = wrap_text(&memory.content, area.width.saturating_sub(2) as usize);
    for line in content_lines {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(Color::White),
        )));
    }

    lines.push(Line::from(""));

    // Summary if available
    if let Some(ref summary) = memory.summary {
        lines.push(Line::from(Span::styled(
            "Summary:",
            Style::default().fg(theme.accent).bold(),
        )));
        lines.push(Line::from(Span::styled(
            summary.clone(),
            Style::default().fg(Color::DarkGray).italic(),
        )));
        lines.push(Line::from(""));
    }

    // Metadata section
    lines.push(Line::from(Span::styled(
        "Metadata:",
        Style::default().fg(theme.accent).bold(),
    )));

    lines.push(Line::from(vec![
        Span::styled("  Created: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&memory.created_at, Style::default().fg(Color::White)),
    ]));

    if let Some(ref expires) = memory.expires_at {
        lines.push(Line::from(vec![
            Span::styled("  Expires: ", Style::default().fg(Color::DarkGray)),
            Span::styled(expires, Style::default().fg(Color::Yellow)),
        ]));
    }

    if let Some(ref accessed) = memory.last_accessed {
        lines.push(Line::from(vec![
            Span::styled("  Last Accessed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(accessed, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Access Count: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", memory.access_count),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Source information
    if let Some(ref source) = memory.source {
        lines.push(Line::from(vec![
            Span::styled("  Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(source, Style::default().fg(Color::White)),
        ]));
    }

    if let Some(ref session_id) = memory.session_id {
        lines.push(Line::from(vec![
            Span::styled("  Session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(session_id, Style::default().fg(Color::Cyan)),
        ]));
    }

    if let Some(ref project_id) = memory.project_id {
        lines.push(Line::from(vec![
            Span::styled("  Project ID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(project_id, Style::default().fg(Color::White)),
        ]));
    }

    if let Some(ref track_id) = memory.track_id {
        lines.push(Line::from(vec![
            Span::styled("  Track ID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(track_id, Style::default().fg(Color::White)),
        ]));
    }

    // Tags
    if !memory.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Tags: ", Style::default().fg(Color::DarkGray)),
            Span::styled(memory.tags.join(", "), Style::default().fg(Color::Green)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Stored By: ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            if memory.stored_by.trim().is_empty() {
                "unknown"
            } else {
                &memory.stored_by
            },
            Style::default().fg(Color::White),
        ),
    ]));

    if memory.nexus_scope.is_some()
        || memory.nexus_runtime_state.is_some()
        || !memory.related_memory_ids.is_empty()
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Nexus Runtime / Subconscious:",
            Style::default().fg(theme.accent).bold(),
        )));

        if let Some(scope) = memory.nexus_scope.as_deref() {
            lines.push(Line::from(vec![
                Span::styled("  Scope: ", Style::default().fg(Color::DarkGray)),
                Span::styled(scope, Style::default().fg(Color::Green)),
            ]));
        }

        if let Some(runtime_state) = memory.nexus_runtime_state.as_deref() {
            lines.push(Line::from(vec![
                Span::styled("  Runtime: ", Style::default().fg(Color::DarkGray)),
                Span::styled(runtime_state, Style::default().fg(Color::Magenta)),
            ]));
        }

        if !memory.related_memory_ids.is_empty() {
            let preview = memory
                .related_memory_ids
                .iter()
                .take(12)
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(Line::from(vec![
                Span::styled("  Related IDs: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if memory.related_memory_ids.len() > 12 {
                        format!("{} …", preview)
                    } else {
                        preview
                    },
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }
    }

    // Agent access history
    if !memory.accessed_by.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Accessed By Agents:",
            Style::default().fg(theme.accent).bold(),
        )));
        for agent in &memory.accessed_by {
            lines.push(Line::from(vec![
                Span::styled("  * ", Style::default().fg(Color::DarkGray)),
                Span::styled(agent, Style::default().fg(Color::Magenta)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Access History:",
        Style::default().fg(theme.accent).bold().underlined(),
    )));

    if memory.access_history.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No detailed access history recorded.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for event in &memory.access_history {
            let mut spans = vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    if event.timestamp.trim().is_empty() {
                        "unknown-time".to_string()
                    } else {
                        event.timestamp.clone()
                    },
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(&event.agent_id, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!(" ({})", event.access_type),
                    Style::default().fg(Color::Gray),
                ),
            ];

            if let Some(tool_used) = &event.tool_used {
                spans.push(Span::styled(
                    format!(" via {}", tool_used),
                    Style::default().fg(Color::Cyan),
                ));
            }

            lines.push(Line::from(spans));
        }
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_memory_graph(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Memory Relationships ")
        .title_style(Style::default().fg(theme.accent));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.memories.is_empty() || inner.height == 0 || inner.width == 0 {
        frame.render_widget(
            Paragraph::new("No memories to visualize.").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let targets = graph_navigation_targets(app);
    if targets.is_empty() {
        frame.render_widget(
            Paragraph::new("No graph targets available.")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let selected_idx = app
        .memory_state
        .selected()
        .unwrap_or(0)
        .min(app.memories.len().saturating_sub(1));
    let graph_cursor = app
        .memory_graph_selection
        .min(targets.len().saturating_sub(1));
    let selected = &app.memories[selected_idx];
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("anchor ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected.content.chars().count() > 26 {
                    format!("{}...", selected.content.chars().take(26).collect::<String>())
                } else {
                    selected.content.clone()
                },
                Style::default().fg(Color::Cyan).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("focus ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if app.input_mode == InputMode::MemoryDetailFocus {
                    "graph navigation"
                } else {
                    "list navigation"
                },
                Style::default().fg(Color::White),
            ),
            Span::styled("  Tab ", Style::default().fg(Color::DarkGray)),
            Span::styled("switch", Style::default().fg(Color::Green)),
            Span::styled("  Enter ", Style::default().fg(Color::DarkGray)),
            Span::styled("open", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
    ];

    if let Some(scope) = selected.nexus_scope.as_deref() {
        lines.push(Line::from(vec![
            Span::styled("scope ", Style::default().fg(Color::DarkGray)),
            Span::styled(scope, Style::default().fg(Color::Green)),
        ]));
    }

    if let Some(runtime_state) = selected.nexus_runtime_state.as_deref() {
        lines.push(Line::from(vec![
            Span::styled("runtime ", Style::default().fg(Color::DarkGray)),
            Span::styled(runtime_state, Style::default().fg(Color::Magenta)),
        ]));
    }

    if !selected.related_memory_ids.is_empty() {
        let preview = selected
            .related_memory_ids
            .iter()
            .take(8)
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(vec![
            Span::styled("nexus ids ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected.related_memory_ids.len() > 8 {
                    format!("{} …", preview)
                } else {
                    preview
                },
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    lines.push(Line::from(""));

    let mut related_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for idx in targets.into_iter().skip(1) {
        let memory = &app.memories[idx];
        let relation = relation_kind_label(classify_relation(selected, memory)).to_string();
        related_groups.entry(relation).or_default().push(idx);
    }

    for (relation, indices) in related_groups {
        let kind = match relation.as_str() {
            "nexus link" => MemoryRelationKind::NexusLink,
            "session link" => MemoryRelationKind::SessionLink,
            "track link" => MemoryRelationKind::TrackLink,
            "project link" => MemoryRelationKind::ProjectLink,
            "agent link" => MemoryRelationKind::AgentLink,
            "tag link" => MemoryRelationKind::TagLink,
            "category link" => MemoryRelationKind::CategoryLink,
            _ => MemoryRelationKind::Other,
        };
        let (category_color, category_icon) = relation_style(kind);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} {}", category_icon, relation),
                Style::default().fg(category_color).bold(),
            ),
            Span::styled(
                format!(" ({})", indices.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Cache graph navigation targets before the loop
        let graph_nav_targets = graph_navigation_targets(app);

        for idx in indices {
            let memory = &app.memories[idx];
            let cursor_idx = graph_nav_targets
                .iter()
                .position(|candidate| *candidate == idx)
                .unwrap_or(0);
            let is_graph_selected = cursor_idx == graph_cursor;
            let preview = if memory.content.chars().count() > 28 {
                format!("{}...", memory.content.chars().take(28).collect::<String>())
            } else {
                memory.content.clone()
            };
            let style = if is_graph_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled("  ├─ ", Style::default().fg(category_color)),
                Span::styled(
                    format!("[{}] ", memory.id),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(preview, style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Render a simple vector visualization using ASCII art
fn render_vector_visualization(
    frame: &mut Frame,
    area: Rect,
    memory: &crate::state::MemoryInfo,
    theme: &crate::theme::Theme,
) {
    if area.height < 5 || area.width < 10 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Vector Space Visualization ")
        .title_style(Style::default().fg(theme.accent));

    let inner = block.inner(area);

    // Create a simple ASCII visualization of memory clustering
    // This represents the concept of vector similarity without actual embeddings
    let viz_lines = generate_vector_visualization(memory, inner.width, inner.height);

    let para = Paragraph::new(viz_lines).style(Style::default().fg(Color::White));
    frame.render_widget(para, inner);
    frame.render_widget(block, area);
}

/// Generate ASCII art for vector space visualization
fn generate_vector_visualization(
    memory: &crate::state::MemoryInfo,
    width: u16,
    height: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Create a grid representation
    let grid_width = width.saturating_sub(2) as usize;
    let grid_height = height.saturating_sub(2) as usize;

    if grid_width < 10 || grid_height < 3 {
        return vec![Line::from("Too small")];
    }

    // Simulated cluster positions (would be real in production)
    let center_x = grid_width / 2;
    let center_y = grid_height / 2;

    // Build visualization grid
    for y in 0..grid_height {
        let mut row = String::new();
        for x in 0..grid_width {
            // Calculate distance from center
            let dx = (x as i32 - center_x as i32).abs();
            let dy = (y as i32 - center_y as i32).abs();
            let dist = (dx * dx + dy * dy) as f32;

            // Similarity score affects visualization intensity
            let intensity = memory.similarity_score.unwrap_or(0.8);

            if x == center_x && y == center_y {
                // Current memory (highlighted)
                row.push('*');
            } else if dist < (grid_width as f32 * intensity * 0.3).powi(2) {
                // Related memories cluster
                row.push('.');
            } else if dist < (grid_width as f32 * 0.5).powi(2) {
                // Outer cluster
                if x % 4 == 0 && y % 2 == 0 {
                    row.push('o');
                } else {
                    row.push(' ');
                }
            } else {
                row.push(' ');
            }
        }
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Add legend
    if lines.len() > 2 {
        lines.push(Line::from(vec![
            Span::styled("* ", Style::default().fg(Color::Green)),
            Span::styled("Current ", Style::default().fg(Color::DarkGray)),
            Span::styled(". ", Style::default().fg(Color::Cyan)),
            Span::styled("Related ", Style::default().fg(Color::DarkGray)),
            Span::styled("o ", Style::default().fg(Color::DarkGray)),
            Span::styled("Other", Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines
}

/// Get color and icon for a memory category
fn category_style(category: &str) -> (Color, &'static str) {
    match category.to_lowercase().as_str() {
        "general" => (Color::Yellow, ""),
        "knowledge" => (Color::Blue, ""),
        "preference" | "preferences" => (Color::Magenta, ""),
        "specification" | "specifications" => (Color::Cyan, ""),
        "fact" => (Color::Green, ""),
        "pattern" => (Color::LightBlue, ""),
        "decision" => (Color::LightYellow, ""),
        "context" => (Color::Gray, ""),
        "temporary" => (Color::DarkGray, ""),
        "observation" => (Color::LightCyan, ""),
        _ => (Color::White, ""),
    }
}

/// Get color for importance level
fn importance_color(importance: &str) -> Color {
    match importance.to_lowercase().as_str() {
        "critical" => Color::Red,
        "high" => Color::LightRed,
        "normal" => Color::White,
        "low" => Color::DarkGray,
        _ => Color::White,
    }
}

/// Simple text wrapper
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width < 10 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_width {
            if !current_line.is_empty() {
                lines.push(current_line.trim().to_string());
                current_line = String::new();
            }
            // Handle very long words
            if word.len() > max_width {
                for chunk in word.as_bytes().chunks(max_width) {
                    lines.push(String::from_utf8_lossy(chunk).to_string());
                }
            } else {
                current_line = word.to_string();
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Render non-intrusive suggestion hints from the hot cache
fn render_suggestion_hints(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    // Get active suggestions
    let suggestions = app.hot_cache.active_suggestions();

    if suggestions.is_empty() {
        return;
    }

    // Build hint lines with bounded flash intensity
    let hint_lines: Vec<Line> = suggestions
        .iter()
        .take(3) // Max 3 hints
        .enumerate()
        .map(|(i, suggestion)| {
            // Clamp flash intensity to [0.0, 1.0]
            let intensity = clamp_flash(suggestion.flash_intensity);

            // Calculate color based on intensity (non-intrusive)
            let base_color = if intensity > 0.8 {
                Color::Cyan
            } else if intensity > 0.6 {
                Color::LightBlue
            } else {
                Color::Blue
            };

            // Create compact hint line
            Line::from(vec![
                Span::styled(
                    format!("💡[{}] ", i + 1),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    &suggestion.preview,
                    Style::default().fg(base_color).italic(),
                ),
                Span::styled(
                    format!(" ({:.0}%)", suggestion.relevance_score * 100.0),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        })
        .collect();

    // Render hints in a compact block
    let hint_block = Block::default()
        .borders(Borders::BOTTOM | Borders::TOP)
        .border_style(Style::default().fg(theme.muted));

    frame.render_widget(Paragraph::new(hint_lines).block(hint_block), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text() {
        let text = "This is a test of the text wrapping functionality";
        let wrapped = wrap_text(text, 20);
        for line in &wrapped {
            assert!(line.len() <= 20);
        }
    }

    #[test]
    fn test_category_style() {
        let (color, _icon) = category_style("knowledge");
        assert_eq!(color, Color::Blue);

        let (color, _icon) = category_style("fact");
        assert_eq!(color, Color::Green);
    }

    #[test]
    fn test_importance_color() {
        assert_eq!(importance_color("critical"), Color::Red);
        assert_eq!(importance_color("high"), Color::LightRed);
        assert_eq!(importance_color("normal"), Color::White);
        assert_eq!(importance_color("low"), Color::DarkGray);
    }

    fn memory_info(id: i64, category: &str, stored_by: &str) -> crate::state::MemoryInfo {
        crate::state::MemoryInfo {
            id,
            content: format!("memory-{}", id),
            category: category.to_string(),
            summary: None,
            importance: "normal".to_string(),
            source: None,
            session_id: None,
            project_id: None,
            track_id: None,
            created_at: "2026-03-28T00:00:00Z".to_string(),
            expires_at: None,
            last_accessed: None,
            access_count: 0,
            accessed_by: Vec::new(),
            tags: Vec::new(),
            is_expanded: false,
            similarity_score: None,
            related_memory_ids: Vec::new(),
            nexus_runtime_state: None,
            nexus_scope: None,
            stored_by: stored_by.to_string(),
            access_history: Vec::new(),
        }
    }

    #[test]
    fn test_classify_relation_prefers_nexus_links() {
        let mut selected = memory_info(1, "context", "agent-a");
        selected.related_memory_ids = vec![2];
        let memory = memory_info(2, "fact", "agent-b");

        assert_eq!(
            classify_relation(&selected, &memory),
            MemoryRelationKind::NexusLink
        );
    }

    #[test]
    fn test_classify_relation_uses_session_before_category() {
        let mut selected = memory_info(1, "context", "agent-a");
        selected.session_id = Some("session-1".to_string());
        let mut memory = memory_info(2, "fact", "agent-b");
        memory.session_id = Some("session-1".to_string());

        assert_eq!(
            classify_relation(&selected, &memory),
            MemoryRelationKind::SessionLink
        );
    }
}
