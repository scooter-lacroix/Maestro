//! Dashboard tab rendering for Cockpit TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, List, ListItem, Paragraph, Wrap},
    Frame,
    prelude::*,
};

use crate::app::App;
use crate::state::{DashFocus, DashSessionEntry};
use leindex_analyzers::memory::LspStatus;

pub fn render_dashboard(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(chunks[0]);

    // Stats cards
    let stats_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" ⚡ Quick Stats ")
        .title_style(Style::default().fg(Color::Yellow));

    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  📁 PROJECTS:   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}", app.stats.project_count),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                "  [Active System Roots]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🎯 TRACKS:     ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}", app.stats.track_count),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                "  [Active Workstreams]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  🧠 MEMORIES:   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}", app.stats.memory_count),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                "  [Context Vectors]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ⚡ LEINDEX:    ", Style::default().fg(Color::Cyan)),
            Span::styled("HD", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                "  [Multi-Layer structural cache]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
    ];
    let stats = Paragraph::new(stats_text).block(stats_block);
    frame.render_widget(stats, left_chunks[0]);

    // Welcome message
    let welcome_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Welcome ")
        .title_style(Style::default().fg(Color::Magenta));

    // Updated welcome section with multi-layer architecture diagram & ANIMATION
    let anim_char = match (app.frame_count / 10) % 4 {
        0 => "⠋",
        1 => "⠙",
        2 => "⠹",
        _ => "⠸",
    };
    let welcome_color = if (app.frame_count / 20) % 2 == 0 {
        Color::Magenta
    } else {
        Color::LightMagenta
    };

    let welcome_text = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} MAESTRO SYSTEM OVERVIEW ", anim_char),
                Style::default().fg(welcome_color).bold(),
            ),
            Span::styled(
                " [v2.0-beta-5]",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(""),
        Line::from("  [WORKSPACE] ─────▶ [SCANNER] ─────▶ [LEINDEXER]"),
        Line::from("       │                │                │"),
        Line::from("       ▼                ▼                ▼"),
        Line::from("  [CONFIGS]        [TRACKS]         [MEMORY DB]"),
        Line::from("       │                │                │"),
        Line::from("       └──────┬─────────┴────────────────┘"),
        Line::from("              ▼"),
        Line::from(vec![Span::styled(
            "      [ AI AGENT LAYER ]",
            Style::default()
                .fg(Color::LightMagenta)
                .bold()
                .add_modifier(Modifier::DIM),
        )]),
        Line::from("      (Claude / Gemini / Codex / AMP)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  🚀 CAPABILITIES & FEATURES:",
            Style::default().bold().fg(Color::Yellow),
        )]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Green)),
            Span::styled("Indexing: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("N-Layer vector search via LEANN."),
            Span::styled(
                " (Example: 'scan /path/to/repo')",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Cyan)),
            Span::styled("Sessions: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Persistent tmux environments."),
            Span::styled(
                " (Example: 'n' to spawn)",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Magenta)),
            Span::styled("Analysis: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Structural code intelligence."),
            Span::styled(
                " (Example: 'analyze src/')",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(Color::Blue)),
            Span::styled("Memory:   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Global cross-project knowledge."),
            Span::styled(
                " (Example: Tab 5)",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Maestro is your autonomous coding cockpit. ",
                Style::default().fg(Color::LightBlue).italic(),
            ),
            Span::styled(
                "Stay playful, build fast!",
                Style::default().fg(Color::Yellow).bold(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("'/'", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                " for the Ultimate Command Guide",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];
    let welcome = Paragraph::new(welcome_text).block(welcome_block);
    frame.render_widget(welcome, left_chunks[1]);

    // Right side - System Status & MCP Pool
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(chunks[1]);

    // Top Right - Recent Sessions
    let session_block = Block::default()
        .borders(Borders::ALL)
        .border_type(
            if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions {
                BorderType::Double
            } else {
                BorderType::Rounded
            },
        )
        .title(" 🕒 Recent Sessions ")
        .title_style(
            if app.tab_index == 0 && app.dash_focus == DashFocus::Sessions {
                Style::default().fg(Color::Blue).bold()
            } else {
                Style::default().fg(Color::Blue)
            },
        );

    let mut session_items = Vec::new();
    if app.dash_session_entries.is_empty() {
        session_items.push(ListItem::new("  No active sessions"));
    } else {
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

        for entry in &app.dash_session_entries {
            match entry {
                DashSessionEntry::GroupHeader { group_path } => {
                    let group_name = if group_path == "uncategorized" {
                        "[Uncategorized]".to_string()
                    } else {
                        app.groups
                            .iter()
                            .find(|g| g.path == *group_path)
                            .map(|g| g.name.clone())
                            .unwrap_or_else(|| group_path.to_string())
                    };
                    session_items.push(ListItem::new(Line::from(vec![Span::styled(
                        format!(" 📁 {} ", group_name),
                        Style::default().fg(Color::Yellow).bold(),
                    )])));
                }
                DashSessionEntry::Session(sess) => {
                    let status_icon = match sess.status {
                        leindex_analyzers::memory::models::SessionStatus::Running => {
                            Span::styled(" ● ", Style::default().fg(Color::Green))
                        }
                        leindex_analyzers::memory::models::SessionStatus::Terminated => {
                            Span::styled(" x ", Style::default().fg(Color::Red))
                        }
                        leindex_analyzers::memory::models::SessionStatus::Waiting => {
                            Span::styled(" ◒ ", Style::default().fg(Color::Yellow))
                        }
                        _ => Span::styled(" o ", Style::default().fg(Color::Gray)),
                    };

                    // Build line with session status, title, and LSP indicators
                    let mut line_spans = vec![
                        Span::raw("   "),
                        status_icon,
                        Span::styled(sess.title.clone(), Style::default().bold()),
                    ];

                    // Add LSP indicators if any exist (use pre-collected map)
                    if let Some(indicators) = lsp_indicators_map.get(&sess.session_id) {
                        if !indicators.is_empty() {
                            line_spans.push(Span::raw(" "));
                            line_spans
                                .push(Span::styled("LSP:", Style::default().fg(Color::DarkGray)));
                            for indicator in indicators {
                                line_spans.push(indicator.clone());
                            }
                        }
                    }

                    session_items.push(ListItem::new(Line::from(line_spans)));
                }
            }
        }
    }
    let sessions = List::new(session_items)
        .block(session_block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(sessions, right_chunks[0], &mut app.dash_session_state);

    // Bottom Right - MCP Pool
    let mcp_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.tab_index == 0 && app.dash_focus == DashFocus::Mcp {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .title(" 🕹️ Interactive MCP Pool ")
        .title_style(if app.tab_index == 0 && app.dash_focus == DashFocus::Mcp {
            Style::default().fg(theme.accent).bold()
        } else {
            Style::default().fg(theme.accent)
        });

    let mcp_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(right_chunks[1]);

    let mcp_info = Paragraph::new(vec![Line::from(vec![
        Span::styled("Tip: ", Style::default().fg(theme.muted)),
        Span::styled("Tool Search", Style::default().fg(theme.warning).bold()),
        Span::styled(
            " is dynamic via `maestro mcp tool-search` (no full tool listing).",
            Style::default().fg(theme.muted),
        ),
    ])])
    .block(Block::default())
    .wrap(Wrap { trim: true });
    frame.render_widget(mcp_info, mcp_chunks[0]);

    let mcp_items: Vec<ListItem> = app
        .mcp_servers
        .iter()
        .map(|s| {
            let status_color = if s.status == leindex_analyzers::memory::models::McpStatus::Running
            {
                Color::Green
            } else {
                Color::Red
            };
            ListItem::new(vec![Line::from(vec![
                Span::styled(format!("  {} ", s.name), Style::default().bold()),
                Span::styled(
                    format!(" [{}] ", s.status.to_string()),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!(" {} active", s.client_count),
                    Style::default().fg(Color::Gray),
                ),
            ])])
        })
        .collect();

    let mcp_list = List::new(mcp_items)
        .block(mcp_block)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .bold(),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(mcp_list, mcp_chunks[1], &mut app.mcp_state);
}
