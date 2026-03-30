use super::model::{ConductorStatus, SelectableItem};
use super::normalized_model::ConductorNodeStatus;
use super::pane::ConductorPane;
use super::theme::ConductorTheme;
use super::theme::{STATUS_ACTIVE, STATUS_DONE, STATUS_PENDING};
use leindex_core::orchestrate::model::TrackStatus;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

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

/// Render a track tree using the normalized tree model
pub fn render_track_tree_normalized(
    frame: &mut Frame,
    area: Rect,
    pane: &ConductorPane,
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

    // Get the tree from pane state
    let tree = &pane.state.normalized_tree;

    if tree.root_ids.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No tracks found.",
                Style::default().fg(theme.warning),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  Create: {}/tracks.md", pane.tracks_dir.display()),
                Style::default().fg(theme.muted),
            )),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Left)
            .style(Style::default().fg(theme.muted));
        frame.render_widget(paragraph, inner_area);
        return;
    }

    // Render visible nodes from the normalized tree
    let visible_nodes = tree.visible_nodes();
    let items: Vec<ListItem> = visible_nodes
        .iter()
        .enumerate()
        .map(|(_idx, (node_id, node))| {
            let is_selected = tree.selected_node.as_ref() == Some(node_id);
            let is_expanded = tree.is_expanded(node_id);

            let (status_symbol, status_color) =
                status_to_symbol_and_color(node.status(), &conductor_theme);

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
                Style::default().fg(conductor_theme.fg_secondary)
            };

            ListItem::new(Line::from(vec![
                Span::styled(expand_symbol, style),
                Span::styled(status_symbol, Style::default().fg(status_color)),
                Span::styled(format!(" {}", node.title()), style),
            ]))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg),
    );

    frame.render_widget(list, inner_area);
}

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
