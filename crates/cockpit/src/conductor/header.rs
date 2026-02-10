use super::model::ConductorState;
use super::theme::ConductorTheme;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Render a compact, one-line status bar for the Conductor
pub fn render_header(frame: &mut Frame, area: Rect, state: &ConductorState) {
    let theme = ConductorTheme::default();

    let status_color = match state.status {
        super::model::ConductorStatus::Running | super::model::ConductorStatus::Executing => {
            theme.status_success
        }
        super::model::ConductorStatus::Pausing | super::model::ConductorStatus::Paused => {
            theme.status_warning
        }
        super::model::ConductorStatus::Failed => theme.status_error,
        _ => theme.accent_primary,
    };

    let status_text = format!(" {:?} ", state.status).to_uppercase();
    let track_text = state.current_track.as_deref().unwrap_or("None");
    let task_text = state.current_task.as_deref().unwrap_or("Idle");

    let agent_info = if let Some(agent) = &state.active_agent {
        format!(
            " @ {} ({})",
            agent.tool,
            agent.model.as_deref().unwrap_or("default")
        )
    } else {
        "".to_string()
    };

    let progress = if state.total_tasks > 0 {
        format!(" [{}/{}]", state.tasks_completed, state.total_tasks)
    } else {
        "".to_string()
    };

    let iteration = if state.current_iteration > 0 {
        format!(" iter:{}", state.current_iteration)
    } else {
        "".to_string()
    };

    let content = Line::from(vec![
        Span::styled(
            status_text,
            Style::default().bg(status_color).fg(Color::Black).bold(),
        ),
        Span::styled(
            format!(" Track: {} ", track_text),
            Style::default().fg(theme.accent_secondary).bold(),
        ),
        Span::styled(
            format!("> Task: {} ", task_text),
            Style::default().fg(theme.fg_primary),
        ),
        Span::styled(agent_info, Style::default().fg(theme.fg_muted)),
        Span::styled(iteration, Style::default().fg(theme.fg_muted).italic()),
        Span::styled(progress, Style::default().fg(theme.accent_primary)),
    ]);

    let paragraph = Paragraph::new(content).style(Style::default().bg(theme.bg_primary));

    frame.render_widget(paragraph, area);
}
