use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::conductor::model::ConductorState;

pub fn render_dashboard(f: &mut Frame, area: Rect, state: &ConductorState) {
    let dashboard_area = centered_rect(60, 40, area);
    f.render_widget(Clear, dashboard_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Conductor Dashboard ")
        .border_style(Style::default().fg(Color::Cyan));

    let git_info = match &state.git_info {
        Some(gi) => format!("{} ({})", gi.branch.as_deref().unwrap_or("unknown"), if gi.is_dirty { "dirty" } else { "clean" }),
        None => "Not available".to_string(),
    };

    let session_id = state.session_id.clone().unwrap_or_else(|| "N/A".to_string());
    let active_subagents = state.subagents.iter().filter(|s| s.status == crate::conductor::model::SubagentStatus::Running).count();
    let completed_tasks = state.tasks_completed;
    
    let rate_limit_info = if let Some(rl) = &state.rate_limit {
        if rl.limited_at.is_some() {
            format!("LIMITED (until {})", rl.backoff_until.map(|b| b.format("%H:%M:%S").to_string()).unwrap_or_else(|| "N/A".to_string()))
        } else {
            format!("OK ({} retries)", rl.retry_count)
        }
    } else {
        "N/A".to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Session ID:    ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(session_id),
        ]),
        Line::from(vec![
            Span::styled("Git Status:    ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(git_info),
        ]),
        Line::from(vec![
            Span::styled("Active Agents: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(active_subagents.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Completed:     ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(completed_tasks.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Rate Limit:    ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(rate_limit_info),
        ]),
        Line::from(vec![
            Span::styled("Engine Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", state.status)),
        ]),
        Line::from(vec![
            Span::styled("Model:         ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(state.active_agent.as_ref().and_then(|a| a.model.clone()).unwrap_or_else(|| "N/A".to_string())),
        ]),
        Line::from(vec![
            Span::styled("Uptime:        ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}s", state.elapsed_secs)),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, dashboard_area);
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
