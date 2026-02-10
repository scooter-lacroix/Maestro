use super::theme::ConductorTheme;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Render Ralph-style minimalist keyboard shortcuts
pub fn render_footer(frame: &mut Frame, area: Rect) {
    let theme = ConductorTheme::default();

    let keys = [
        ("s", "start"),
        ("p", "pause"),
        ("r", "resume"),
        ("n", "new track"),
        ("tab", "next tab"),
        ("j/k", "select"),
        ("space", "expand"),
        ("d", "dash"),
        ("P", "proj"),
        ("esc", "exit"),
    ];

    let mut spans = Vec::new();
    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(theme.fg_dim)));
        }
        spans.push(Span::styled(
            format!("[{}]", key),
            Style::default().fg(theme.accent_primary).bold(),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(theme.fg_secondary),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg_primary));

    frame.render_widget(paragraph, area);
}
