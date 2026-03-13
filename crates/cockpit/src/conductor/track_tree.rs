use super::model::{ConductorStatus, SelectableItem};
use super::pane::ConductorPane;
use super::theme::ConductorTheme;
use super::theme::{STATUS_ACTIVE, STATUS_DONE, STATUS_PENDING};
use super::{ConductorNode, ConductorNodeStatus, TreeNodeId};
use leindex_core::orchestrate::model::TrackStatus;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::sync::Arc;

/// Convert ConductorNodeStatus to display symbol and color
fn status_to_symbol_and_color(
    status: ConductorNodeStatus,
    theme: &ConductorTheme,
) -> (&'static str, Color) {
    match status {
        ConductorNodeStatus::Pending => (STATUS_PENDING, theme.task_pending),
        ConductorNodeStatus::InProgress => (STATUS_ACTIVE, theme.task_active),
        ConductorNodeStatus::Completed => (STATUS_DONE, theme.task_done),
        ConductorNodeStatus::Running => (STATUS_ACTIVE, theme.task_active),
        ConductorNodeStatus::Paused => ("[P]", Color::Yellow),
        ConductorNodeStatus::Failed => ("[F]", Color::Red),
        ConductorNodeStatus::Idle => ("[I]", Color::Gray),
        ConductorNodeStatus::Unknown => ("[?]", Color::Gray),
    }
}

/// Render a single node in the tree
fn render_node<'a>(
    node: &Arc<dyn ConductorNode>,
    node_id: &TreeNodeId,
    depth: usize,
    is_selected: bool,
    is_expanded: bool,
    theme: &crate::theme::Theme,
    conductor_theme: &ConductorTheme,
    pane: &ConductorPane,
) -> ListItem<'a> {
    let indent = "  ".repeat(depth);
    let (status_symbol, status_color) = status_to_symbol_and_color(node.status(), conductor_theme);

    // Determine expand/collapse symbol
    let has_children = node.is_expandable();
    let expand_symbol = if has_children {
        if is_expanded {
            "[-] "
        } else {
            "[+] "
        }
    } else {
        "    "
    };

    let style = if is_selected {
        Style::default()
            .fg(conductor_theme.accent_primary)
            .bold()
            .bg(conductor_theme.bg_highlight)
    } else {
        Style::default().fg(conductor_theme.fg_primary)
    };

    // Special handling for different node types
    let title = node.title();
    let mut spans = vec![
        Span::styled(indent.clone(), style),
        Span::styled(expand_symbol, style),
        Span::styled(
            format!(" {} ", status_symbol),
            Style::default().fg(status_color),
        ),
        Span::styled(title.to_string(), style),
    ];

    // Add special markers based on node type
    // Check if it's a track node by trying to downcast
    if let Some(_track) = (**node).as_any().downcast_ref::<super::ConductorTrackNode>() {
        // Add iteration count if available
        if let Some(track_id_str) = node_id.as_str().strip_prefix("track:") {
            if pane.state.current_track.as_ref() == Some(&track_id_str.to_string()) {
                spans.push(Span::styled(
                    format!(" ({})", pane.state.current_iteration),
                    Style::default().fg(theme.muted),
                ));
            }
        }
    }

    ListItem::new(Line::from(spans))
}

/// Render track tree using normalized model
pub fn render_track_tree_normalized(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let conductor_theme = ConductorTheme::default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Tracks & Tasks (Normalized) ")
        .border_style(if !pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if pane.normalized_tree.nodes.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Building tree...",
                Style::default().fg(theme.muted),
            )),
            Line::from(""),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Left)
            .style(Style::default().fg(theme.muted));
        frame.render_widget(paragraph, inner_area);
        return;
    }

    // Get visible nodes from normalized tree
    let visible_nodes = pane.get_visible_nodes();

    // Render each visible node
    let items: Vec<ListItem> = visible_nodes
        .iter()
        .enumerate()
        .map(|(_idx, node)| {
            let node_id = node.id();
            let is_selected = pane.normalized_tree.selected_node.as_ref() == Some(node_id);
            let is_expanded = pane.normalized_tree.is_expanded(node_id);
            let depth = node_id.as_str().matches(':').count(); // Simple depth estimation

            render_node(
                node,
                node_id,
                depth.saturating_sub(1), // Adjust for prefix
                is_selected,
                is_expanded,
                theme,
                &conductor_theme,
                pane,
            )
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg),
    );

    frame.render_widget(list, inner_area);
}

pub fn render_track_tree(
    frame: &mut Frame,
    area: Rect,
    pane: &mut ConductorPane,
    theme: &crate::theme::Theme,
) {
    let conductor_theme = ConductorTheme::default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Tracks & Tasks ")
        .border_style(if !pane.output_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
        });

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if pane.tracks.is_empty() {
        let mut text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No tracks found.",
                Style::default().fg(theme.warning),
            )),
            Line::from(""),
        ];

        // Show error message if available (helps debug path issues)
        if let Some(ref err) = pane.error_message {
            text.push(Line::from(Span::styled(
                format!("  {}", err),
                Style::default().fg(theme.muted).italic(),
            )));
            text.push(Line::from(""));
        }

        text.push(Line::from("  To create tracks:"));
        text.push(Line::from(Span::styled(
            "  1. Run: maestro newTrack",
            Style::default().fg(theme.fg),
        )));
        text.push(Line::from(Span::styled(
            format!("  2. Or create: {}/tracks.md", pane.tracks_dir.display()),
            Style::default().fg(theme.muted),
        )));

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Left)
            .style(Style::default().fg(theme.muted));
        frame.render_widget(paragraph, inner_area);
        return;
    }

    let selectable_items = pane.get_selectable_items();
    let items: Vec<ListItem> = selectable_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == pane.selected_index;

            match item {
                SelectableItem::Track {
                    id,
                    is_master,
                    is_external,
                    ..
                } => {
                    let runtime_status = pane.state.track_runtime_statuses.get(id);
                    let (status_symbol, status_color) = match runtime_status {
                        Some(ConductorStatus::Running) => {
                            (STATUS_ACTIVE, conductor_theme.task_active)
                        }
                        Some(ConductorStatus::Paused) => ("[P]", theme.warning),
                        Some(ConductorStatus::Failed) => ("[F]", theme.error),
                        Some(ConductorStatus::Completed) => {
                            (STATUS_DONE, conductor_theme.task_done)
                        }
                        _ => ("", conductor_theme.fg_primary),
                    };

                    // Get iteration count for this track if available
                    let iter_str =
                        if let Some(_track_state) = pane.state.track_runtime_statuses.get(id) {
                            // We need iteration count per track in state.
                            // For now, if it's the current track, use the global count.
                            if pane.state.current_track.as_ref() == Some(id) {
                                format!(" ({})", pane.state.current_iteration)
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        };

                    let style = if is_selected {
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .bold()
                            .bg(conductor_theme.bg_highlight)
                    } else if *is_master {
                        Style::default().fg(theme.warning).bold()
                    } else {
                        Style::default().fg(conductor_theme.fg_primary)
                    };

                    let mut spans = vec![Span::styled(
                        format!(" {} ", status_symbol),
                        Style::default().fg(status_color),
                    )];

                    if *is_master {
                        spans.push(Span::styled("👑 ", Style::default().fg(theme.warning)));
                    }

                    spans.push(Span::styled(id.clone(), style));
                    spans.push(Span::styled(iter_str, Style::default().fg(theme.muted)));

                    if *is_external {
                        spans.push(Span::styled(
                            " (ext)",
                            Style::default().fg(theme.muted).italic(),
                        ));
                    }

                    ListItem::new(Line::from(spans))
                }
                SelectableItem::Task {
                    title,
                    depth,
                    status,
                    has_children,
                    is_expanded,
                    ..
                } => {
                    let indent = "  ".repeat(*depth);
                    let (status_symbol, status_color) = match status {
                        TrackStatus::Pending => (STATUS_PENDING, conductor_theme.task_pending),
                        TrackStatus::InProgress => (STATUS_ACTIVE, conductor_theme.task_active),
                        TrackStatus::Completed => (STATUS_DONE, conductor_theme.task_done),
                    };

                    let expand_symbol = if *has_children {
                        if *is_expanded {
                            "[-] "
                        } else {
                            "[+] "
                        }
                    } else {
                        "    "
                    };

                    let style = if is_selected {
                        Style::default()
                            .fg(conductor_theme.accent_primary)
                            .bold()
                            .bg(conductor_theme.bg_highlight)
                    } else {
                        Style::default().fg(conductor_theme.fg_secondary)
                    };

                    let line = Line::from(vec![
                        Span::styled(indent, style),
                        Span::styled(expand_symbol, style),
                        Span::styled(status_symbol, Style::default().fg(status_color)),
                        Span::styled(format!(" {}", title), style),
                    ]);
                    ListItem::new(line)
                }
            }
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg),
    );

    frame.render_widget(list, inner_area);
}
