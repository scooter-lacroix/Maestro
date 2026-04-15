//! MaesterClaw command center modules
//!
//! This module provides the command center functionality for the MaesterClaw tab,
//! including channel management, gateway connections, and hot cache for suggestions.

pub mod agent_status;
pub mod channels;
pub mod checklist;
pub mod claw_loop;
pub mod gateway;
pub mod hot_cache;
pub mod provider_selector;
pub mod pty_bridge;
pub mod session_browser;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_integration_tests;
pub mod wizard;

pub use agent_status::{AgentStatus, SessionDisplay, TurnDisplay};
pub use channels::{ChannelConfig, ChannelControlPlane, ChannelStatus, ChannelType};
pub use checklist::Checklist;
pub use gateway::{ConnectedClient, GatewayAuthStatus, GatewayConfig, GatewayControlPlane};
pub use hot_cache::{clamp_flash, BufferedSuggestion, HotCache, MemorySuggestion, SuggestionTtl};
pub use session_browser::{SessionBrowser, SessionEntry};
pub use wizard::{SetupWizard, WizardStep};

use chrono;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use std::collections::HashSet;

fn channel_credentials_available(channel: ChannelType) -> bool {
    match channel {
        ChannelType::Telegram => std::env::var("TELEGRAM_BOT_TOKEN").is_ok(),
        ChannelType::Discord => {
            std::env::var("DISCORD_BOT_TOKEN").is_ok() && std::env::var("DISCORD_GUILD_ID").is_ok()
        }
        ChannelType::Slack => {
            std::env::var("SLACK_BOT_TOKEN").is_ok() && std::env::var("SLACK_APP_TOKEN").is_ok()
        }
        ChannelType::Matrix => {
            std::env::var("MATRIX_ACCESS_TOKEN").is_ok()
                && std::env::var("MATRIX_HOMESERVER_URL").is_ok()
        }
        ChannelType::WhatsApp => {
            std::env::var("WHATSAPP_BRIDGE_URL").is_ok()
                && std::env::var("WHATSAPP_API_TOKEN").is_ok()
        }
        ChannelType::Mattermost => {
            std::env::var("MATTERMOST_TOKEN").is_ok()
                && std::env::var("MATTERMOST_SERVER_URL").is_ok()
        }
    }
}

/// Active claw session state
#[derive(Debug, Clone)]
pub struct ClawSession {
    pub id: String,
    pub tool: String,
    pub model: Option<String>,
    pub status: ClawSessionStatus,
    pub iteration: u64,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub tokens_used: u64,
    pub cost_estimate: f64,
    pub provider_profile: String,
    pub analysis_provider: String,
    pub memory_provider: String,
    pub suppression_policy: String,
    pub cli_overlap_profile: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClawSessionStatus {
    Starting,
    Running,
    Paused,
    Idle,
    Stopped,
    Error,
}

#[derive(Debug, Clone)]
pub struct AgentOutputLine {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub content: String,
    pub line_type: OutputLineType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputLineType {
    AgentText,
    ToolCall,
    ToolResult,
    UserInput,
    SystemMessage,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClawViewMode {
    Agent,
    Cron,
    Channels,
    Setup,
    SessionBrowser,
}

#[derive(Debug, Clone)]
pub struct CronJobDisplay {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ChannelStatusDisplay {
    pub channel_type: String,
    pub connected: bool,
    pub last_message: Option<String>,
    pub config_status: String,
}

/// Which sub-pane within the MaestroClaw view currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Left sidebar – session rail
    SessionRail,
    /// Center column – agent output + input
    AgentPanel,
    /// Right column – quick actions / status / setup
    CommandPalette,
}

impl Default for FocusedPane {
    fn default() -> Self {
        Self::AgentPanel
    }
}

impl FocusedPane {
    pub fn next(self) -> Self {
        match self {
            Self::SessionRail => Self::AgentPanel,
            Self::AgentPanel => Self::CommandPalette,
            Self::CommandPalette => Self::SessionRail,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::SessionRail => Self::CommandPalette,
            Self::AgentPanel => Self::SessionRail,
            Self::CommandPalette => Self::AgentPanel,
        }
    }
}

/// MaestroClaw pane state for the TUI
#[derive(Debug, Clone)]
pub struct MaestroClawPane {
    pub selected_session: Option<usize>,
    session_count: usize,
    wizard_active: bool,
    /// The setup wizard
    pub wizard: SetupWizard,
    /// Whether to show the wizard (can be suppressed)
    show_wizard: bool,
    /// The session browser overlay
    session_browser: SessionBrowser,
    /// Active claw session
    pub claw_session: Option<ClawSession>,
    /// Agent output lines
    pub agent_output: Vec<AgentOutputLine>,
    /// Current user input text
    pub user_input: String,
    /// Cursor position in user input
    pub input_cursor: usize,
    /// Output scroll offset
    pub output_scroll: u16,
    /// Current view mode
    pub view_mode: ClawViewMode,
    /// Cron job display entries
    pub cron_jobs: Vec<CronJobDisplay>,
    /// Channel status display entries
    pub channel_statuses: Vec<ChannelStatusDisplay>,
    /// Which sub-pane has keyboard focus
    pub focused_pane: FocusedPane,
}

impl MaestroClawPane {
    /// Create a new MaestroClaw pane with the given workspace directory.
    ///
    /// The `workspace_dir` should come from `Config::default().workspace_dir`
    /// (or `Config::load()?.workspace_dir`), keeping a single source of truth
    /// shared with doctor, onboarding, and gateway.
    pub fn new(workspace_dir: std::path::PathBuf) -> Self {
        let wizard = SetupWizard::new(workspace_dir);
        Self {
            selected_session: None,
            session_count: 0,
            wizard_active: false,
            wizard,
            show_wizard: true,
            session_browser: SessionBrowser::new(),
            claw_session: None,
            agent_output: Vec::new(),
            user_input: String::new(),
            input_cursor: 0,
            output_scroll: 0,
            view_mode: ClawViewMode::Agent,
            cron_jobs: Vec::new(),
            channel_statuses: Vec::new(),
            focused_pane: FocusedPane::default(),
        }
    }

    /// Sync the session count from the app
    pub fn sync_sessions(&mut self, count: usize) {
        self.session_count = count;
        // Ensure selected_session stays within bounds
        if let Some(selected) = self.selected_session {
            if selected >= count && count > 0 {
                self.selected_session = Some(count - 1);
            } else if count == 0 {
                self.selected_session = None;
            }
        }
    }

    /// Activate the setup wizard.
    ///
    /// If the wizard was previously completed or dismissed, it is reset so the
    /// user gets a fresh walkthrough instead of reopening the stale Complete
    /// screen.
    pub fn activate_wizard(&mut self) {
        self.session_browser.set_active(false);
        if self.wizard.is_completed() || self.wizard.is_dismissed() {
            self.wizard.reset();
        }
        self.wizard_active = true;
    }

    /// Check if wizard is active
    pub fn is_wizard_active(&self) -> bool {
        self.wizard_active
    }

    /// Deactivate the wizard
    pub fn deactivate_wizard(&mut self) {
        self.wizard_active = false;
    }

    /// Check if wizard should be shown
    pub fn should_show_wizard(&self, force: bool) -> bool {
        force || (self.show_wizard && !self.wizard.is_dismissed() && !self.wizard.is_completed())
    }

    /// Set whether to show the wizard
    pub fn set_show_wizard(&mut self, show: bool) {
        self.show_wizard = show;
    }

    /// Activate the session browser overlay
    pub fn activate_session_browser(&mut self) {
        self.wizard_active = false;
        self.session_browser.set_active(true);
    }

    /// Deactivate the session browser overlay
    pub fn deactivate_session_browser(&mut self) {
        self.session_browser.set_active(false);
    }

    /// Check if session browser is active
    pub fn is_session_browser_active(&self) -> bool {
        self.session_browser.is_active()
    }

    /// Load sessions into the session browser
    pub fn load_session_entries(&mut self, entries: Vec<SessionEntry>) {
        self.session_browser.load(entries);
    }

    /// Get the currently selected session ID from the browser
    pub fn selected_browser_session_id(&self) -> Option<String> {
        self.session_browser
            .selected_session_id()
            .map(|s| s.to_string())
    }

    /// Returns the session browser's current filter text (for testing).
    pub fn browser_filter_text(&self) -> &str {
        self.session_browser.filter_text()
    }

    /// Render the MaestroClaw pane
    pub fn render(&self, frame: &mut Frame, area: Rect, _app: &crate::app::App) {
        if self.wizard_active {
            self.render_wizard(frame, area);
        } else if self.session_browser.is_active() {
            self.session_browser.render(frame, area);
        } else {
            self.render_main_view(frame, area, Some(_app));
        }
    }

    /// Render the main MaestroClaw view
    fn render_main_view(&self, frame: &mut Frame, area: Rect, app: Option<&crate::app::App>) {
        let outer = Block::default()
            .title(" MaestroClaw // live harness ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(24),
                Constraint::Percentage(48),
                Constraint::Percentage(28),
            ])
            .split(inner);

        let center_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(main_chunks[1]);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Min(8),
            ])
            .split(main_chunks[2]);

        let agent_panel_focused = self.focused_pane == FocusedPane::AgentPanel;
        let command_palette_focused = self.focused_pane == FocusedPane::CommandPalette;

        self.render_session_rail(frame, main_chunks[0], app);
        self.render_agent_summary(frame, center_chunks[0], agent_panel_focused);
        self.render_agent_output(frame, center_chunks[1], agent_panel_focused);
        self.render_input_box(frame, center_chunks[2], agent_panel_focused);
        self.render_command_palette(frame, right_chunks[0], command_palette_focused);
        self.render_status_sidebar(frame, right_chunks[1], command_palette_focused);
        self.render_setup_sidebar(frame, right_chunks[2], command_palette_focused);
    }

    fn render_session_rail(&self, frame: &mut Frame, area: Rect, app: Option<&crate::app::App>) {
        let is_focused = self.focused_pane == FocusedPane::SessionRail;
        let block = Block::default()
            .title(if is_focused { " [Sessions] ● " } else { " Sessions " })
            .borders(Borders::ALL)
            .border_type(if is_focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if is_focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::LightBlue)
            });

        let mut lines = vec![
            Line::from(vec![
                Span::styled("live ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.claw_session
                        .as_ref()
                        .map(|session| session.id.as_str())
                        .unwrap_or("none"),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(""),
        ];

        if app.map(|app| app.sessions.is_empty()).unwrap_or(true) {
            lines.push(Line::from(Span::styled(
                "No sessions discovered yet.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            // SAFETY: app is guaranteed Some here — the if-branch covers both
            // None and Some(empty-sessions), so else means Some(non-empty).
            let app = app.unwrap();
            for (idx, session) in app
                .sessions
                .iter()
                .take(8)
                .enumerate()
            {
                let is_selected = self.selected_session == Some(idx);
                let is_live = self
                    .claw_session
                    .as_ref()
                    .map(|live| live.id == session.session_id)
                    .unwrap_or(false);
                let marker = if is_live {
                    "●"
                } else if is_selected {
                    "»"
                } else {
                    " "
                };
                let title = if session.title.chars().count() > 22 {
                    format!("{}...", session.title.chars().take(22).collect::<String>())
                } else {
                    session.title.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", marker),
                        Style::default().fg(if is_live {
                            Color::Green
                        } else if is_selected {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::styled(
                        title,
                        if is_selected || is_live {
                            Style::default()
                                .fg(if is_selected { Color::Yellow } else { Color::White })
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        session.tool.as_deref().unwrap_or("unknown"),
                        Style::default().fg(Color::Cyan),
                    ),
                ]));
            }
        }

        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).block(block),
            area,
        );
    }

    fn render_agent_summary(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(if focused { " [Agent Panel] ● " } else { " Agent Panel " })
            .borders(Borders::ALL)
            .border_type(if focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            });

        let status = self.claw_session.as_ref().map(|session| {
            let status = match session.status {
                ClawSessionStatus::Starting => "starting",
                ClawSessionStatus::Running => "running",
                ClawSessionStatus::Paused => "paused",
                ClawSessionStatus::Idle => "idle",
                ClawSessionStatus::Stopped => "stopped",
                ClawSessionStatus::Error => "error",
            };
            vec![
                Line::from(vec![
                    Span::styled("session ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&session.id, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("tool ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&session.tool, Style::default().fg(Color::Green)),
                    Span::styled("  status ", Style::default().fg(Color::DarkGray)),
                    Span::styled(status, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("profile ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&session.provider_profile, Style::default().fg(Color::Cyan)),
                    Span::styled("  analysis ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        &session.analysis_provider,
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("memory ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&session.memory_provider, Style::default().fg(Color::Green)),
                    Span::styled("  policy ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        &session.suppression_policy,
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("iteration ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        session.iteration.to_string(),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("  tokens ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        session.tokens_used.to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("overlap ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        &session.cli_overlap_profile,
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("started ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        session.started_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
            ]
        });

        let fallback = vec![
            Line::from(vec![Span::styled(
                "No live claw session.",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::styled(
                format!("{} tools detected", self.wizard.available_tools.len()),
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "n",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to create or ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "w",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" for setup.", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(status.unwrap_or(fallback))
                .wrap(Wrap { trim: true })
                .block(block),
            area,
        );
    }

    fn render_agent_output(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(if focused { " [Interaction Feed] ● " } else { " Interaction Feed " })
            .borders(Borders::ALL)
            .border_type(if focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Blue)
            });

        let lines: Vec<Line> = if self.agent_output.is_empty() {
            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "No agent output yet.",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![
                    Span::styled("[n]", Style::default().fg(Color::Green)),
                    Span::raw(" new session  "),
                    Span::styled("[b]", Style::default().fg(Color::Green)),
                    Span::raw(" browse  "),
                    Span::styled("[w]", Style::default().fg(Color::Green)),
                    Span::raw(" wizard"),
                ]),
            ]
        } else {
            self.agent_output
                .iter()
                .rev()
                .take(area.height.saturating_sub(2) as usize)
                .rev()
                .map(|line| {
                    let color = match line.line_type {
                        OutputLineType::AgentText => Color::White,
                        OutputLineType::ToolCall => Color::Yellow,
                        OutputLineType::ToolResult => Color::Green,
                        OutputLineType::UserInput => Color::Cyan,
                        OutputLineType::SystemMessage => Color::Blue,
                        OutputLineType::Error => Color::Red,
                    };
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", line.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(&line.content, Style::default().fg(color)),
                    ])
                })
                .collect()
        };

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: false })
                .scroll((self.output_scroll, 0))
                .block(block),
            area,
        );
    }

    fn render_input_box(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(if focused { " [Prompt] ● " } else { " Prompt " })
            .borders(Borders::ALL)
            .border_type(if focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Magenta)
            });

        let prompt = if self.user_input.is_empty() {
            "Type a prompt for the active CLI bridge..."
        } else {
            &self.user_input
        };

        frame.render_widget(
            Paragraph::new(prompt)
                .style(if self.user_input.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                })
                .block(block),
            area,
        );
    }

    fn render_command_palette(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(if focused { " [Quick Actions] ● " } else { " Quick Actions " })
            .borders(Borders::ALL)
            .border_type(if focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            });

        let lines = vec![
            Line::from("[n] new claw session"),
            Line::from("[b] browse prior sessions"),
            Line::from("[w] setup wizard"),
            Line::from("[Enter] open selected / send prompt"),
            Line::from("[PgUp/PgDn] scroll transcript"),
        ];

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_status_sidebar(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(if focused { " [Runtime Status] ● " } else { " Runtime Status " })
            .borders(Borders::ALL)
            .border_type(if focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            });

        let view_label = match self.view_mode {
            ClawViewMode::Agent => "agent",
            ClawViewMode::Cron => "cron",
            ClawViewMode::Channels => "channels",
            ClawViewMode::Setup => "setup",
            ClawViewMode::SessionBrowser => "browser",
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("view ", Style::default().fg(Color::DarkGray)),
                Span::styled(view_label, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("sessions ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.session_count.to_string(),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("cron jobs ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.cron_jobs.len().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("channels ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.channel_statuses.len().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("lines ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.agent_output.len().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_setup_sidebar(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(if focused { " [Setup Readiness] ● " } else { " Setup Readiness " })
            .borders(Borders::ALL)
            .border_type(if focused { BorderType::Double } else { BorderType::Rounded })
            .border_style(if focused {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            });

        let selected_channels = self.wizard.selected_channels.len();
        let primary_tool = self
            .wizard
            .selected_primary_tool
            .and_then(|idx| self.wizard.tool_details.get(idx))
            .map(|(name, _, _)| name.as_str())
            .unwrap_or("not selected");

        let lines = vec![
            Line::from(vec![
                Span::styled("primary tool ", Style::default().fg(Color::DarkGray)),
                Span::styled(primary_tool, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("providers ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.wizard.provider_list.len().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("channels selected ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    selected_channels.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Hermes-guided shell wrapper backed by the session command.",
                Style::default().fg(Color::DarkGray),
            )]),
        ];

        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).block(block),
            area,
        );
    }

    /// Render the setup wizard
    fn render_wizard(&self, frame: &mut Frame, area: Rect) {
        let step = self.wizard.current_step();
        let step_label = match step {
            WizardStep::Welcome => "Welcome",
            WizardStep::ToolDetection => "Tool Detection",
            WizardStep::PrimaryToolSelection => "Primary Tool",
            WizardStep::ProviderSelection => "Provider",
            WizardStep::ChannelSetup => "Channels",
            WizardStep::CronSetup => "Cron",
            WizardStep::ToolSummary => "Summary",
            WizardStep::Complete => "Complete",
        };

        let title = format!(
            " MaestroClaw Setup — {} [{}/{}] ",
            step_label,
            step.number(),
            WizardStep::TOTAL_STEPS,
        );
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Progress bar at top of inner
        let progress_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(3)])
            .split(inner);

        let progress_ratio = step.number() as f64 / WizardStep::TOTAL_STEPS as f64;
        let filled = (progress_chunks[0].width as f64 * progress_ratio) as usize;
        let bar = "█".repeat(filled)
            + &"░".repeat((progress_chunks[0].width as usize).saturating_sub(filled));
        frame.render_widget(
            Paragraph::new(bar).style(Style::default().fg(Color::Green)),
            progress_chunks[0],
        );

        // Dispatch to sub-renderers
        match self.wizard.current_step() {
            WizardStep::Welcome => self.render_wizard_welcome(frame, progress_chunks[1]),
            WizardStep::ToolDetection => {
                self.render_wizard_tool_detection(frame, progress_chunks[1])
            }
            WizardStep::PrimaryToolSelection => {
                self.render_wizard_primary_tool(frame, progress_chunks[1])
            }
            WizardStep::ProviderSelection => provider_selector::render_provider_selection(
                &self.wizard,
                frame,
                progress_chunks[1],
            ),
            WizardStep::ChannelSetup => self.render_wizard_channels(frame, progress_chunks[1]),
            WizardStep::CronSetup => self.render_wizard_cron(frame, progress_chunks[1]),
            WizardStep::ToolSummary => self.render_wizard_tool_summary(frame, progress_chunks[1]),
            WizardStep::Complete => self.render_wizard_complete(frame, progress_chunks[1]),
        }
    }

    /// Render the welcome screen
    fn render_wizard_welcome(&self, frame: &mut Frame, area: Rect) {
        let text = Text::from(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "⚡ Welcome to MaestroClaw!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "This wizard will guide you through setting up MaestroClaw",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "for optimal performance with your available tools.",
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(
                "The wizard covers:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  1.", Style::default().fg(Color::Green)),
                Span::raw(" Detect and verify AI agent tools on your system"),
            ]),
            Line::from(vec![
                Span::styled("  2.", Style::default().fg(Color::Green)),
                Span::raw(" Select your primary AI tool"),
            ]),
            Line::from(vec![
                Span::styled("  3.", Style::default().fg(Color::Green)),
                Span::raw(" Configure AI provider credentials"),
            ]),
            Line::from(vec![
                Span::styled("  4.", Style::default().fg(Color::Green)),
                Span::raw(" Set up communication channels"),
            ]),
            Line::from(vec![
                Span::styled("  5.", Style::default().fg(Color::Green)),
                Span::raw(" Configure scheduled cron automation"),
            ]),
            Line::from(vec![
                Span::styled("  6.", Style::default().fg(Color::Green)),
                Span::raw(" Review your tool availability summary"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to begin, or ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to dismiss.", Style::default().fg(Color::DarkGray)),
            ]),
        ]);
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// Render the tool detection screen
    fn render_wizard_tool_detection(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("Found {} agent tool(s):", self.wizard.tool_details.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        if self.wizard.tool_details.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No CLI agent tools found!",
                Style::default().fg(Color::Red),
            )]));
        } else {
            for (idx, (name, version, path)) in self.wizard.tool_details.iter().enumerate() {
                let version_display = version.as_deref().unwrap_or("unknown");
                let path_display = path.as_deref().unwrap_or("unknown path");

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  [{}] ", idx),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        name,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" — "),
                    Span::styled(version_display, Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(path_display, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to continue.", Style::default().fg(Color::DarkGray)),
        ]));

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// Render the primary tool selection screen
    fn render_wizard_primary_tool(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Choose your primary agent tool:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        if self.wizard.tool_details.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No tools available to select.",
                Style::default().fg(Color::Yellow),
            )]));
        } else {
            for (idx, (name, version, _path)) in self.wizard.tool_details.iter().enumerate() {
                let is_cursor = idx == self.wizard.cursor;
                let is_selected = self.wizard.selected_primary_tool == Some(idx);

                let arrow = if is_cursor { " → " } else { "   " };
                let radio = if is_selected { "●" } else { "○" };
                let radio_color = if is_selected {
                    Color::Green
                } else {
                    Color::DarkGray
                };

                let version_display = version.as_deref().unwrap_or("unknown");

                lines.push(Line::from(vec![
                    Span::styled(arrow, Style::default().fg(Color::Green)),
                    Span::styled(radio, Style::default().fg(radio_color)),
                    Span::raw(" "),
                    Span::styled(
                        name,
                        if is_cursor {
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                    Span::raw(" — "),
                    Span::styled(version_display, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::DarkGray)),
            Span::raw(" navigate  "),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" select  "),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" back"),
        ]));

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// Render the channel setup screen using Checklist widget
    fn render_wizard_channels(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(7)])
            .split(area);

        let channels = ChannelType::all();
        let channel_names: Vec<String> = channels
            .iter()
            .map(|ch| format!("{} {}", ch.icon(), ch.label()))
            .collect();
        let selected_indices: HashSet<usize> = self
            .wizard
            .selected_channels
            .iter()
            .filter_map(|ch| channels.iter().position(|c| c == ch))
            .collect();

        let mut checklist = Checklist::with_selected(
            "Select messaging channels to configure",
            channel_names,
            selected_indices,
        );
        checklist.cursor = self.wizard.cursor;
        checklist.render(frame, chunks[0]);

        let mut status_lines = vec![
            Line::from(vec![
                Span::styled(
                    "Credentials import:",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" selected channel credentials will be imported from environment when available."),
            ]),
            Line::from(""),
        ];

        for channel in channels {
            status_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} {} ", channel.icon(), channel.label()),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    if channel_credentials_available(*channel) {
                        "credentials detected"
                    } else {
                        "credentials not detected"
                    },
                    Style::default().fg(if channel_credentials_available(*channel) {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]));
        }

        frame.render_widget(
            Paragraph::new(status_lines).wrap(Wrap { trim: true }),
            chunks[1],
        );
    }

    fn render_wizard_cron(&self, frame: &mut Frame, area: Rect) {
        let options = [
            format!(
                "{} Scheduled automation",
                if self.wizard.cron_enabled {
                    "✓"
                } else {
                    "○"
                }
            ),
            "History: 25 runs".to_string(),
            "History: 50 runs".to_string(),
            "History: 100 runs".to_string(),
            "Continue".to_string(),
        ];

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Cron job configuration",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(
                    "Workspace cron file: {}",
                    self.wizard
                        .workspace_dir
                        .join("cron")
                        .join("jobs.toml")
                        .display()
                ),
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(""),
        ];

        for (idx, option) in options.iter().enumerate() {
            let is_cursor = idx == self.wizard.cursor;
            let is_selected_history = matches!(
                (idx, self.wizard.cron_max_run_history),
                (1, 25) | (2, 50) | (3, 100)
            );
            lines.push(Line::from(vec![
                Span::styled(
                    if is_cursor { " → " } else { "   " },
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    option,
                    if is_cursor {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if is_selected_history {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" toggle/select  "),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" back"),
        ]));

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
    }

    /// Render the tool summary screen
    fn render_wizard_tool_summary(&self, frame: &mut Frame, area: Rect) {
        let available_count = self
            .wizard
            .tool_summary
            .iter()
            .filter(|t| t.available)
            .count();
        let total_count = self.wizard.tool_summary.len();

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!(
                    "{}/{} tool categories available:",
                    available_count, total_count
                ),
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(""),
        ];

        for item in &self.wizard.tool_summary {
            if item.available {
                lines.push(Line::from(vec![
                    Span::styled("✓", Style::default().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled(&item.name, Style::default().fg(Color::White)),
                ]));
            } else {
                let hint = item.missing_hint.as_deref().unwrap_or("not configured");
                lines.push(Line::from(vec![
                    Span::styled("✗", Style::default().fg(Color::Red)),
                    Span::raw("  "),
                    Span::styled(&item.name, Style::default().fg(Color::White)),
                    Span::styled(format!(" ({})", hint), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to continue to completion.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// Render the completion screen
    fn render_wizard_complete(&self, frame: &mut Frame, area: Rect) {
        let text = Text::from(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "✅ Setup Complete!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("MaestroClaw is now configured and ready to use."),
            Line::from(""),
            Line::from(format!(
                "Cron automation: {} (history {})",
                if self.wizard.cron_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                self.wizard.cron_max_run_history
            )),
            Line::from(format!(
                "Selected channels: {}",
                self.wizard.selected_channels.len()
            )),
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Quick start commands:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Green)),
                Span::raw("maestro claw status  — View current status"),
            ]),
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Green)),
                Span::raw("maestro claw doctor   — Run diagnostics"),
            ]),
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Green)),
                Span::raw("maestro claw daemon   — Start background service"),
            ]),
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Green)),
                Span::raw("maestro claw cron     — Schedule automated tasks"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to close this wizard.",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ]);
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// Handle key events for the MaestroClaw pane
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> MaestroClawAction {
        // Session browser overlay takes priority over wizard and main view
        if self.session_browser.is_active() {
            return match key {
                crossterm::event::KeyCode::Esc => {
                    self.session_browser.set_active(false);
                    MaestroClawAction::SessionBrowserClose
                }
                crossterm::event::KeyCode::Up => {
                    self.session_browser.move_up();
                    MaestroClawAction::Navigate
                }
                crossterm::event::KeyCode::Down => {
                    self.session_browser.move_down();
                    MaestroClawAction::Navigate
                }
                crossterm::event::KeyCode::Enter => {
                    if self.session_browser.selected_session_id().is_some() {
                        self.session_browser.set_active(false);
                        MaestroClawAction::SessionBrowserSelect
                    } else {
                        MaestroClawAction::None
                    }
                }
                crossterm::event::KeyCode::Backspace => {
                    self.session_browser.on_backspace();
                    MaestroClawAction::None
                }
                crossterm::event::KeyCode::Char(c) => {
                    self.session_browser.on_char(c);
                    MaestroClawAction::None
                }
                _ => MaestroClawAction::None,
            };
        }

        if self.wizard_active {
            match self.wizard.current_step() {
                WizardStep::PrimaryToolSelection => match key {
                    crossterm::event::KeyCode::Up => {
                        if self.wizard.cursor > 0 {
                            self.wizard.cursor -= 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Down => {
                        let max = self.wizard.tool_details.len().saturating_sub(1);
                        if self.wizard.cursor < max {
                            self.wizard.cursor += 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Enter => {
                        if !self.wizard.tool_details.is_empty() {
                            self.wizard.selected_primary_tool = Some(self.wizard.cursor);
                        }
                        self.wizard.cursor = 0;
                        self.wizard.next_step();
                        MaestroClawAction::WizardAdvanced
                    }
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Left
                    | crossterm::event::KeyCode::BackTab => {
                        self.wizard.previous_step();
                        MaestroClawAction::WizardBack
                    }
                    _ => MaestroClawAction::None,
                },
                WizardStep::ProviderSelection => match key {
                    crossterm::event::KeyCode::Up => {
                        if self.wizard.cursor > 0 {
                            self.wizard.cursor -= 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Down => {
                        let max = self.wizard.provider_list.len().saturating_sub(1);
                        if self.wizard.cursor < max {
                            self.wizard.cursor += 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Enter => {
                        if let Some(_provider) = self.wizard.provider_list.get(self.wizard.cursor) {
                            // Selection-only: always advance regardless of is_configured
                            self.wizard.selected_provider = Some(self.wizard.cursor);
                            self.wizard.cursor = 0;
                            self.wizard.next_step();
                            MaestroClawAction::WizardAdvanced
                        } else {
                            // Out-of-range cursor — clamp and do nothing
                            self.wizard.cursor = self.wizard.provider_list.len().saturating_sub(1);
                            MaestroClawAction::None
                        }
                    }
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Left
                    | crossterm::event::KeyCode::BackTab => {
                        self.wizard.previous_step();
                        // Clamp cursor to PrimaryToolSelection's item count (tool_details)
                        let max = self.wizard.tool_details.len().saturating_sub(1);
                        if self.wizard.cursor > max {
                            self.wizard.cursor = max;
                        }
                        MaestroClawAction::WizardBack
                    }
                    _ => MaestroClawAction::None,
                },
                WizardStep::ChannelSetup => match key {
                    crossterm::event::KeyCode::Up => {
                        if self.wizard.cursor > 0 {
                            self.wizard.cursor -= 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Down => {
                        let channel_count = ChannelType::all().len();
                        if self.wizard.cursor < channel_count {
                            self.wizard.cursor += 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Char(' ') => {
                        let channels = ChannelType::all();
                        if let Some(&ch) = channels.get(self.wizard.cursor) {
                            if self.wizard.selected_channels.contains(&ch) {
                                self.wizard.selected_channels.remove(&ch);
                            } else {
                                self.wizard.selected_channels.insert(ch);
                            }
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Enter => {
                        let channel_count = ChannelType::all().len();
                        if self.wizard.cursor == channel_count {
                            self.wizard.cursor = 0;
                            self.wizard.next_step();
                            MaestroClawAction::WizardAdvanced
                        } else {
                            let channels = ChannelType::all();
                            if let Some(&ch) = channels.get(self.wizard.cursor) {
                                if self.wizard.selected_channels.contains(&ch) {
                                    self.wizard.selected_channels.remove(&ch);
                                } else {
                                    self.wizard.selected_channels.insert(ch);
                                }
                            }
                            MaestroClawAction::WizardSelection
                        }
                    }
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Left
                    | crossterm::event::KeyCode::BackTab => {
                        self.wizard.previous_step();
                        // Clamp cursor to provider list range when returning to ProviderSelection
                        if matches!(self.wizard.current_step(), WizardStep::ProviderSelection) {
                            let max = self.wizard.provider_list.len().saturating_sub(1);
                            if self.wizard.cursor > max {
                                self.wizard.cursor = max;
                            }
                        }
                        MaestroClawAction::WizardBack
                    }
                    _ => MaestroClawAction::None,
                },
                WizardStep::CronSetup => match key {
                    crossterm::event::KeyCode::Up => {
                        if self.wizard.cursor > 0 {
                            self.wizard.cursor -= 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Down => {
                        if self.wizard.cursor < 4 {
                            self.wizard.cursor += 1;
                        }
                        MaestroClawAction::WizardSelection
                    }
                    crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char(' ') => {
                        match self.wizard.cursor {
                            0 => {
                                self.wizard.cron_enabled = !self.wizard.cron_enabled;
                                MaestroClawAction::WizardSelection
                            }
                            1 => {
                                self.wizard.cron_max_run_history = 25;
                                MaestroClawAction::WizardSelection
                            }
                            2 => {
                                self.wizard.cron_max_run_history = 50;
                                MaestroClawAction::WizardSelection
                            }
                            3 => {
                                self.wizard.cron_max_run_history = 100;
                                MaestroClawAction::WizardSelection
                            }
                            _ => {
                                self.wizard.cursor = 0;
                                self.wizard.next_step();
                                MaestroClawAction::WizardAdvanced
                            }
                        }
                    }
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Left
                    | crossterm::event::KeyCode::BackTab => {
                        self.wizard.previous_step();
                        let max = ChannelType::all().len();
                        if self.wizard.cursor > max {
                            self.wizard.cursor = max;
                        }
                        MaestroClawAction::WizardBack
                    }
                    _ => MaestroClawAction::None,
                },
                _ => {
                    // Simple Enter/Esc steps (Welcome, ToolDetection, ToolSummary, Complete)
                    match key {
                        crossterm::event::KeyCode::Enter => {
                            if matches!(self.wizard.current_step(), WizardStep::Complete) {
                                // Enter on the actual Complete step closes the wizard
                                self.wizard_active = false;
                                MaestroClawAction::WizardComplete
                            } else {
                                // ToolSummary (and Welcome/ToolDetection) advance normally
                                self.wizard.next_step();
                                MaestroClawAction::WizardAdvanced
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            if matches!(self.wizard.current_step(), WizardStep::Welcome) {
                                self.wizard_active = false;
                                self.wizard.dismiss();
                                MaestroClawAction::WizardDismissed
                            } else {
                                // Complete → ToolSummary, and other steps go back normally
                                self.wizard.previous_step();
                                MaestroClawAction::WizardBack
                            }
                        }
                        crossterm::event::KeyCode::Left | crossterm::event::KeyCode::BackTab => {
                            self.wizard.previous_step();
                            MaestroClawAction::WizardBack
                        }
                        _ => MaestroClawAction::None,
                    }
                }
            }
        } else {
            match key {
                crossterm::event::KeyCode::Char('n') => MaestroClawAction::NewSession,
                crossterm::event::KeyCode::Char('w') => {
                    self.activate_wizard();
                    MaestroClawAction::StartSetup
                }
                crossterm::event::KeyCode::Char('b') => MaestroClawAction::OpenSessionBrowser,
                crossterm::event::KeyCode::Enter => MaestroClawAction::OpenSelected,
                crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Down => {
                    MaestroClawAction::Navigate
                }
                crossterm::event::KeyCode::Tab => {
                    self.focused_pane = self.focused_pane.next();
                    MaestroClawAction::FocusChanged
                }
                crossterm::event::KeyCode::BackTab => {
                    self.focused_pane = self.focused_pane.prev();
                    MaestroClawAction::FocusChanged
                }
                _ => MaestroClawAction::None,
            }
        }
    }

    /// Handle key events with session count context
    pub fn handle_key_with_session_count(
        &mut self,
        key: crossterm::event::KeyCode,
        session_count: usize,
    ) -> MaestroClawAction {
        self.sync_sessions(session_count);
        self.handle_key(key)
    }

    /// Check whether a key event should be intercepted by the session browser.
    /// Returns `true` only when the browser overlay is active, the key carries
    /// no CTRL or ALT modifier, and the key is a browser-relevant shape.
    /// Non-browser keys (BackTab, Tab, etc.) fall through to global handlers
    /// even when the browser is active.
    /// This is the shared predicate used by both the real `run_app` match guard
    /// and `route_key_browser_priority` for testing.
    pub fn should_route_to_browser(
        &self,
        modifiers: crossterm::event::KeyModifiers,
        code: crossterm::event::KeyCode,
    ) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        let is_browser_key = matches!(
            code,
            KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace
        ) || matches!(code, KeyCode::Char(c) if c != '?');
        self.session_browser.is_active()
            && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && is_browser_key
    }

    /// App-loop key routing gate: returns `Some(action)` when the session browser
    /// is active and the key should be captured by it (i.e. is an unmodified
    /// browser-supported shape). Returns `None` when the browser is inactive or
    /// the key carries CTRL/ALT modifiers that must fall through to global
    /// handlers like Ctrl+C (quit) or Ctrl+Q.
    pub fn route_key_browser_priority(
        &mut self,
        key: crossterm::event::KeyEvent,
        session_count: usize,
    ) -> Option<MaestroClawAction> {
        if !self.should_route_to_browser(key.modifiers, key.code) {
            return None;
        }
        let action = self.handle_key_with_session_count(key.code, session_count);
        Some(action)
    }
}

impl Default for MaestroClawPane {
    fn default() -> Self {
        use maestro_claw::config::Config;
        Self::new(Config::default().workspace_dir)
    }
}

#[cfg(test)]
impl MaestroClawPane {
    /// Expose the private render_wizard_channels adapter for buffer-based tests.
    /// Caller must ensure wizard is on WizardStep::ChannelSetup before calling.
    pub(crate) fn test_render_wizard_channels(&self, frame: &mut Frame, area: Rect) {
        self.render_wizard_channels(frame, area);
    }

    /// Expose the private render_wizard outer shell for buffer-based tests.
    /// Caller must set wizard_active = true and position the wizard on the
    /// desired step before calling.
    pub(crate) fn test_render_wizard(&self, frame: &mut Frame, area: Rect) {
        self.render_wizard(frame, area);
    }

    /// Expose the private render_wizard_tool_summary method for buffer-based tests.
    /// Caller must ensure wizard is on WizardStep::ToolSummary before calling.
    pub(crate) fn test_render_wizard_tool_summary(&self, frame: &mut Frame, area: Rect) {
        self.render_wizard_tool_summary(frame, area);
    }

    /// Expose the private render_main_view method for buffer-based tests.
    /// Caller must ensure wizard_active = false and session_browser inactive.
    pub(crate) fn test_render_main_view(&self, frame: &mut Frame, area: Rect) {
        self.render_main_view(frame, area, None);
    }
}

/// Actions that can be performed in the MaestroClaw pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaestroClawAction {
    /// No action
    None,
    /// Focus has changed
    FocusChanged,
    /// Navigate within the pane
    Navigate,
    /// Create a new session
    NewSession,
    /// Open the selected session
    OpenSelected,
    /// Open the session browser
    OpenSessionBrowser,
    /// A session was selected in the browser (query selected_browser_session_id for the ID)
    SessionBrowserSelect,
    /// The session browser was closed
    SessionBrowserClose,
    /// Start the setup process
    StartSetup,
    /// Repair bootstrap configuration
    RepairBootstrap,
    /// Wizard advanced to next step
    WizardAdvanced,
    /// Wizard went back
    WizardBack,
    /// Wizard selection changed
    WizardSelection,
    /// Wizard completed
    WizardComplete,
    /// Wizard was dismissed
    WizardDismissed,
}

impl Default for MaestroClawAction {
    fn default() -> Self {
        Self::None
    }
}
