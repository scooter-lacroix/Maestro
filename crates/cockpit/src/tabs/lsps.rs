//! LSPs tab rendering for Cockpit TUI
//!
//! Features:
//! - LSP status summary with running/stopped/error counts
//! - Detected LSPs for each session (even when not running)
//! - Missing LSPs with installation guidance
//! - LSP installer dropdown for Rust-based LSPs
//! - Diagnostic detail view with expandable tree
//! - "Send to Agent" functionality for debugging

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::tabs::lsp_registry::{check_lsp_installed, get_available_lsps, get_install_command};
use leindex_core::memory::LspStatus;

pub fn render_lsps(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let has_missing_lsps = app.lsp_availability.values().any(|&available| !available);

    let summary_height = 7;
    let missing_height = if has_missing_lsps { 10 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(summary_height),
            Constraint::Min(5),
            Constraint::Length(missing_height),
        ])
        .split(area);

    render_header(frame, chunks[0], app, &theme);
    render_status_summary(frame, chunks[1], app, &theme);
    render_lsp_list(frame, chunks[2], app, &theme);

    if has_missing_lsps {
        render_missing_lsps(frame, chunks[3], app, &theme);
    }

    if app.lsp_installer.is_open {
        render_lsp_installer_modal(frame, area, app, &theme);
    }

    if app.diagnostic_view.is_open {
        render_diagnostic_detail_modal(frame, area, app, &theme);
    }
}

fn render_header(frame: &mut Frame, area: Rect, _app: &App, theme: &crate::theme::Theme) {
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
        Span::styled("[i] Install LSP ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[d] Diagnostics ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("[l] Logs", Style::default().fg(theme.warning).bold()),
    ])];
    frame.render_widget(Paragraph::new(header_text).block(header_block), area);
}

fn render_status_summary(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    let summary = &app.lsp_status_summary;

    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 📊 LSP Summary ")
        .title_style(Style::default().fg(theme.accent_alt));

    let running_color = if summary.running > 0 {
        Color::Green
    } else {
        Color::DarkGray
    };
    let stopped_color = if summary.stopped > 0 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let error_color = if summary.errors > 0 {
        Color::Red
    } else {
        Color::DarkGray
    };

    let status_line = Line::from(vec![
        Span::styled("Running: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} ", summary.running),
            Style::default().fg(running_color).bold(),
        ),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("Stopped: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} ", summary.stopped),
            Style::default().fg(stopped_color).bold(),
        ),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("Errors: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} ", summary.errors),
            Style::default().fg(error_color).bold(),
        ),
        Span::styled("| ", Style::default().fg(theme.muted)),
        Span::styled("Total: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{}", summary.total_lsps),
            Style::default().fg(theme.fg).bold(),
        ),
    ]);

    let issues_color = if summary.total_errors > 0 {
        Color::Red
    } else if summary.total_warnings > 0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let issues_line = Line::from(vec![
        Span::styled("Issues: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!(
                "{} errors, {} warnings",
                summary.total_errors, summary.total_warnings
            ),
            Style::default().fg(issues_color),
        ),
        Span::styled("  [d] View Details", Style::default().fg(Color::Cyan)),
    ]);

    let available_lsps: Vec<&str> = app
        .lsp_availability
        .iter()
        .filter(|(_, &available)| available)
        .map(|(name, _)| name.as_str())
        .collect();

    let available_line = Line::from(vec![
        Span::styled("Installed: ", Style::default().fg(theme.muted)),
        Span::styled(
            if available_lsps.is_empty() {
                "None".to_string()
            } else {
                available_lsps.join(", ")
            },
            Style::default().fg(if available_lsps.is_empty() {
                Color::Red
            } else {
                Color::Green
            }),
        ),
    ]);

    let sessions_with_detected = app
        .lsp_detected_cache
        .iter()
        .filter(|(_, lsps)| !lsps.is_empty())
        .count();

    let detected_line = Line::from(vec![
        Span::styled(
            "Sessions with detected LSPs: ",
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{}", sessions_with_detected),
            Style::default().fg(theme.fg),
        ),
        Span::styled("  [i] Install More LSPs", Style::default().fg(Color::Cyan)),
    ]);

    let text = vec![
        Line::from(""),
        status_line,
        issues_line,
        available_line,
        detected_line,
        Line::from(""),
    ];

    frame.render_widget(Paragraph::new(text).block(summary_block), area);
}

fn render_lsp_list(frame: &mut Frame, area: Rect, app: &mut App, theme: &crate::theme::Theme) {
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 📋 LSP Servers ")
        .title_style(Style::default().fg(theme.accent_alt));

    let mut lsp_entries: Vec<LspEntry> = Vec::new();

    for session in &app.sessions {
        let session_title = session.title.clone();
        let running_lsps = app.lsp_status_cache.get(&session.session_id);
        let detected = app.lsp_detected_cache.get(&session.session_id);

        if let Some(lsp_states) = running_lsps {
            for (lsp_name, status) in lsp_states {
                lsp_entries.push(LspEntry {
                    session_id: session.session_id.clone(),
                    lsp_name: lsp_name.clone(),
                    status: *status,
                    session_title: session_title.clone(),
                    is_running: true,
                });
            }
        }

        if let Some(detected_lsps) = detected {
            let running_names: Vec<&String> = running_lsps
                .map(|v| v.iter().map(|(n, _)| n).collect())
                .unwrap_or_default();

            for lsp_name in detected_lsps {
                if !running_names.contains(&lsp_name) {
                    let available = app.lsp_availability.get(lsp_name).copied().unwrap_or(false);
                    lsp_entries.push(LspEntry {
                        session_id: session.session_id.clone(),
                        lsp_name: lsp_name.clone(),
                        status: if available {
                            LspStatus::Stopped
                        } else {
                            LspStatus::Error
                        },
                        session_title: session_title.clone(),
                        is_running: false,
                    });
                }
            }
        }
    }

    if lsp_entries.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from("  No LSP servers detected."),
            Line::from(""),
            Line::from("  Sessions with code files will show detected LSPs here."),
            Line::from("  Press 'r' to refresh after creating sessions."),
            Line::from("  Press 'i' to open the LSP installer."),
        ];
        let para = Paragraph::new(text).block(list_block);
        frame.render_widget(para, area);
    } else {
        lsp_entries.sort_by(|a, b| match (a.is_running, b.is_running) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => match a.session_title.cmp(&b.session_title) {
                std::cmp::Ordering::Equal => a.lsp_name.cmp(&b.lsp_name),
                other => other,
            },
        });

        let lsp_items: Vec<ListItem> = lsp_entries
            .iter()
            .map(|entry| {
                let (status_text, status_color, icon) = match entry.status {
                    LspStatus::Running => ("Running", Color::Green, "●"),
                    LspStatus::Stopped => {
                        if entry.is_running {
                            ("Stopped", Color::Red, "■")
                        } else {
                            ("Detected", Color::Yellow, "○")
                        }
                    }
                    LspStatus::Error => ("Missing", Color::Red, "⚠"),
                    LspStatus::Starting => ("Starting", Color::Yellow, "○"),
                };

                let short_title = if entry.session_title.chars().count() > 18 {
                    let truncated: String = entry.session_title.chars().take(15).collect();
                    format!("{}...", truncated)
                } else {
                    entry.session_title.clone()
                };

                let lsp_short = match entry.lsp_name.as_str() {
                    "rust-analyzer" => "rust-analyzer",
                    "ruff" => "ruff",
                    "typescript-language-server" => "ts-lsp",
                    other => other,
                };

                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(format!("{:<18} ", lsp_short), Style::default().bold()),
                    Span::styled(
                        format!("[{:<8}] ", status_text),
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
        frame.render_stateful_widget(lsp_list, area, &mut app.lsp_state);
    }
}

fn render_missing_lsps(frame: &mut Frame, area: Rect, app: &App, _theme: &crate::theme::Theme) {
    let missing_lsps: Vec<&str> = app
        .lsp_availability
        .iter()
        .filter(|(_, &available)| !available)
        .map(|(name, _)| name.as_str())
        .collect();

    if missing_lsps.is_empty() {
        return;
    }

    let missing_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" ⚠ Missing LSPs - Press [i] to Install ")
        .title_style(Style::default().fg(Color::Yellow).bold());

    let mut missing_lines = vec![Line::from(vec![Span::styled(
        format!(
            "Missing: {}  (Press 'i' for installer)",
            missing_lsps.join(", ")
        ),
        Style::default().fg(Color::Yellow),
    )])];

    for lsp_name in &missing_lsps {
        missing_lines.push(Line::from(vec![
            Span::styled(
                format!("  ▸ {} ", lsp_name),
                Style::default().fg(Color::Red),
            ),
            Span::styled("NOT INSTALLED", Style::default().fg(Color::Red).bold()),
        ]));
    }

    let missing_para = Paragraph::new(missing_lines)
        .block(missing_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(missing_para, area);
}

fn render_lsp_installer_modal(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    theme: &crate::theme::Theme,
) {
    let modal_area = centered_rect(70, 75, area);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 📦 Install Rust-Based Language Servers ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .style(Style::default().bg(Color::Reset));

    frame.render_widget(modal_block.clone(), modal_area);

    let inner_area = modal_block.inner(modal_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let help_text = Paragraph::new(Line::from(vec![
        Span::styled("↑/↓ Navigate  ", Style::default().fg(theme.muted)),
        Span::styled("[Enter] Install  ", Style::default().fg(Color::Cyan)),
        Span::styled("[Esc] Close", Style::default().fg(theme.muted)),
    ]));
    frame.render_widget(help_text, chunks[0]);

    let available_lsps = get_available_lsps();
    let items: Vec<ListItem> = available_lsps
        .iter()
        .enumerate()
        .map(|(i, lsp)| {
            let is_installed = check_lsp_installed(lsp);
            let is_selected = i == app.lsp_installer.selected_index;

            let status_icon = if is_installed { "✓" } else { "○" };
            let status_color = if is_installed {
                Color::Green
            } else {
                Color::Yellow
            };

            let style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .bold()
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(format!("{:<22} ", lsp.display_name), style),
                Span::styled(
                    format!("[{:<12}] ", lsp.language),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    if is_installed {
                        "INSTALLED"
                    } else {
                        "Available"
                    },
                    Style::default().fg(status_color),
                ),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.lsp_installer.selected_index));

    let lsp_list = List::new(items).highlight_symbol(">> ");
    frame.render_stateful_widget(lsp_list, chunks[1], &mut list_state);

    if let Some(selected_lsp) = available_lsps.get(app.lsp_installer.selected_index) {
        let install_cmd = get_install_command(selected_lsp);
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Install: ", Style::default().fg(theme.muted)),
            Span::styled(&install_cmd, Style::default().fg(Color::Cyan)),
        ]));
        frame.render_widget(footer, chunks[2]);
    }
}

fn render_diagnostic_detail_modal(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    theme: &crate::theme::Theme,
) {
    let modal_area = centered_rect(80, 80, area);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" 🔍 LSP Diagnostics Detail ")
        .title_style(Style::default().fg(Color::Yellow).bold())
        .style(Style::default().bg(Color::Reset));

    frame.render_widget(modal_block.clone(), modal_area);

    let inner_area = modal_block.inner(modal_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(4),
        ])
        .split(inner_area);

    let summary = &app.lsp_status_summary;
    let help_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(
                "{} Errors, {} Warnings  ",
                summary.total_errors, summary.total_warnings
            ),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("↑/↓ Navigate  ", Style::default().fg(theme.muted)),
        Span::styled("[Enter] Expand  ", Style::default().fg(Color::Cyan)),
        Span::styled("[Esc] Close", Style::default().fg(theme.muted)),
    ]));
    frame.render_widget(help_text, chunks[0]);

    if app.lsp_diagnostics_cache.is_empty() {
        let empty_text = vec![
            Line::from(""),
            Line::from("  No diagnostics available."),
            Line::from(""),
            Line::from("  Run an LSP on a project to capture diagnostics."),
            Line::from("  Press 'r' to refresh after LSP starts."),
        ];
        frame.render_widget(Paragraph::new(empty_text), chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .lsp_diagnostics_cache
            .iter()
            .enumerate()
            .map(|(i, diag)| {
                let is_selected = i == app.diagnostic_view.selected_index;
                let (severity_icon, severity_color) = match diag.severity {
                    crate::state::DiagnosticSeverity::Error => ("✗", Color::Red),
                    crate::state::DiagnosticSeverity::Warning => ("⚠", Color::Yellow),
                    crate::state::DiagnosticSeverity::Info => ("ℹ", Color::Blue),
                    crate::state::DiagnosticSeverity::Hint => ("💡", Color::Cyan),
                };

                let style = if is_selected {
                    Style::default()
                        .bg(theme.highlight_bg)
                        .fg(theme.highlight_fg)
                } else {
                    Style::default()
                };

                let file_short = diag.file_path.rsplit('/').next().unwrap_or(&diag.file_path);

                ListItem::new(Line::from(vec![
                    Span::styled(severity_icon, Style::default().fg(severity_color)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{}:{}: ", file_short, diag.line + 1),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        if diag.message.len() > 60 {
                            format!("{}...", &diag.message[..57])
                        } else {
                            diag.message.clone()
                        },
                        style,
                    ),
                ]))
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(app.diagnostic_view.selected_index));

        let diag_list = List::new(items).highlight_symbol(">> ");
        frame.render_stateful_widget(diag_list, chunks[1], &mut list_state);
    }

    let send_prompt_hint = Paragraph::new(Line::from(vec![
        Span::styled(
            "[S] Send All to Agent  ",
            Style::default().fg(Color::Green).bold(),
        ),
        Span::styled(
            "- Copy diagnostics to clipboard for debugging",
            Style::default().fg(theme.muted),
        ),
    ]));
    frame.render_widget(send_prompt_hint, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[allow(dead_code)]
struct LspEntry {
    session_id: String,
    lsp_name: String,
    status: LspStatus,
    session_title: String,
    is_running: bool,
}

pub fn generate_agent_prompt(
    diagnostics: &[crate::state::LspDiagnosticDetail],
    project_path: &str,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("# LSP Diagnostic Issues Report\n\n");
    prompt.push_str(&format!("Project: {}\n\n", project_path));

    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, crate::state::DiagnosticSeverity::Error))
        .collect();
    let warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, crate::state::DiagnosticSeverity::Warning))
        .collect();

    if !errors.is_empty() {
        prompt.push_str(&format!("## Errors ({})\n\n", errors.len()));
        for diag in &errors {
            prompt.push_str(&format!(
                "- **{}:{}:{}** [{}]: {}\n",
                diag.file_path,
                diag.line + 1,
                diag.column + 1,
                diag.source.as_deref().unwrap_or("LSP"),
                diag.message
            ));
            if let Some(code) = &diag.code {
                prompt.push_str(&format!("  - Code: `{}`\n", code));
            }
        }
        prompt.push('\n');
    }

    if !warnings.is_empty() {
        prompt.push_str(&format!("## Warnings ({})\n\n", warnings.len()));
        for diag in &warnings {
            prompt.push_str(&format!(
                "- **{}:{}:{}**: {}\n",
                diag.file_path,
                diag.line + 1,
                diag.column + 1,
                diag.message
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Instructions\n\n");
    prompt.push_str("Please analyze and fix all issues listed above:\n\n");
    prompt.push_str("1. **Fix all errors** - Address each error with proper implementation\n");
    prompt.push_str("2. **Resolve warnings** - Fix or acknowledge warnings as appropriate\n");
    prompt.push_str("3. **Dead/Orphaned Code** - Only remove code if genuinely unused across the entire codebase\n");
    prompt.push_str("4. **Import Management** - Add missing imports, remove unused ones\n");
    prompt.push_str(
        "5. **Method/Function Implementation** - Ensure all methods are properly implemented\n\n",
    );
    prompt.push_str("After fixes, verify the code compiles and tests pass.\n");

    prompt
}
