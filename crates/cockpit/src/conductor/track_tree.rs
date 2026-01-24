use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use leindex_core::orchestrate::model::TrackStatus;
use super::model::SelectableItem;
use super::pane::ConductorPane;
use super::theme::ConductorTheme;
use super::theme::{STATUS_DONE, STATUS_PENDING, STATUS_ACTIVE};

pub fn render_track_tree(frame: &mut Frame, area: Rect, pane: &mut ConductorPane, theme: &crate::theme::Theme) {
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
            Line::from(Span::styled("  No tracks found.", Style::default().fg(theme.warning))),
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
    let items: Vec<ListItem> = selectable_items.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == pane.selected_index;
        
        match item {
            SelectableItem::Track { id, .. } => {
                let style = if is_selected {
                    Style::default().fg(conductor_theme.accent_primary).bold().bg(conductor_theme.bg_highlight)
                } else {
                    Style::default().fg(conductor_theme.fg_primary)
                };
                ListItem::new(Span::styled(format!(" {} {}", STATUS_ACTIVE, id), style))
            }
            SelectableItem::Task { title, depth, status, has_children, is_expanded, .. } => {
                let indent = "  ".repeat(*depth);
                let (status_symbol, status_color) = match status {
                    TrackStatus::Pending => (STATUS_PENDING, conductor_theme.task_pending),
                    TrackStatus::InProgress => (STATUS_ACTIVE, conductor_theme.task_active),
                    TrackStatus::Completed => (STATUS_DONE, conductor_theme.task_done),
                };
                
                let expand_symbol = if *has_children {
                    if *is_expanded { "[-] " } else { "[+] " }
                } else {
                    "    "
                };
                
                let style = if is_selected {
                    Style::default().fg(conductor_theme.accent_primary).bold().bg(conductor_theme.bg_highlight)
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
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(theme.highlight_bg).fg(theme.highlight_fg));

    frame.render_widget(list, inner_area);
}
