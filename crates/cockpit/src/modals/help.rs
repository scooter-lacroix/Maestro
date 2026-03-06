//! Help modal rendering
//!
//! This module provides the help modal, which displays a comprehensive
//! keyboard shortcut and command reference for the Cockpit TUI.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::text::Line;

use crate::theme::theme_from_name;
use crate::app::App;

/// Renders the help modal.
///
/// This modal displays a comprehensive list of keyboard shortcuts and
/// commands organized by context (Global, Sessions, Memory, Projects, etc.).
/// The content is scrollable using PgUp/PgDn keys.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - Reference to the application state
pub fn render_help_modal(frame: &mut Frame, app: &App) {
    let area = crate::modals::centered_rect(60, 40, frame.area());
    let theme = theme_from_name(&app.config.theme);
    let block = Block::default()
        .title(" Commands Cheat-sheet ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(theme.panel_bg));

    let text = build_help_text(app);

    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Left)
        .scroll((app.help_scroll, 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}

/// Builds the help text content.
///
/// This function constructs the vector of lines that make up the help modal
/// content. The help is organized into sections covering different aspects
/// of the Cockpit TUI.
///
/// # Arguments
/// * `app` - Reference to the application state (used for frame_count animation)
///
/// # Returns
/// A vector of `Line` objects containing styled help text
pub fn build_help_text(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            " GLOBAL CONTROLS:",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   Tab / S-Tab   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Cycle Tabs / Focus Preview (e.g. 1->2->3)"),
        ]),
        Line::from(vec![
            Span::styled("   ↑ / ↓         ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Navigate / Scroll Preview"),
        ]),
        Line::from(vec![
            Span::styled("   / or ?        ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Open/close this modal"),
        ]),
        Line::from(vec![
            Span::styled("   PgUp/PgDn     ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Scroll modal content"),
        ]),
        Line::from(vec![
            Span::styled("   q / Ctrl-C    ", Style::default().fg(Color::Red).bold()),
            Span::raw(" Quit Maestro Cockpit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Dash: k / d   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Kill / Delete Highlighted Dashboard Session"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " SESSIONS (Tab 2):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled(
                "   n             ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" New Session Wizard (Title, Path, Tool)"),
        ]),
        Line::from(vec![
            Span::styled(
                "   Enter         ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" Attach (auto-resume if terminated)"),
        ]),
        Line::from(vec![
            Span::styled(
                "   u             ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" Resume (restore shell + resume agent, best-effort)"),
        ]),
        Line::from(vec![
            Span::styled(
                "   R             ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" Restart (restore shell + start tool fresh)"),
        ]),
        Line::from(vec![
            Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Session Hub (Rename, Move, Search history)"),
        ]),
        Line::from(vec![
            Span::styled("   Alt + p       ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Focus Preview Pane (for scrolling history)"),
        ]),
        Line::from(vec![
            Span::styled(
                "   Alt + c       ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" Create New Group"),
        ]),
        Line::from(vec![
            Span::styled("   Alt + ↑/↓     ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Reorder group/session (persists to DB)"),
        ]),
        Line::from(vec![
            Span::styled(
                "   m             ",
                Style::default().fg(Color::Magenta).bold(),
            ),
            Span::raw(" Move Session to Existing Group"),
        ]),
        Line::from(vec![
            Span::styled(
                "   G             ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" Create Standalone Group"),
        ]),
        Line::from(vec![
            Span::styled("   k             ", Style::default().fg(Color::Red).bold()),
            Span::raw(" Kill tmux Session Process"),
        ]),
        Line::from(vec![
            Span::styled("   d / Alt + D   ", Style::default().fg(Color::Red).bold()),
            Span::raw(" PURMANENT DELETE Session/Group from DB"),
        ]),
        Line::from(vec![
            Span::styled(
                "   f             ",
                Style::default().fg(Color::Magenta).bold(),
            ),
            Span::raw(" Fork Session (Clone state to new session)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " CONDUCTOR (Tab 4):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   o / O         ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Next / Previous Track"),
        ]),
        Line::from(vec![
            Span::styled("   Space         ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Toggle Task Expansion"),
        ]),
        Line::from(vec![
            Span::styled("   s             ", Style::default().fg(Color::Green).bold()),
            Span::raw(" Start Orchestrate Loop"),
        ]),
        Line::from(vec![
            Span::styled("   p             ", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" Pause Orchestrate Loop"),
        ]),
        Line::from(vec![
            Span::styled("   r             ", Style::default().fg(Color::Green).bold()),
            Span::raw(" Resume Orchestrate Loop"),
        ]),
        Line::from(vec![
            Span::styled("   x             ", Style::default().fg(Color::Red).bold()),
            Span::raw(" Abort Orchestrate Loop"),
        ]),
        Line::from(vec![
            Span::styled("   c             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Clear Output"),
        ]),
        Line::from(vec![
            Span::styled("   Alt + 1-3     ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Switch View Mode (Details, Output, Prompt)"),
        ]),
        Line::from(vec![
            Span::styled("   Alt + p       ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Toggle Focus between Tree and Output"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " PROJECTS (Tab 3):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled(
                "   Enter         ",
                Style::default().fg(Color::Green).bold(),
            ),
            Span::raw(" Open Zide (File Picker + Editor)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " MEMORY (Tab 5):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   Ctrl + f      ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Search memories (hybrid Tantivy/SQLite)"),
        ]),
        Line::from(vec![
            Span::styled("   Ctrl + l      ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Clear memory search"),
        ]),
        Line::from(vec![
            Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Refresh/import system-wide memories"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " ANALYSIS (Tab 6):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   a             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Enter Analysis Command Box"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   m             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Toggle Ultra / Balanced mode"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   1-5 / b       ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Run analysis phases / bundle"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " KRUSTOP (Tab 7):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Force refresh metrics"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Alt + p       ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Pause/Resume updates"),
        ]),
        Line::from(vec![
            Span::styled("   Alt + + / -   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Adjust refresh rate (1-10s)"),
        ]),
        Line::from(vec![
            Span::styled("   Alt + Tab     ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Cycle section focus"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " LSPs (Tab 8):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   Enter         ", Style::default().fg(Color::Green).bold()),
            Span::raw(" Toggle Start/Stop LSP"),
        ]),
        Line::from(vec![
            Span::styled("   r             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Force Restart LSP"),
        ]),
        Line::from(vec![
            Span::styled("   l             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" View LSP Logs"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " SETTINGS (Tab 9):",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(vec![
            Span::styled("   ↑ / ↓         ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" Navigate options"),
        ]),
        Line::from(vec![
            Span::styled("   Enter         ", Style::default().fg(Color::Green).bold()),
            Span::raw(" Change setting"),
        ]),
        Line::from(""),
        Line::from("  ---------------------------------- "),
        Line::from(format!(
            "  Maestro TUI Cockpit v2.0-beta-8  {}",
            if (app.frame_count / 30).is_multiple_of(2) {
                "⚡"
            } else {
                "  "
            }
        )),
    ]
}
