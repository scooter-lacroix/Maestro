//! LSPs tab rendering for Cockpit TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, List, ListItem, Paragraph, Wrap},
    Frame,
    prelude::*,
};

use crate::app::App;
use leindex_core::memory::LspStatus;

/// Get installation commands for a given LSP name
pub fn get_lsp_install_command(lsp_name: &str) -> Vec<&'static str> {
    match lsp_name {
        "rust-analyzer" => vec![
            "# Via rustup (recommended):",
            "rustup component add rust-analyzer",
            "",
            "# Or pre-built binary:",
            "curl -L https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu -o ~/.local/bin/rust-analyzer",
            "chmod +x ~/.local/bin/rust-analyzer",
        ],
        "ruff" => vec![
            "# Via cargo (Rust binary - recommended):",
            "cargo install ruff",
            "",
            "# Or via pip (also installs Rust binary):",
            "pip install ruff",
            "",
            "# Then run: ruff server",
        ],
        "typescript-language-server" => vec![
            "# Via cargo (Rust binary from crates.io):",
            "cargo install typescript-language-server",
        ],
        _ => vec![
            "# See LSP documentation for installation instructions",
        ],
    }
}

pub fn render_lsps(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    // Collect missing LSPs for installation guidance
    let missing_lsps: Vec<&str> = app
        .lsp_availability
        .iter()
        .filter(|(_, &available)| !available)
        .map(|(name, _)| name.as_str())
        .collect();

    // Determine if we need to show the missing LSPs section
    let has_missing_lsps = !missing_lsps.is_empty();

    // Calculate constraints: header + LSP list + (optional) missing LSPs section
    let list_min = if has_missing_lsps {
        // Reserve space for missing LSPs section (approximately 15 lines)
        Constraint::Min(0)
    } else {
        Constraint::Min(0)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                                     // Header
            list_min,                                                  // LSP list
            Constraint::Length(if has_missing_lsps { 15 } else { 0 }), // Missing LSPs section
        ])
        .split(area);

    // Header block with control hints
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 🔌 Language Server Protocol (LSP) Status ")
        .title_style(Style::default().fg(theme.accent));

    let header_text = vec![Line::from(vec![
        Span::styled("Controls: ", Style::default().fg(theme.muted)),
        Span::styled("[s] Toggle ", Style::default().fg(theme.warning).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[R] Restart ", Style::default().fg(theme.warning).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[r] Refresh ", Style::default().fg(theme.warning).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[l] Logs", Style::default().fg(theme.warning).bold()),
    ])];
    frame.render_widget(Paragraph::new(header_text).block(header_block), chunks[0]);

    // Collect all LSPs across all sessions into a flat list
    let mut lsp_entries: Vec<(String, String, LspStatus, Option<String>)> = Vec::new();
    // (session_id, lsp_name, status, session_title)

    for session in &app.sessions {
        let session_title = session.title.clone();
        if let Some(lsp_states) = app.lsp_status_cache.get(&session.session_id) {
            for (lsp_name, status) in lsp_states {
                lsp_entries.push((
                    session.session_id.clone(),
                    lsp_name.clone(),
                    *status,
                    Some(session_title.clone()),
                ));
            }
        }
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 📋 LSP Servers (by session) ")
        .title_style(Style::default().fg(theme.accent_alt));

    if lsp_entries.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No LSP servers found."),
            Line::from(""),
            Line::from("  Tip: LSPs are auto-detected from tmux sessions."),
            Line::from("  Press 'r' to refresh status."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, chunks[1]);
    } else {
        // Create list items with color-coded status
        let lsp_items: Vec<ListItem> = lsp_entries
            .iter()
            .map(|(_session_id, lsp_name, status, session_title)| {
                let (status_text, status_color, icon) = match status {
                    LspStatus::Running => ("Running", Color::Green, "●"),
                    LspStatus::Stopped => ("Stopped", Color::Red, "■"),
                    LspStatus::Error => ("Error", Color::Red, "⚠"),
                    LspStatus::Starting => ("Starting", Color::Yellow, "○"),
                };

                // Get short session title (truncate if too long)
                // Use character-based slicing to avoid UTF-8 truncation panic
                let short_title = session_title
                    .as_ref()
                    .map(|t| {
                        if t.chars().count() > 20 {
                            let truncated: String = t.chars().take(17).collect();
                            format!("{}...", truncated)
                        } else {
                            t.clone()
                        }
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(format!("{} ", lsp_name), Style::default().bold()),
                    Span::styled(
                        format!("[{}] ", status_text),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(
                        format!("({})", short_title),
                        Style::default().fg(Color::DarkGray).italic(),
                    ),
                ]))
            })
            .collect();

        let lsp_list = List::new(lsp_items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .bold(),
            )
            .highlight_symbol(">> ");
        frame.render_stateful_widget(lsp_list, chunks[1], &mut app.lsp_state);
    }

    // Missing LSPs section with installation commands
    if has_missing_lsps {
        let missing_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" ⚠ Missing LSPs - Installation Required ")
            .title_style(Style::default().fg(Color::Yellow).bold());

        let mut missing_lines = vec![
            Line::from(vec![Span::styled(
                "The following LSPs are not available on your system:",
                Style::default().fg(Color::Yellow).bold(),
            )]),
            Line::from(""),
        ];

        for lsp_name in &missing_lsps {
            let install_commands = get_lsp_install_command(lsp_name);
            missing_lines.push(Line::from(vec![
                Span::styled(
                    format!("▸ {} ", lsp_name),
                    Style::default().fg(Color::Red).bold(),
                ),
                Span::styled("NOT FOUND", Style::default().fg(Color::Red).bold()),
            ]));

            for cmd in &install_commands {
                if cmd.starts_with("#") {
                    missing_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(*cmd, Style::default().fg(Color::DarkGray).italic()),
                    ]));
                } else if !cmd.is_empty() {
                    missing_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("$ {}", cmd), Style::default().fg(Color::Cyan)),
                    ]));
                } else {
                    missing_lines.push(Line::from(""));
                }
            }
            missing_lines.push(Line::from(""));
        }

        let missing_para = Paragraph::new(missing_lines)
            .block(missing_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(missing_para, chunks[2]);
    }
}
