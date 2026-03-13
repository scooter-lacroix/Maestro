//! MCP (Model Context Protocol) related modal rendering
//!
//! This module provides modals for MCP server management including
//! the MCP menu and MCP logs viewer.

use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::App;
use crate::state::McpOption;
use crate::theme::theme_from_name;

/// Renders the MCP menu modal.
///
/// This modal displays options for managing MCP servers:
/// - Start/Stop Server
/// - Pause Connection
/// - View Server Logs
/// - Add New Server
/// - Remove from Pool
/// - Install Component
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_mcp_menu(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(40, 40, frame.area());
    frame.render_widget(Clear, area);
    let theme = theme_from_name(&app.config.theme);

    let name = app.target_mcp_name.as_deref().unwrap_or("Unknown");
    let block = Block::default()
        .title(format!(" MCP: {} ", name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let options = vec![
        (McpOption::Start, "▶ Start Server"),
        (McpOption::Stop, "■ Stop Server"),
        (McpOption::Pause, "⏸ Pause Connection"),
        (McpOption::Logs, "📋 View Server Logs"),
        (McpOption::Add, "➕ Add New Server"),
        (McpOption::Install, "🛠 Install Managed Server"),
        (McpOption::Reinstall, "♻ Reinstall Managed Server"),
        (McpOption::Remove, "❌ Remove from Pool"),
        (McpOption::Uninstall, "🗑 Uninstall Managed Server"),
    ];

    let mut list_items = Vec::new();
    for (opt, label) in options {
        let style = if app.mcp_menu_option == opt {
            Style::default()
                .fg(Color::Yellow)
                .bold()
                .bg(Color::Rgb(40, 40, 60))
        } else {
            Style::default()
        };
        list_items.push(ListItem::new(vec![Line::from(vec![
            Span::styled(
                if app.mcp_menu_option == opt {
                    " >> "
                } else {
                    "    "
                },
                style,
            ),
            Span::styled(label, style),
        ])]));
    }

    let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
}

/// Renders the MCP logs modal.
///
/// This modal displays server logs for either MCP servers or LSP servers.
/// The content is scrollable and supports viewing log output from running
/// services.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_mcp_logs_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(80, 70, frame.area());
    frame.render_widget(Clear, area);

    // Determine if we're showing MCP or LSP logs
    let is_lsp_logs = app.lsp_log_source.is_some();

    let (title, content, scroll_offset) = if is_lsp_logs {
        // LSP logs
        let (session_id, lsp_name) = app.lsp_log_source.as_ref().unwrap();
        let title = format!(
            " LSP Logs: {} - Session {} (Esc to close) ",
            lsp_name, session_id
        );
        let content = if app.lsp_log_content.is_empty() {
            vec![
                Line::from(""),
                Line::from("  No logs found."),
                Line::from(""),
                Line::from("  Tip: LSP logs may not be enabled for this server."),
            ]
        } else {
            app.lsp_log_content.lines().map(Line::from).collect()
        };
        let scroll_offset = (app.lsp_log_scroll, 0);
        (title, content, scroll_offset)
    } else {
        // MCP logs
        let name = app.target_mcp_name.as_deref().unwrap_or("Unknown");
        let title = format!(" MCP Logs: {} (Esc to close) ", name);
        let content = if app.mcp_log_lines.is_empty() {
            vec![
                Line::from(""),
                Line::from("  No logs found."),
                Line::from(""),
                Line::from("  Tip: start the server to generate logs."),
            ]
        } else {
            app.mcp_log_lines
                .iter()
                .map(|l| Line::from(l.as_str()))
                .collect()
        };
        let scroll_offset = (app.mcp_log_scroll, 0);
        (title, content, scroll_offset)
    };

    let theme = theme_from_name(&app.config.theme);
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let para = Paragraph::new(content)
        .scroll(scroll_offset)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}
