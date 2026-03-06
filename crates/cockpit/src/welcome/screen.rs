//! Welcome screen rendering and state management

use super::wizard::WelcomeState;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Welcome screen state and configuration
#[derive(Clone, Debug)]
pub struct WelcomeScreen {
    /// Current wizard state
    pub state: WelcomeState,
    /// Input buffer for current field
    pub input_buffer: String,
    /// Currently selected editor index
    pub selected_editor: usize,
    /// Currently selected theme index
    pub selected_theme: usize,
    /// Whether to use environment variable for API key
    pub use_env_key: bool,
    /// Custom API key (if not using env)
    pub custom_key: String,
    /// Whether the screen is visible
    pub is_visible: bool,
}

impl Default for WelcomeScreen {
    fn default() -> Self {
        Self {
            state: WelcomeState::default(),
            input_buffer: super::default_workspace_path(),
            selected_editor: 0,
            selected_theme: 0,
            use_env_key: true,
            custom_key: String::new(),
            is_visible: false,
        }
    }
}

impl WelcomeScreen {
    /// Create a new welcome screen
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the welcome screen
    pub fn show(&mut self) {
        self.is_visible = true;
        if matches!(self.state, WelcomeState::NotStarted) {
            self.state.start();
        }
    }

    /// Hide the welcome screen
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Check if the screen is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Handle character input for current field
    pub fn handle_char(&mut self, c: char) {
        match self.state {
            WelcomeState::WorkspaceSetup { .. } => {
                self.input_buffer.push(c);
            }
            WelcomeState::ProviderSetup { .. } if !self.use_env_key => {
                self.custom_key.push(c);
            }
            _ => {}
        }
    }

    /// Handle backspace for current field
    pub fn handle_backspace(&mut self) {
        match self.state {
            WelcomeState::WorkspaceSetup { .. } => {
                self.input_buffer.pop();
            }
            WelcomeState::ProviderSetup { .. } if !self.use_env_key => {
                self.custom_key.pop();
            }
            _ => {}
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        match self.state {
            WelcomeState::EditorSelection { .. } => {
                if self.selected_editor > 0 {
                    self.selected_editor -= 1;
                }
            }
            WelcomeState::ThemeSelection { .. } => {
                if self.selected_theme > 0 {
                    self.selected_theme -= 1;
                }
            }
            _ => {}
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        match self.state {
            WelcomeState::EditorSelection { .. } => {
                if self.selected_editor < super::AVAILABLE_EDITORS.len() - 1 {
                    self.selected_editor += 1;
                }
            }
            WelcomeState::ThemeSelection { .. } => {
                if self.selected_theme < super::AVAILABLE_THEMES.len() - 1 {
                    self.selected_theme += 1;
                }
            }
            _ => {}
        }
    }

    /// Toggle environment variable usage
    pub fn toggle_env_key(&mut self) {
        if matches!(self.state, WelcomeState::ProviderSetup { .. }) {
            self.use_env_key = !self.use_env_key;
        }
    }

    /// Advance to next step
    pub fn advance(&mut self) {
        let _editor_name = super::AVAILABLE_EDITORS
            .get(self.selected_editor)
            .map(|(id, _)| *id)
            .unwrap_or("helix");
        let theme_name = super::AVAILABLE_THEMES
            .get(self.selected_theme)
            .map(|(id, _)| *id)
            .unwrap_or("system");

        self.state
            .advance(&self.input_buffer, self.selected_editor, theme_name);

        // Update state fields based on new state
        if let WelcomeState::WorkspaceSetup { path } = &self.state {
            self.input_buffer = path.clone();
        }
    }

    /// Go back to previous step
    pub fn go_back(&mut self) {
        self.state.go_back();
    }

    /// Check if wizard is complete
    pub fn is_complete(&self) -> bool {
        self.state.is_complete()
    }

    /// Get selected editor name
    pub fn selected_editor_name(&self) -> &'static str {
        super::AVAILABLE_EDITORS
            .get(self.selected_editor)
            .map(|(id, _)| *id)
            .unwrap_or("helix")
    }

    /// Get selected theme name
    pub fn selected_theme_name(&self) -> &'static str {
        super::AVAILABLE_THEMES
            .get(self.selected_theme)
            .map(|(id, _)| *id)
            .unwrap_or("system")
    }

    /// Render the welcome screen
    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        if !self.is_visible {
            return;
        }

        let area = frame.area();
        let popup_area = centered_rect(60, 70, area);

        // Clear the area first
        frame.render_widget(Clear, popup_area);

        // Create outer block with title
        let block = Block::default()
            .title(" Welcome to Maestro ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.bg));

        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Layout: ASCII art, step indicator, content, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(8), // ASCII art
                Constraint::Length(3), // Step indicator
                Constraint::Min(10),   // Content
                Constraint::Length(2), // Help
            ])
            .split(inner_area);

        // Render ASCII art logo
        render_logo(frame, chunks[0], theme);

        // Render step indicator
        render_step_indicator(frame, chunks[1], &self.state, theme);

        // Render content based on current step
        render_step_content(frame, chunks[2], self, theme);

        // Render help text
        render_help_text(frame, chunks[3], &self.state, theme);
    }
}

/// Create a centered rectangle
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

/// Render the ASCII art logo
fn render_logo(frame: &mut Frame, area: Rect, theme: &Theme) {
    let logo = [
        "     ███████╗ █████╗  ██████╗ ██████╗ ██████╗ ███████╗",
        "     ██╔════╝██╔══██╗██╔════╝██╔═══██╗██╔══██╗██╔════╝",
        "     █████╗  ███████║██║     ██║   ██║██║  ██║█████╗  ",
        "     ██╔══╝  ██╔══██║██║     ██║   ██║██║  ██║██╔══╝  ",
        "     ██║     ██║  ██║╚██████╗╚██████╔╝██████╔╝███████╗",
        "     ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝",
        "",
        "     Autonomous Development Cockpit v2.5",
    ];

    let lines: Vec<Line> = logo
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                *line,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Render the step indicator
fn render_step_indicator(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &Theme) {
    let current = state.current_step();
    let total = WelcomeState::total_steps();

    let step_text = if state.is_complete() {
        "Setup Complete!".to_string()
    } else if current > 0 {
        format!("[Step {} of {}]", current, total)
    } else {
        "Starting...".to_string()
    };

    let step_info = state.current_step_info();
    let title = step_info
        .as_ref()
        .map(|info| info.title.as_str())
        .unwrap_or("");

    let line = Line::from(vec![
        Span::styled(
            step_text,
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            title,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
    ]);

    let paragraph = Paragraph::new(line)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

/// Render content based on current step
fn render_step_content(frame: &mut Frame, area: Rect, screen: &WelcomeScreen, theme: &Theme) {
    let content = match &screen.state {
        WelcomeState::WorkspaceSetup { .. } => render_workspace_content(screen, theme),
        WelcomeState::EditorSelection { .. } => render_editor_content(screen, theme),
        WelcomeState::ProviderSetup { .. } => render_provider_content(screen, theme),
        WelcomeState::ThemeSelection { .. } => render_theme_content(screen, theme),
        WelcomeState::Completed => render_completed_content(theme),
        WelcomeState::NotStarted => Paragraph::new(""),
    };

    frame.render_widget(content, area);
}

/// Render workspace setup content
fn render_workspace_content(screen: &WelcomeScreen, theme: &Theme) -> Paragraph<'static> {
    let input = screen.input_buffer.clone();
    let lines: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Maestro needs a workspace directory for projects and tracks.",
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Workspace path:",
            Style::default().fg(theme.muted),
        )),
        Line::from(vec![
            Span::styled("[", Style::default().fg(theme.muted)),
            Span::styled(
                input,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", Style::default().fg(theme.muted)),
        ]),
        Line::from(""),
    ];

    Paragraph::new(lines).wrap(Wrap { trim: true })
}

/// Render editor selection content
fn render_editor_content(screen: &WelcomeScreen, theme: &Theme) -> Paragraph<'static> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Choose your preferred editor for opening files from Maestro:",
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
    ];

    for (i, (_, name)) in super::AVAILABLE_EDITORS.iter().enumerate() {
        let prefix = if i == screen.selected_editor {
            "► "
        } else {
            "  "
        };
        let style = if i == screen.selected_editor {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg)
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, name),
            style,
        )));
    }

    Paragraph::new(lines).wrap(Wrap { trim: true })
}

/// Render provider setup content
fn render_provider_content(screen: &WelcomeScreen, theme: &Theme) -> Paragraph<'static> {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Configure your AI provider API key for enhanced features.",
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This step is optional - you can configure providers later.",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                if screen.use_env_key { "●" } else { "○" },
                Style::default().fg(if screen.use_env_key {
                    theme.success
                } else {
                    theme.muted
                }),
            ),
            Span::styled(
                " Use environment variable (ANTHROPIC_API_KEY)",
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(""),
    ];

    Paragraph::new(lines).wrap(Wrap { trim: true })
}

/// Render theme selection content
fn render_theme_content(screen: &WelcomeScreen, theme: &Theme) -> Paragraph<'static> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Choose a visual theme for the TUI:",
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
    ];

    for (i, (_, name)) in super::AVAILABLE_THEMES.iter().enumerate() {
        let prefix = if i == screen.selected_theme {
            "► "
        } else {
            "  "
        };
        let style = if i == screen.selected_theme {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg)
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, name),
            style,
        )));
    }

    Paragraph::new(lines).wrap(Wrap { trim: true })
}

/// Render completed content
fn render_completed_content(theme: &Theme) -> Paragraph<'static> {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "✓ Setup Complete!",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Maestro is ready to use. Your configuration has been saved.",
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to start using Maestro.",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
    ];

    Paragraph::new(lines).wrap(Wrap { trim: true })
}

/// Render help text
fn render_help_text(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &Theme) {
    let step_info = state.current_step_info();
    let help = step_info
        .as_ref()
        .map(|info| info.help_text.as_str())
        .unwrap_or("");

    let line = Line::from(Span::styled(help, Style::default().fg(theme.muted)));
    let paragraph = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_screen_default() {
        let screen = WelcomeScreen::default();
        assert!(!screen.is_visible());
        assert!(matches!(screen.state, WelcomeState::NotStarted));
    }

    #[test]
    fn test_welcome_screen_show() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        assert!(screen.is_visible());
        assert!(!matches!(screen.state, WelcomeState::NotStarted));
    }

    #[test]
    fn test_welcome_screen_hide() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        screen.hide();
        assert!(!screen.is_visible());
    }

    #[test]
    fn test_welcome_screen_advance() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        screen.advance();
        assert!(matches!(screen.state, WelcomeState::EditorSelection { .. }));
    }

    #[test]
    fn test_welcome_screen_navigation() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        screen.state = WelcomeState::EditorSelection { selected: 0 };

        screen.move_down();
        assert_eq!(screen.selected_editor, 1);

        screen.move_up();
        assert_eq!(screen.selected_editor, 0);
    }

    #[test]
    fn test_welcome_screen_input() {
        let mut screen = WelcomeScreen::default();
        screen.show();
        screen.state = WelcomeState::WorkspaceSetup {
            path: String::new(),
        };
        screen.input_buffer = String::new();

        screen.handle_char('t');
        screen.handle_char('e');
        screen.handle_char('s');
        screen.handle_char('t');
        assert_eq!(screen.input_buffer, "test");

        screen.handle_backspace();
        assert_eq!(screen.input_buffer, "tes");
    }

    #[test]
    fn test_selected_editor_name() {
        let screen = WelcomeScreen::default();
        assert_eq!(screen.selected_editor_name(), "helix");
    }

    #[test]
    fn test_selected_theme_name() {
        let screen = WelcomeScreen::default();
        assert_eq!(screen.selected_theme_name(), "system");
    }
}
