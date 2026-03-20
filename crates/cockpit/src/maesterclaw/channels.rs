//! Channel control plane for MaesterClaw
//!
//! Provides UI components for managing communication channels
//! (Telegram, Discord, Slack) with bind status and allowlist management.

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Channel types supported by MaesterClaw
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    Matrix,
    WhatsApp,
    Mattermost,
}

impl ChannelType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Telegram => "Telegram",
            Self::Discord => "Discord",
            Self::Slack => "Slack",
            Self::Matrix => "Matrix",
            Self::WhatsApp => "WhatsApp",
            Self::Mattermost => "Mattermost",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Telegram => "📱",
            Self::Discord => "💬",
            Self::Slack => "💼",
            Self::Matrix => "\u{1F517}",
            Self::WhatsApp => "\u{1F4F2}",
            Self::Mattermost => "\u{1F535}",
        }
    }

    pub fn setup_instructions(&self) -> &'static str {
        match self {
            Self::Telegram => "1. Message @BotFather on Telegram\n2. Send /newbot\n3. Copy the bot token",
            Self::Discord => "1. Go to discord.com/developers\n2. Create application → Bot\n3. Copy the bot token",
            Self::Slack => "1. Go to api.slack.com/apps\n2. Create app → OAuth & Permissions\n3. Copy the bot token",
            Self::Matrix => "1. Create a bot account on your Matrix server\n2. Copy the access token",
            Self::WhatsApp => "Requires Node.js bridge.\nRun 'maestro claw whatsapp' for guided setup.",
            Self::Mattermost => "1. Go to Integrations → Bot Accounts\n2. Add Bot Account\n3. Copy the bot token",
        }
    }

    pub fn token_env_var(&self) -> &'static str {
        match self {
            Self::Telegram => "TELEGRAM_BOT_TOKEN",
            Self::Discord => "DISCORD_BOT_TOKEN",
            Self::Slack => "SLACK_BOT_TOKEN",
            Self::Matrix => "MATRIX_ACCESS_TOKEN",
            Self::WhatsApp => "WHATSAPP_ENABLED",
            Self::Mattermost => "MATTERMOST_TOKEN",
        }
    }

    /// Returns all channel variants in display order.
    pub const fn all() -> &'static [ChannelType] {
        &[
            ChannelType::Telegram,
            ChannelType::Discord,
            ChannelType::Slack,
            ChannelType::Matrix,
            ChannelType::WhatsApp,
            ChannelType::Mattermost,
        ]
    }
}

/// Channel connection status
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ChannelStatus {
    /// Not configured
    #[default]
    NotConfigured,
    /// Configured but not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Error state
    Error { message: String },
}

/// Channel configuration
#[derive(Clone, Debug)]
pub struct ChannelConfig {
    /// Channel type
    pub channel_type: ChannelType,
    /// Connection status
    pub status: ChannelStatus,
    /// Bot token/credentials configured
    pub has_credentials: bool,
    /// Allowlist of user IDs
    pub allowlist: Vec<String>,
    /// Blocked user IDs
    pub blocked: Vec<String>,
    /// Webhook URL (for some channels)
    pub webhook_url: Option<String>,
}

impl ChannelConfig {
    pub fn new(channel_type: ChannelType) -> Self {
        Self {
            channel_type,
            status: ChannelStatus::default(),
            has_credentials: false,
            allowlist: Vec::new(),
            blocked: Vec::new(),
            webhook_url: None,
        }
    }

    /// Check if channel is ready to use
    pub fn is_ready(&self) -> bool {
        matches!(self.status, ChannelStatus::Connected)
    }

    /// Connect the channel
    pub fn connect(&mut self) {
        if self.has_credentials {
            self.status = ChannelStatus::Connecting;
        }
    }

    /// Mark as connected
    pub fn set_connected(&mut self) {
        self.status = ChannelStatus::Connected;
    }

    /// Disconnect the channel
    pub fn disconnect(&mut self) {
        self.status = ChannelStatus::Disconnected;
    }

    /// Set credentials
    pub fn set_credentials(&mut self, has_credentials: bool) {
        self.has_credentials = has_credentials;
        if !has_credentials {
            self.status = ChannelStatus::NotConfigured;
        }
    }

    /// Add user to allowlist
    pub fn allow_user(&mut self, user_id: String) {
        if !self.allowlist.contains(&user_id) {
            self.allowlist.push(user_id.clone());
        }
        // Remove from blocked if present
        self.blocked.retain(|id| id != &user_id);
    }

    /// Block user
    pub fn block_user(&mut self, user_id: String) {
        if !self.blocked.contains(&user_id) {
            self.blocked.push(user_id.clone());
        }
        // Remove from allowlist if present
        self.allowlist.retain(|id| id != &user_id);
    }

    /// Remove user from both lists
    pub fn remove_user(&mut self, user_id: &str) {
        self.allowlist.retain(|id| id != user_id);
        self.blocked.retain(|id| id != user_id);
    }
}

/// Channel control plane state
#[derive(Clone, Debug)]
pub struct ChannelControlPlane {
    /// All configured channels
    pub channels: Vec<ChannelConfig>,
    /// Currently selected channel index
    pub selected: usize,
}

impl Default for ChannelControlPlane {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelControlPlane {
    /// Create a new channel control plane
    ///
    /// Channel list is derived from [`ChannelType::all()`], the single
    /// source of truth for the supported set and display order.
    pub fn new() -> Self {
        let channels = ChannelType::all()
            .iter()
            .map(|&ct| ChannelConfig::new(ct))
            .collect();

        Self {
            channels,
            selected: 0,
        }
    }

    /// Get selected channel
    pub fn selected_channel(&self) -> Option<&ChannelConfig> {
        self.channels.get(self.selected)
    }

    /// Get selected channel mutably
    pub fn selected_channel_mut(&mut self) -> Option<&mut ChannelConfig> {
        self.channels.get_mut(self.selected)
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        if !self.channels.is_empty() && self.selected < self.channels.len() - 1 {
            self.selected += 1;
        }
    }

    /// Get channel by type
    pub fn get_channel(&self, channel_type: &ChannelType) -> Option<&ChannelConfig> {
        self.channels
            .iter()
            .find(|c| &c.channel_type == channel_type)
    }

    /// Get channel by type mutably
    pub fn get_channel_mut(&mut self, channel_type: &ChannelType) -> Option<&mut ChannelConfig> {
        self.channels
            .iter_mut()
            .find(|c| &c.channel_type == channel_type)
    }

    /// Count connected channels
    pub fn connected_count(&self) -> usize {
        self.channels
            .iter()
            .filter(|c| matches!(c.status, ChannelStatus::Connected))
            .count()
    }

    /// Render the channel control plane
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Channels ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        // Render channel list
        self.render_channel_list(frame, chunks[0], theme);

        // Render selected channel details
        self.render_channel_details(frame, chunks[1], theme);
    }

    fn render_channel_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .channels
            .iter()
            .enumerate()
            .map(|(i, channel)| {
                let is_selected = i == self.selected;
                let status_icon = match &channel.status {
                    ChannelStatus::NotConfigured => "○",
                    ChannelStatus::Disconnected => "○",
                    ChannelStatus::Connecting => "◐",
                    ChannelStatus::Connected => "●",
                    ChannelStatus::Error { .. } => "✗",
                };

                let status_color = match &channel.status {
                    ChannelStatus::NotConfigured => theme.muted,
                    ChannelStatus::Disconnected => theme.muted,
                    ChannelStatus::Connecting => theme.warning,
                    ChannelStatus::Connected => theme.success,
                    ChannelStatus::Error { .. } => theme.error,
                };

                let style = if is_selected {
                    Style::default().bg(theme.accent).fg(theme.bg)
                } else {
                    Style::default().fg(theme.fg)
                };

                let status_style = if is_selected {
                    Style::default().bg(theme.accent).fg(theme.bg)
                } else {
                    Style::default().fg(status_color)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", status_icon), status_style),
                    Span::styled(
                        format!(
                            "{} {}",
                            channel.channel_type.icon(),
                            channel.channel_type.label()
                        ),
                        style,
                    ),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, area);
    }

    fn render_channel_details(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let channel = match self.selected_channel() {
            Some(c) => c,
            None => {
                let paragraph = Paragraph::new("No channel selected")
                    .style(Style::default().fg(theme.muted))
                    .alignment(Alignment::Center);
                frame.render_widget(paragraph, area);
                return;
            }
        };

        let status_text = match &channel.status {
            ChannelStatus::NotConfigured => "Not configured",
            ChannelStatus::Disconnected => "Disconnected",
            ChannelStatus::Connecting => "Connecting...",
            ChannelStatus::Connected => "Connected",
            ChannelStatus::Error { message } => message,
        };

        let status_color = match &channel.status {
            ChannelStatus::NotConfigured => theme.muted,
            ChannelStatus::Disconnected => theme.muted,
            ChannelStatus::Connecting => theme.warning,
            ChannelStatus::Connected => theme.success,
            ChannelStatus::Error { .. } => theme.error,
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(theme.muted)),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("Credentials: ", Style::default().fg(theme.muted)),
                Span::styled(
                    if channel.has_credentials {
                        "Configured"
                    } else {
                        "Not set"
                    },
                    Style::default().fg(if channel.has_credentials {
                        theme.success
                    } else {
                        theme.warning
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                format!("Allowlist: {} users", channel.allowlist.len()),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                format!("Blocked: {} users", channel.blocked.len()),
                Style::default().fg(theme.fg),
            )),
        ];

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_control_plane_default() {
        let plane = ChannelControlPlane::default();
        assert_eq!(plane.channels.len(), 6);
        assert_eq!(plane.selected, 0);
    }

    #[test]
    fn test_channel_control_plane_navigation() {
        let mut plane = ChannelControlPlane::new();
        assert_eq!(plane.selected, 0);

        plane.move_down();
        assert_eq!(plane.selected, 1);

        plane.move_down();
        assert_eq!(plane.selected, 2);

        plane.move_down();
        assert_eq!(plane.selected, 3);

        plane.move_down();
        assert_eq!(plane.selected, 4);

        plane.move_down();
        assert_eq!(plane.selected, 5);

        plane.move_down(); // Should stay at max
        assert_eq!(plane.selected, 5);

        plane.move_up();
        assert_eq!(plane.selected, 4);
    }

    #[test]
    fn test_channel_config_new() {
        let config = ChannelConfig::new(ChannelType::Telegram);
        assert_eq!(config.channel_type, ChannelType::Telegram);
        assert!(matches!(config.status, ChannelStatus::NotConfigured));
        assert!(!config.has_credentials);
        assert!(config.allowlist.is_empty());
    }

    #[test]
    fn test_channel_config_connect() {
        let mut config = ChannelConfig::new(ChannelType::Telegram);
        config.connect();
        // Can't connect without credentials
        assert!(matches!(config.status, ChannelStatus::NotConfigured));

        config.set_credentials(true);
        config.connect();
        assert!(matches!(config.status, ChannelStatus::Connecting));
    }

    #[test]
    fn test_channel_config_allowlist() {
        let mut config = ChannelConfig::new(ChannelType::Telegram);

        config.allow_user("user1".to_string());
        assert_eq!(config.allowlist.len(), 1);

        config.allow_user("user2".to_string());
        assert_eq!(config.allowlist.len(), 2);

        // Duplicate should not be added
        config.allow_user("user1".to_string());
        assert_eq!(config.allowlist.len(), 2);
    }

    #[test]
    fn test_channel_config_block_user() {
        let mut config = ChannelConfig::new(ChannelType::Telegram);

        config.allow_user("user1".to_string());
        assert_eq!(config.allowlist.len(), 1);

        // Blocking should remove from allowlist
        config.block_user("user1".to_string());
        assert!(config.allowlist.is_empty());
        assert_eq!(config.blocked.len(), 1);
    }

    #[test]
    fn test_channel_config_remove_user() {
        let mut config = ChannelConfig::new(ChannelType::Telegram);

        config.allow_user("user1".to_string());
        config.block_user("user2".to_string());

        config.remove_user("user1");
        assert!(config.allowlist.is_empty());

        config.remove_user("user2");
        assert!(config.blocked.is_empty());
    }

    #[test]
    fn test_channel_type_labels() {
        assert_eq!(ChannelType::Telegram.label(), "Telegram");
        assert_eq!(ChannelType::Discord.label(), "Discord");
        assert_eq!(ChannelType::Slack.label(), "Slack");
        assert_eq!(ChannelType::Matrix.label(), "Matrix");
        assert_eq!(ChannelType::WhatsApp.label(), "WhatsApp");
        assert_eq!(ChannelType::Mattermost.label(), "Mattermost");
    }

    #[test]
    fn test_channel_status_default() {
        let status = ChannelStatus::default();
        assert!(matches!(status, ChannelStatus::NotConfigured));
    }

    #[test]
    fn test_channel_control_plane_connected_count() {
        let mut plane = ChannelControlPlane::new();
        assert_eq!(plane.connected_count(), 0);

        plane.channels[0].set_credentials(true);
        plane.channels[0].set_connected();
        assert_eq!(plane.connected_count(), 1);

        plane.channels[1].set_credentials(true);
        plane.channels[1].set_connected();
        assert_eq!(plane.connected_count(), 2);
    }

    #[test]
    fn test_channel_control_plane_get_channel() {
        let plane = ChannelControlPlane::new();

        let telegram = plane.get_channel(&ChannelType::Telegram);
        assert!(telegram.is_some());

        let discord = plane.get_channel(&ChannelType::Discord);
        assert!(discord.is_some());

        let matrix = plane.get_channel(&ChannelType::Matrix);
        assert!(matrix.is_some());

        let whatsapp = plane.get_channel(&ChannelType::WhatsApp);
        assert!(whatsapp.is_some());

        let mattermost = plane.get_channel(&ChannelType::Mattermost);
        assert!(mattermost.is_some());
    }

    #[test]
    fn test_channel_type_setup_instructions() {
        // Verify all new types return non-empty instructions
        assert!(!ChannelType::Matrix.setup_instructions().is_empty());
        assert!(!ChannelType::WhatsApp.setup_instructions().is_empty());
        assert!(!ChannelType::Mattermost.setup_instructions().is_empty());

        // Verify known substrings for correctness
        assert!(ChannelType::Matrix.setup_instructions().contains("Matrix server"));
        assert!(ChannelType::WhatsApp.setup_instructions().contains("Node.js bridge"));
        assert!(ChannelType::Mattermost.setup_instructions().contains("Bot Accounts"));
    }

    #[test]
    fn test_channel_type_token_env_var() {
        assert_eq!(ChannelType::Matrix.token_env_var(), "MATRIX_ACCESS_TOKEN");
        assert_eq!(ChannelType::WhatsApp.token_env_var(), "WHATSAPP_ENABLED");
        assert_eq!(ChannelType::Mattermost.token_env_var(), "MATTERMOST_TOKEN");
    }

    #[test]
    fn test_channel_type_icons() {
        // Verify icons are non-empty for all types
        for ct in ChannelType::all() {
            assert!(!ct.icon().is_empty(), "icon for {:?} should not be empty", ct);
        }
    }

    #[test]
    fn test_channel_type_all() {
        let all = ChannelType::all();
        assert_eq!(all, &[
            ChannelType::Telegram,
            ChannelType::Discord,
            ChannelType::Slack,
            ChannelType::Matrix,
            ChannelType::WhatsApp,
            ChannelType::Mattermost,
        ]);
    }

    #[test]
    fn test_selected_channel() {
        let plane = ChannelControlPlane::new();
        let selected = plane.selected_channel();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().channel_type, ChannelType::Telegram);
    }

    #[test]
    fn test_channel_control_plane_order_matches_all() {
        let plane = ChannelControlPlane::new();
        let expected: Vec<ChannelType> = ChannelType::all().to_vec();
        let actual: Vec<ChannelType> = plane.channels.iter().map(|c| c.channel_type).collect();
        assert_eq!(
            actual, expected,
            "ChannelControlPlane channel order must exactly match ChannelType::all()"
        );
    }
}
