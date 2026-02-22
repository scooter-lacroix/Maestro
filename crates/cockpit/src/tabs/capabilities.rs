//! MaesterClaw tab rendering for Cockpit TUI
//!
//! This tab displays MaesterClaw operational capabilities:
//! - Cron Jobs (scheduled tasks)
//! - MCP Servers (external tool integrations)
//! - Sandbox Status (security policies)

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::state::MaesterClawSetupCheck;
use crate::theme::theme_from_name;

/// Capabilities section selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CapabilitiesSection {
    #[default]
    CronJobs,
    McpServers,
    Sandbox,
}

/// Render the capabilities tab
pub fn render_capabilities(frame: &mut Frame, app: &App) {
    let theme = app.theme();
    let area = frame.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚡ MAESTERCLAW COMMAND CENTER ")
        .border_type(BorderType::Rounded)
        .border_style(ratatui::style::Style::default().fg(theme.accent))
        .style(ratatui::style::Style::default().bg(theme.panel_bg));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Create layout: section tabs at top, content below
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Section tabs
            Constraint::Min(0),    // Content
            Constraint::Length(2), // iFlow integration hint
        ])
        .split(inner_area);

    // Render section tabs
    render_section_tabs(frame, app, chunks[0]);

    // Render content based on selected section
    render_section_content(frame, app, chunks[1]);
    let iflow_hint =
        Paragraph::new(" [W] Setup Wizard   |   iFlow non-interactive: iflow -p \"<prompt>\" ")
            .alignment(Alignment::Center)
            .style(ratatui::style::Style::default().fg(theme.muted));
    frame.render_widget(iflow_hint, chunks[2]);

    if app.maesterclaw_setup.is_open {
        render_setup_wizard(frame, app);
    }
}

fn render_section_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let tabs = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    let section = app.capabilities_section.unwrap_or_default();

    // Cron Jobs tab
    let cron_style = if section == CapabilitiesSection::CronJobs {
        ratatui::style::Style::default()
            .fg(theme.warning)
            .bold()
            .bg(theme.highlight_bg)
    } else {
        ratatui::style::Style::default().fg(theme.fg)
    };
    let cron_count = app.cron_jobs.len();
    let cron_tab = Paragraph::new(format!(
        " {} ⏰ Cron Jobs ({}) ",
        if section == CapabilitiesSection::CronJobs {
            "▶"
        } else {
            " "
        },
        cron_count
    ))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(cron_style),
    );
    frame.render_widget(cron_tab, tabs[0]);

    // MCP Servers tab
    let mcp_style = if section == CapabilitiesSection::McpServers {
        ratatui::style::Style::default()
            .fg(theme.warning)
            .bold()
            .bg(theme.highlight_bg)
    } else {
        ratatui::style::Style::default().fg(theme.fg)
    };
    let mcp_tab = Paragraph::new(format!(
        " {} 🔌 MCP Servers ",
        if section == CapabilitiesSection::McpServers {
            "▶"
        } else {
            " "
        }
    ))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(mcp_style),
    );
    frame.render_widget(mcp_tab, tabs[1]);

    // Sandbox tab
    let sandbox_style = if section == CapabilitiesSection::Sandbox {
        ratatui::style::Style::default()
            .fg(theme.warning)
            .bold()
            .bg(theme.highlight_bg)
    } else {
        ratatui::style::Style::default().fg(theme.fg)
    };
    let sandbox_tab = Paragraph::new(format!(
        " {} 🔒 Sandbox ",
        if section == CapabilitiesSection::Sandbox {
            "▶"
        } else {
            " "
        }
    ))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(sandbox_style),
    );
    frame.render_widget(sandbox_tab, tabs[2]);
}

fn render_section_content(frame: &mut Frame, app: &App, area: Rect) {
    let theme = theme_from_name(&app.config.theme);
    let section = app.capabilities_section.unwrap_or_default();

    match section {
        CapabilitiesSection::CronJobs => render_cron_jobs(frame, app, area, theme),
        CapabilitiesSection::McpServers => render_mcp_servers(frame, app, area, theme),
        CapabilitiesSection::Sandbox => render_sandbox(frame, app, area, theme),
    }
}

fn render_setup_wizard(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width.min(96).max(60);
    let height = area.height.min(24).max(14);
    let popup = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );

    let current = app
        .maesterclaw_setup
        .steps
        .get(app.maesterclaw_setup.current_step)
        .cloned();

    let mut lines = vec![
        Line::from(""),
        Line::from(format!(
            "Step {}/{}",
            app.maesterclaw_setup.current_step + 1,
            app.maesterclaw_setup.steps.len()
        )),
        Line::from(""),
    ];

    if let Some(step) = current {
        lines.push(Line::from(step.title.to_string()));
        lines.push(Line::from(step.description));
        lines.push(Line::from(""));
        lines.push(Line::from(format!("Verification: {}", step.verification)));
        lines.push(Line::from(format!(
            "Status: {}",
            match (step.check, step.is_ready) {
                (_, true) => "READY",
                (MaesterClawSetupCheck::ManualAcknowledge, false) => "AWAITING ACK",
                (MaesterClawSetupCheck::CronConfigured, false) => "ADD CRON JOB",
                (MaesterClawSetupCheck::McpConnected, false) => "CONNECT MCP/PROVIDER",
                (MaesterClawSetupCheck::MemoryVisualizationAvailable, false) => "VERIFY MEMORY TAB",
                (MaesterClawSetupCheck::SandboxPolicyVisible, false) => "VERIFY SANDBOX PANEL",
            }
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Checklist:"));
    for step in &app.maesterclaw_setup.steps {
        let marker = if step.is_ready { "[x]" } else { "[ ]" };
        lines.push(Line::from(format!("{} {}", marker, step.title)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Controls: Enter/Right next • Left back • Esc close",
    ));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" MaesterClaw Setup Wizard ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, popup);
}

fn render_cron_jobs(frame: &mut Frame, app: &App, area: Rect, theme: crate::theme::Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Jobs list
            Constraint::Length(3), // Help text
        ])
        .split(area);

    // Jobs table header
    let header = Row::new(vec![
        Cell::from("ID").style(ratatui::style::Style::default().bold()),
        Cell::from("Name").style(ratatui::style::Style::default().bold()),
        Cell::from("Schedule").style(ratatui::style::Style::default().bold()),
        Cell::from("Type").style(ratatui::style::Style::default().bold()),
        Cell::from("Enabled").style(ratatui::style::Style::default().bold()),
    ])
    .style(ratatui::style::Style::default().fg(theme.accent));

    // Build rows from actual cron jobs
    let rows: Vec<Row> = app
        .cron_jobs
        .iter()
        .map(|job| {
            let schedule_str = match &job.schedule {
                maestro_core::Schedule::Cron { expr, .. } => expr.clone(),
                maestro_core::Schedule::At { at } => {
                    format!("at {}", at.format("%Y-%m-%d %H:%M"))
                }
                maestro_core::Schedule::Every { every_ms, .. } => {
                    format!("every {}ms", every_ms)
                }
            };

            let type_str = match job.job_type {
                maestro_core::JobType::Shell => "Shell",
                maestro_core::JobType::Agent => "Agent",
            };

            let enabled_str = if job.enabled { "●" } else { "○" };
            let enabled_style = if job.enabled {
                ratatui::style::Style::default().fg(theme.success)
            } else {
                ratatui::style::Style::default().fg(theme.muted)
            };

            Row::new(vec![
                Cell::from(job.id.as_str()),
                Cell::from(job.name.as_deref().unwrap_or("-")),
                Cell::from(schedule_str),
                Cell::from(type_str),
                Cell::from(enabled_str).style(enabled_style),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(18),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Scheduled Jobs ({}) ", app.cron_jobs.len()))
            .border_style(ratatui::style::Style::default().fg(theme.accent)),
    );

    frame.render_widget(table, chunks[0]);

    // Help text
    let help = Paragraph::new(" [N] New Job  [E] Edit  [D] Delete  [T] Toggle  [R] Run Now ")
        .alignment(Alignment::Center)
        .style(ratatui::style::Style::default().fg(theme.muted));
    frame.render_widget(help, chunks[1]);
}

fn render_mcp_servers(frame: &mut Frame, app: &App, area: Rect, theme: crate::theme::Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Servers list
            Constraint::Length(3), // Help text
        ])
        .split(area);

    // Get server status from McpManager (non-blocking)
    let (registered, connected) = app.mcp_manager.try_get_status();

    let servers: Vec<ListItem> = if registered.is_empty() {
        // No registered servers - show hint
        vec![
            ListItem::new(" No MCP servers registered")
                .style(ratatui::style::Style::default().fg(theme.muted)),
            ListItem::new(" Register servers via config or [A] Add Server")
                .style(ratatui::style::Style::default().fg(theme.muted)),
        ]
    } else {
        // Show registered servers with connection status
        registered
            .iter()
            .map(|name| {
                let is_connected = connected.contains(name);
                let status = if is_connected { "●" } else { "○" };
                let status_text = if is_connected {
                    "Ready"
                } else {
                    "Not Connected"
                };
                let style = if is_connected {
                    ratatui::style::Style::default().fg(theme.success)
                } else {
                    ratatui::style::Style::default().fg(theme.muted)
                };
                ListItem::new(format!(" {} {:<20} [{}]", status, name, status_text)).style(style)
            })
            .collect()
    };

    let title = format!(" MCP Servers ({}/{}) ", connected.len(), registered.len());
    let list = List::new(servers).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(ratatui::style::Style::default().fg(theme.accent)),
    );

    frame.render_widget(list, chunks[0]);

    // Help text
    let help = Paragraph::new(" [A] Add Server  [C] Connect  [X] Disconnect  [R] Refresh Tools ")
        .alignment(Alignment::Center)
        .style(ratatui::style::Style::default().fg(theme.muted));
    frame.render_widget(help, chunks[1]);
}

fn render_sandbox(frame: &mut Frame, app: &App, area: Rect, theme: crate::theme::Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Security policy
            Constraint::Length(6), // Available runtimes
            Constraint::Min(0),    // Help
        ])
        .split(area);

    // Get the actual security policy
    let policy = app.sandbox_manager.default_policy();

    // Security Policy section
    let policy_block = Block::default()
        .borders(Borders::ALL)
        .title(" Security Policy ")
        .border_style(ratatui::style::Style::default().fg(theme.accent));

    let policy_inner = policy_block.inner(chunks[0]);
    frame.render_widget(policy_block, chunks[0]);

    let autonomy_str = match policy.autonomy_level {
        maestro_core::AutonomyLevel::HumanApproval => "Human Approval",
        maestro_core::AutonomyLevel::Supervised => "Supervised",
        maestro_core::AutonomyLevel::Autonomous => "Autonomous",
    };
    let autonomy_color = match policy.autonomy_level {
        maestro_core::AutonomyLevel::HumanApproval => theme.warning,
        maestro_core::AutonomyLevel::Supervised => theme.accent,
        maestro_core::AutonomyLevel::Autonomous => theme.success,
    };

    let memory_str = if policy.max_memory_bytes > 0 {
        format!("{} MB", policy.max_memory_bytes / (1024 * 1024))
    } else {
        "Unlimited".to_string()
    };

    let network_str = if policy.allow_network {
        "Enabled"
    } else {
        "Disabled"
    };
    let network_color = if policy.allow_network {
        theme.success
    } else {
        theme.warning
    };

    let policy_lines = vec![
        Line::from(vec![
            Span::styled(
                "Autonomy Level: ",
                ratatui::style::Style::default().fg(theme.muted),
            ),
            Span::styled(
                autonomy_str,
                ratatui::style::Style::default().fg(autonomy_color).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Memory Limit: ",
                ratatui::style::Style::default().fg(theme.muted),
            ),
            Span::raw(memory_str),
        ]),
        Line::from(vec![
            Span::styled(
                "CPU Shares: ",
                ratatui::style::Style::default().fg(theme.muted),
            ),
            Span::raw(format!("{}", policy.max_cpu_shares)),
        ]),
        Line::from(vec![
            Span::styled(
                "Network: ",
                ratatui::style::Style::default().fg(theme.muted),
            ),
            Span::styled(
                network_str,
                ratatui::style::Style::default().fg(network_color),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Timeout: ",
                ratatui::style::Style::default().fg(theme.muted),
            ),
            Span::raw(format!("{:?}", policy.max_execution_time)),
        ]),
    ];

    let policy_para = Paragraph::new(policy_lines)
        .block(Block::default().padding(ratatui::widgets::Padding::new(1, 1, 1, 1)));
    frame.render_widget(policy_para, policy_inner);

    // Available Runtimes section
    let runtimes_block = Block::default()
        .borders(Borders::ALL)
        .title(" Available Runtimes ")
        .border_style(ratatui::style::Style::default().fg(theme.accent));

    let runtimes_inner = runtimes_block.inner(chunks[1]);
    frame.render_widget(runtimes_block, chunks[1]);

    let runtimes = app.sandbox_manager.available_runtimes();
    let runtime_items: Vec<ListItem> = runtimes
        .iter()
        .map(|r| match *r {
            "native" => {
                ListItem::new(" ● native      Native process execution (trusted code only)")
                    .style(ratatui::style::Style::default().fg(theme.success))
            }
            "wasm" => ListItem::new(" ○ wasm        WASM sandbox (experimental)")
                .style(ratatui::style::Style::default().fg(theme.muted)),
            "docker" => {
                ListItem::new(" ○ docker      Docker container isolation (requires Docker)")
                    .style(ratatui::style::Style::default().fg(theme.muted))
            }
            _ => ListItem::new(format!(" ○ {}      Available", r))
                .style(ratatui::style::Style::default().fg(theme.muted)),
        })
        .collect();

    let runtimes_list = List::new(runtime_items);
    frame.render_widget(runtimes_list, runtimes_inner);

    // Help text
    let help = Paragraph::new(" [P] Change Policy  [W] Enable WASM  [D] Enable Docker ")
        .alignment(Alignment::Center)
        .style(ratatui::style::Style::default().fg(theme.muted));
    frame.render_widget(help, chunks[2]);
}
