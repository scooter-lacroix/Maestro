//! Sessions tab rendering for Cockpit TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::state::SessionEntry;
use leindex_core::memory::LspStatus;

/// Get the tail of a session log file
pub fn session_log_tail(session_name: &str, lines: usize) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};

    // Sanitize session_name to prevent path traversal attacks
    let safe_session_name: String = session_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{}/.maestro/logs/{}.log", home, safe_session_name);

    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();

    // Avoid loading the entire file: read only the tail window.
    let window: u64 = 128 * 1024;
    let start = len.saturating_sub(window);
    let _ = file.seek(SeekFrom::Start(start)).ok()?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;

    let mut out: Vec<String> = Vec::new();
    for line in buf.lines().rev().take(lines) {
        out.push(line.to_string());
    }
    out.reverse();
    Some(out.join("\n"))
}

pub fn render_sessions(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if !app.preview_focused {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .title(" 📁 Sessions & Groups ")
        .title_style(if !app.preview_focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.preview_focused {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .title(format!(
            " 🖥️ Preview {} ",
            if app.preview_focused { "[FOCUSED]" } else { "" }
        ))
        .title_style(if app.preview_focused {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if app.session_entries.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No sessions or groups found."),
            Line::from(""),
            Line::from("  Create a new session with 'n' or select a project."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, area);
    } else {
        let mut items = Vec::new();

        // Pre-collect LSP indicators for all sessions (single borrow, then release)
        // Clone the cache first to avoid borrow checker issues
        let lsp_cache = app.lsp_status_cache.clone();
        let lsp_indicators_map = {
            let mut map = std::collections::HashMap::new();
            for (session_id, lsps) in &lsp_cache {
                if !lsps.is_empty() {
                    let indicators: Vec<Span> = lsps
                        .iter()
                        .map(|(lsp_name, status)| {
                            let (icon, color) = match status {
                                LspStatus::Running => (" ● ", Color::Green),
                                LspStatus::Starting => (" ◐ ", Color::Yellow),
                                LspStatus::Error => (" x ", Color::Red),
                                LspStatus::Stopped => (" ○ ", Color::Gray),
                            };
                            let short_name = if lsp_name.contains("rust") {
                                "R"
                            } else if lsp_name.contains("ruff") || lsp_name.contains("python") {
                                "P"
                            } else if lsp_name.contains("typescript") || lsp_name.contains("ts") {
                                "T"
                            } else {
                                "?"
                            };
                            Span::styled(
                                format!("{}{}", short_name, icon),
                                Style::default().fg(color),
                            )
                        })
                        .collect();
                    map.insert(session_id.clone(), indicators);
                }
            }
            map
        };
        // `app` borrow is now released

        for (i, entry) in app.session_entries.iter().enumerate() {
            match entry {
                SessionEntry::Group(g) => {
                    let icon = if g.is_expanded { "▼ " } else { "▶ " };
                    items.push(ListItem::new(vec![Line::from(vec![
                        Span::styled(format!("  {} ", icon), Style::default().fg(Color::Yellow)),
                        Span::styled(&g.name, Style::default().bold().fg(Color::White)),
                        Span::styled(
                            if let Some(cat) = &g.category {
                                format!(" [{}]", cat)
                            } else {
                                "".to_string()
                            },
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::styled(
                            format!(" ({})", g.path),
                            Style::default().fg(Color::DarkGray).italic(),
                        ),
                    ])]));
                }
                SessionEntry::Session(s) => {
                    let is_running =
                        s.status == leindex_core::memory::models::SessionStatus::Running;
                    let is_terminated =
                        s.status == leindex_core::memory::models::SessionStatus::Terminated;
                    let is_waiting =
                        s.status == leindex_core::memory::models::SessionStatus::Waiting;

                    let (status_icon, status_color) = if is_running {
                        (" * ", Color::Green)
                    } else if is_terminated {
                        (" x ", Color::Red)
                    } else if is_waiting {
                        (" ◒ ", Color::Yellow)
                    } else {
                        (" o ", Color::Gray)
                    };

                    let title_style = if is_running {
                        Style::default().fg(Color::Cyan)
                    } else if is_terminated {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    // Determine if this is the last item in a group (for L-line)
                    let mut branch = " ├─";
                    let is_last_in_group = if let Some(next) = app.session_entries.get(i + 1) {
                        matches!(next, SessionEntry::Group(_))
                    } else {
                        // End of list is also end of group
                        true
                    };

                    if is_last_in_group {
                        branch = " └─";
                    }

                    let mut line_spans = vec![
                        Span::styled(
                            format!("  {}", branch),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::styled(&s.title, title_style),
                    ];

                    if s.status == leindex_core::memory::models::SessionStatus::Terminated {
                        line_spans.push(Span::styled(
                            " [KILLED]",
                            Style::default().fg(Color::Red).bold(),
                        ));
                    }

                    line_spans.push(Span::styled(
                        format!(" [{}]", s.tool.as_deref().unwrap_or("?")),
                        Style::default().fg(Color::DarkGray),
                    ));

                    // Add LSP indicators if any exist (use pre-collected map)
                    if let Some(indicators) = lsp_indicators_map.get(&s.session_id) {
                        if !indicators.is_empty() {
                            line_spans.push(Span::raw(" "));
                            for indicator in indicators {
                                line_spans.push(indicator.clone());
                            }
                        }
                    }

                    items.push(ListItem::new(vec![Line::from(line_spans)]));
                }
            }
        }

        let list = List::new(items).block(list_block).highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .fg(Color::White)
                .bold(),
        );
        frame.render_stateful_widget(list, chunks[0], &mut app.session_state);

        // Render Preview
        let mut preview_lines = Vec::new();

        if let Some(i) = app.session_state.selected() {
            if let Some(SessionEntry::Session(s)) = app.session_entries.get(i) {
                // Header (Replicating Go TUI)
                let status_icon = match s.status {
                    leindex_core::memory::models::SessionStatus::Running => "●",
                    leindex_core::memory::models::SessionStatus::Waiting => "◐",
                    _ => "○",
                };
                let status_color = match s.status {
                    leindex_core::memory::models::SessionStatus::Running => Color::Green,
                    leindex_core::memory::models::SessionStatus::Waiting => Color::Yellow,
                    leindex_core::memory::models::SessionStatus::Terminated => Color::Red,
                    _ => Color::DarkGray,
                };

                // Row 1: Icon Title (ID)
                preview_lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", status_icon),
                        Style::default().fg(status_color).bold(),
                    ),
                    Span::styled(&s.title, Style::default().fg(Color::Cyan).bold()),
                    Span::styled(
                        format!(" ({})", s.session_id),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                // Row 2: Tool, Group, Activity
                let activity_str = "active now"; // Placeholder, replace with actual activity logic if available
                preview_lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", s.tool.as_deref().unwrap_or("shell")),
                        Style::default().bg(Color::Magenta).fg(Color::Black),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!(" {} ", s.group_path.as_deref().unwrap_or("Uncategorized")),
                        Style::default().bg(Color::Cyan).fg(Color::Black),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!(" ⏱ {}", activity_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                // Row 3: Path
                preview_lines.push(Line::from(vec![
                    Span::styled(" 📁 ", Style::default()),
                    Span::styled(&s.project_path, Style::default().fg(Color::DarkGray)),
                ]));

                // Row 4: Tool session IDs (best-effort capture)
                if let Some(ref metadata) = s.metadata {
                    if s.tool.as_deref() == Some("claude") {
                        if let Some(cid) =
                            metadata.get("claude_session_id").and_then(|v| v.as_str())
                        {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Claude: ", Style::default().fg(Color::DarkGray)),
                                Span::styled("● Connected", Style::default().fg(Color::Green)),
                            ]));
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(cid, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    if s.tool.as_deref() == Some("gemini") {
                        if let Some(gid) =
                            metadata.get("gemini_session_id").and_then(|v| v.as_str())
                        {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Gemini: ", Style::default().fg(Color::DarkGray)),
                                Span::styled("● Connected", Style::default().fg(Color::Green)),
                            ]));
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(gid, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    if s.tool.as_deref() == Some("codex") {
                        if let Some(cid) = metadata.get("codex_session_id").and_then(|v| v.as_str())
                        {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Codex: ", Style::default().fg(Color::DarkGray)),
                                Span::styled("● Captured", Style::default().fg(Color::Green)),
                            ]));
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(cid, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    if s.tool.as_deref() == Some("opencode") {
                        if let Some(oid) =
                            metadata.get("opencode_session_id").and_then(|v| v.as_str())
                        {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" OpenCode: ", Style::default().fg(Color::DarkGray)),
                                Span::styled("● Captured", Style::default().fg(Color::Green)),
                            ]));
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Session ID: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(oid, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    if s.tool.as_deref() == Some("amp") {
                        if let Some(tid) = metadata.get("amp_thread_id").and_then(|v| v.as_str()) {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Amp: ", Style::default().fg(Color::DarkGray)),
                                Span::styled("● Captured", Style::default().fg(Color::Green)),
                            ]));
                            preview_lines.push(Line::from(vec![
                                Span::styled(" Thread ID: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(tid, Style::default().fg(Color::White)),
                            ]));
                        }
                    }

                    if let Some(mcps) = metadata.get("loaded_mcp_names").and_then(|v| v.as_array())
                    {
                        let mcp_names: Vec<String> = mcps
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        if !mcp_names.is_empty() {
                            preview_lines.push(Line::from(vec![
                                Span::styled(" 🔌 MCPs: ", Style::default().fg(Color::Cyan)),
                                Span::styled(
                                    mcp_names.join(", "),
                                    Style::default().fg(Color::White),
                                ),
                            ]));
                        }
                    }
                }

                if s.tool.as_deref() == Some("claude") {
                    preview_lines.push(Line::from(vec![
                        Span::styled(" Fork: ", Style::default().fg(Color::DarkGray).italic()),
                        Span::styled("f ", Style::default().fg(Color::Cyan).bold()),
                        Span::raw("(quick), "),
                        Span::styled("F ", Style::default().fg(Color::Cyan).bold()),
                        Span::raw("(options)"),
                    ]));
                }

                // Divider
                preview_lines.push(Line::from(""));
                let divider_width = (chunks[1].width as usize).saturating_sub(6);
                let divider = "─".repeat(divider_width / 2 - 4);
                preview_lines.push(Line::from(vec![Span::styled(
                    format!(" {} Output {} ", divider, divider),
                    Style::default().fg(Color::DarkGray),
                )]));
                preview_lines.push(Line::from(""));
            }
        }

        if app.session_preview_content.is_empty() {
            preview_lines.push(Line::from("  (No preview available)"));
        } else {
            for line in app.session_preview_content.lines() {
                preview_lines.push(Line::from(format!("  {}", line)));
            }
        }

        let preview = Paragraph::new(preview_lines)
            .block(preview_block)
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));
        frame.render_widget(preview, chunks[1]);
    }
}
