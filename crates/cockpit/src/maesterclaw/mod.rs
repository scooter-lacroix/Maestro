//! MaesterClaw command center modules
//!
//! This module provides the command center functionality for the MaesterClaw tab,
//! including channel management, gateway connections, and hot cache for suggestions.

pub mod agent_status;
pub mod channels;
pub mod gateway;
pub mod hot_cache;
pub mod wizard;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_integration_tests;

pub use agent_status::{AgentStatus, SessionDisplay, TurnDisplay};
pub use channels::{ChannelConfig, ChannelControlPlane, ChannelStatus, ChannelType};
pub use gateway::{ConnectedClient, GatewayAuthStatus, GatewayConfig, GatewayControlPlane};
pub use hot_cache::{clamp_flash, BufferedSuggestion, HotCache, MemorySuggestion, SuggestionTtl};
pub use wizard::{SetupWizard, WizardStep};

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

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
}

impl MaestroClawPane {
    /// Create a new MaestroClaw pane
    pub fn new() -> Self {
        let mut wizard = SetupWizard::new();
        wizard.detect_tools();
        Self {
            selected_session: None,
            session_count: 0,
            wizard_active: false,
            wizard,
            show_wizard: true,
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

    /// Activate the setup wizard
    pub fn activate_wizard(&mut self) {
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
    pub fn should_show_wizard(&self, _force: bool) -> bool {
        if _force {
            return true;
        }
        self.show_wizard && !self.wizard.is_dismissed() && !self.wizard.is_completed()
    }

    /// Set whether to show the wizard
    pub fn set_show_wizard(&mut self, show: bool) {
        self.show_wizard = show;
    }

    /// Render the MaestroClaw pane
    pub fn render(&self, frame: &mut Frame, area: Rect, _app: &crate::app::App) {
        if self.wizard_active {
            self.render_wizard(frame, area);
        } else {
            self.render_main_view(frame, area);
        }
    }

    /// Render the main MaestroClaw view
    fn render_main_view(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" MaestroClaw ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let text = if self.session_count == 0 {
            Text::from(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("No sessions active.", Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Press 'n' to create a new session or 'w' to run the setup wizard."),
            ])
        } else {
            Text::from(vec![
                Line::from(""),
                Line::from(format!("Active sessions: {}", self.session_count)),
                Line::from(""),
                Line::from("Use arrow keys to navigate, Enter to open, 'n' for new session."),
            ])
        };

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true }).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Render the setup wizard
    fn render_wizard(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" MaestroClaw Setup Wizard ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.wizard.current_step() {
            WizardStep::Welcome => {
                let text = Text::from(vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Welcome to MaestroClaw!",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("This wizard will help you set up MaestroClaw for optimal performance."),
                    Line::from(""),
                    Line::from(format!(
                        "Detected {} tools on your system.",
                        self.wizard.available_tools.len()
                    )),
                    Line::from(""),
                    Line::from("Press Enter to continue or Esc to dismiss."),
                ]);
                let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, inner);
            }
            WizardStep::ToolDetection => {
                let items: Vec<ListItem> = self
                    .wizard
                    .available_tools
                    .iter()
                    .map(|tool| ListItem::new(tool.as_str()))
                    .collect();
                let list = List::new(items)
                    .block(Block::default().title("Detected Tools").borders(Borders::ALL));
                frame.render_widget(list, inner);
            }
            WizardStep::Configuration => {
                let text = Text::from(vec![
                    Line::from(""),
                    Line::from("Configuration options will appear here."),
                    Line::from(""),
                    Line::from("Press Enter to complete setup."),
                ]);
                let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, inner);
            }
            WizardStep::Complete => {
                let text = Text::from(vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Setup Complete!",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("MaestroClaw is now ready to use."),
                    Line::from(""),
                    Line::from("Press Enter to close this wizard."),
                ]);
                let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, inner);
            }
        }
    }

    /// Handle key events for the MaestroClaw pane
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> MaestroClawAction {
        if self.wizard_active {
            match key {
                crossterm::event::KeyCode::Enter => {
                    self.wizard.next_step();
                    if self.wizard.is_completed() {
                        self.wizard_active = false;
                        MaestroClawAction::WizardComplete
                    } else {
                        MaestroClawAction::WizardAdvanced
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    self.wizard_active = false;
                    self.wizard.dismiss();
                    MaestroClawAction::WizardDismissed
                }
                crossterm::event::KeyCode::Left | crossterm::event::KeyCode::BackTab => {
                    self.wizard.previous_step();
                    MaestroClawAction::WizardBack
                }
                _ => MaestroClawAction::None,
            }
        } else {
            match key {
                crossterm::event::KeyCode::Char('n') => MaestroClawAction::NewSession,
                crossterm::event::KeyCode::Char('w') => {
                    self.activate_wizard();
                    MaestroClawAction::StartSetup
                }
                crossterm::event::KeyCode::Enter => MaestroClawAction::OpenSelected,
                crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Down => {
                    MaestroClawAction::Navigate
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
}

impl Default for MaestroClawPane {
    fn default() -> Self {
        Self::new()
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
