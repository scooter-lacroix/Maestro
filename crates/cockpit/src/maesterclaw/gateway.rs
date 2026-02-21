//! Gateway control plane for MaesterClaw
//!
//! Provides UI components for managing the web gateway, including
//! pairing status, authentication state, and client connections.

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

/// Gateway authentication status
#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum GatewayAuthStatus {
    /// Gateway not started
    #[default]
    NotStarted,
    /// Gateway running but no pairing
    Unpaired,
    /// Pairing in progress (show code)
    Pairing { code: String, expires_in: u32 },
    /// Paired and authenticated
    Paired { client_name: String },
    /// Error state
    Error { message: String },
}


/// Gateway configuration
#[derive(Clone, Debug)]
pub struct GatewayConfig {
    /// Port the gateway is running on
    pub port: u16,
    /// Whether SSE is enabled
    pub sse_enabled: bool,
    /// Whether WebSocket is enabled
    pub websocket_enabled: bool,
    /// Number of connected clients
    pub connected_clients: usize,
    /// Maximum allowed clients
    pub max_clients: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            sse_enabled: true,
            websocket_enabled: true,
            connected_clients: 0,
            max_clients: 10,
        }
    }
}

/// Gateway control plane state
#[derive(Clone, Debug, Default)]
pub struct GatewayControlPlane {
    /// Authentication status
    pub auth_status: GatewayAuthStatus,
    /// Gateway configuration
    pub config: GatewayConfig,
    /// Whether gateway is running
    pub is_running: bool,
    /// Pair token for display
    pub pair_token: Option<String>,
    /// Connected clients list
    pub clients: Vec<ConnectedClient>,
}

/// Connected client information
#[derive(Clone, Debug)]
pub struct ConnectedClient {
    pub id: String,
    pub name: String,
    pub connected_at: String,
    pub scope: String,
}

impl GatewayControlPlane {
    /// Create a new gateway control plane
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if gateway needs pairing
    pub fn needs_pairing(&self) -> bool {
        matches!(self.auth_status, GatewayAuthStatus::Unpaired)
    }

    /// Check if gateway is ready for use
    pub fn is_ready(&self) -> bool {
        matches!(self.auth_status, GatewayAuthStatus::Paired { .. })
    }

    /// Start the gateway
    pub fn start(&mut self, port: u16) {
        self.is_running = true;
        self.config.port = port;
        self.auth_status = GatewayAuthStatus::Unpaired;
    }

    /// Stop the gateway
    pub fn stop(&mut self) {
        self.is_running = false;
        self.clients.clear();
        self.config.connected_clients = 0;
        self.auth_status = GatewayAuthStatus::NotStarted;
    }

    /// Start pairing process
    pub fn start_pairing(&mut self, code: String) {
        self.auth_status = GatewayAuthStatus::Pairing {
            code,
            expires_in: 300, // 5 minutes
        };
    }

    /// Complete pairing
    pub fn complete_pairing(&mut self, client_name: String) {
        self.auth_status = GatewayAuthStatus::Paired { client_name };
    }

    /// Cancel pairing
    pub fn cancel_pairing(&mut self) {
        self.auth_status = GatewayAuthStatus::Unpaired;
    }

    /// Set error state
    pub fn set_error(&mut self, message: String) {
        self.auth_status = GatewayAuthStatus::Error { message };
    }

    /// Add a connected client
    pub fn add_client(&mut self, client: ConnectedClient) {
        self.clients.push(client);
        self.config.connected_clients = self.clients.len();
    }

    /// Remove a connected client
    pub fn remove_client(&mut self, id: &str) {
        self.clients.retain(|c| c.id != id);
        self.config.connected_clients = self.clients.len();
    }

    /// Render the gateway control plane
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Web Gateway ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Status
                Constraint::Length(4), // Auth
                Constraint::Length(3), // Connections
                Constraint::Min(3),    // Clients
            ])
            .split(inner);

        // Render status
        self.render_status(frame, chunks[0], theme);

        // Render auth section
        self.render_auth(frame, chunks[1], theme);

        // Render connections
        self.render_connections(frame, chunks[2], theme);

        // Render clients list
        self.render_clients(frame, chunks[3], theme);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_text = if self.is_running {
            vec![
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(theme.muted)),
                    Span::styled("● Running", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Port: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(":{}", self.config.port),
                        Style::default().fg(theme.fg),
                    ),
                    Span::raw("  "),
                    Span::styled("SSE: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        if self.config.sse_enabled { "●" } else { "○" },
                        Style::default().fg(if self.config.sse_enabled { theme.success } else { theme.muted }),
                    ),
                    Span::raw("  "),
                    Span::styled("WS: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        if self.config.websocket_enabled { "●" } else { "○" },
                        Style::default().fg(if self.config.websocket_enabled { theme.success } else { theme.muted }),
                    ),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(theme.muted)),
                    Span::styled("○ Stopped", Style::default().fg(theme.muted)),
                ]),
                Line::from(Span::styled(
                    "[S] Start Gateway",
                    Style::default().fg(theme.accent),
                )),
            ]
        };

        let paragraph = Paragraph::new(status_text);
        frame.render_widget(paragraph, area);
    }

    fn render_auth(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let auth_lines = match &self.auth_status {
            GatewayAuthStatus::NotStarted => {
                vec![Line::from(Span::styled(
                    "Gateway not started",
                    Style::default().fg(theme.muted),
                ))]
            }
            GatewayAuthStatus::Unpaired => {
                vec![
                    Line::from(vec![
                        Span::styled("Auth: ", Style::default().fg(theme.muted)),
                        Span::styled("○ Unpaired", Style::default().fg(theme.warning)),
                    ]),
                    Line::from(Span::styled(
                        "[P] Start Pairing",
                        Style::default().fg(theme.accent),
                    )),
                ]
            }
            GatewayAuthStatus::Pairing { code, expires_in } => {
                vec![
                    Line::from(vec![
                        Span::styled("Auth: ", Style::default().fg(theme.muted)),
                        Span::styled("◐ Pairing", Style::default().fg(theme.warning)),
                    ]),
                    Line::from(vec![
                        Span::styled("Code: ", Style::default().fg(theme.muted)),
                        Span::styled(
                            code,
                            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" ({}s)", expires_in),
                            Style::default().fg(theme.muted),
                        ),
                    ]),
                ]
            }
            GatewayAuthStatus::Paired { client_name } => {
                vec![
                    Line::from(vec![
                        Span::styled("Auth: ", Style::default().fg(theme.muted)),
                        Span::styled("● Paired", Style::default().fg(theme.success)),
                    ]),
                    Line::from(vec![
                        Span::styled("Client: ", Style::default().fg(theme.muted)),
                        Span::styled(client_name, Style::default().fg(theme.fg)),
                    ]),
                ]
            }
            GatewayAuthStatus::Error { message } => {
                vec![
                    Line::from(vec![
                        Span::styled("Auth: ", Style::default().fg(theme.muted)),
                        Span::styled("✗ Error", Style::default().fg(theme.error)),
                    ]),
                    Line::from(Span::styled(message, Style::default().fg(theme.error))),
                ]
            }
        };

        let paragraph = Paragraph::new(auth_lines);
        frame.render_widget(paragraph, area);
    }

    fn render_connections(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let ratio = self.config.connected_clients as f64 / self.config.max_clients as f64;
        let color = if ratio < 0.5 {
            theme.success
        } else if ratio < 0.8 {
            theme.warning
        } else {
            theme.error
        };

        let label = format!(
            "{}/{} clients",
            self.config.connected_clients, self.config.max_clients
        );

        let gauge = Gauge::default()
            .block(Block::default().title(" Connections ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(color))
            .label(label)
            .ratio(ratio);

        frame.render_widget(gauge, area);
    }

    fn render_clients(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.clients.is_empty() {
            let paragraph = Paragraph::new("No connected clients")
                .style(Style::default().fg(theme.muted))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .clients
            .iter()
            .map(|client| {
                ListItem::new(Line::from(vec![
                    Span::styled("● ", Style::default().fg(theme.success)),
                    Span::styled(&client.name, Style::default().fg(theme.fg)),
                    Span::styled(
                        format!(" ({})", client.scope),
                        Style::default().fg(theme.muted),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_control_plane_default() {
        let plane = GatewayControlPlane::default();
        assert!(!plane.is_running);
        assert!(matches!(plane.auth_status, GatewayAuthStatus::NotStarted));
    }

    #[test]
    fn test_gateway_start_stop() {
        let mut plane = GatewayControlPlane::new();
        plane.start(3000);
        assert!(plane.is_running);
        assert!(matches!(plane.auth_status, GatewayAuthStatus::Unpaired));

        plane.stop();
        assert!(!plane.is_running);
        assert!(matches!(plane.auth_status, GatewayAuthStatus::NotStarted));
    }

    #[test]
    fn test_gateway_needs_pairing() {
        let mut plane = GatewayControlPlane::new();
        plane.start(3000);
        assert!(plane.needs_pairing());
    }

    #[test]
    fn test_gateway_pairing_flow() {
        let mut plane = GatewayControlPlane::new();
        plane.start(3000);
        assert!(plane.needs_pairing());

        plane.start_pairing("123456".to_string());
        assert!(matches!(plane.auth_status, GatewayAuthStatus::Pairing { .. }));

        plane.complete_pairing("test-client".to_string());
        assert!(plane.is_ready());
        assert!(!plane.needs_pairing());
    }

    #[test]
    fn test_gateway_cancel_pairing() {
        let mut plane = GatewayControlPlane::new();
        plane.start(3000);
        plane.start_pairing("123456".to_string());
        plane.cancel_pairing();
        assert!(plane.needs_pairing());
    }

    #[test]
    fn test_gateway_error_state() {
        let mut plane = GatewayControlPlane::new();
        plane.set_error("Connection failed".to_string());
        assert!(matches!(plane.auth_status, GatewayAuthStatus::Error { .. }));
    }

    #[test]
    fn test_gateway_clients() {
        let mut plane = GatewayControlPlane::new();
        plane.add_client(ConnectedClient {
            id: "1".into(),
            name: "client1".into(),
            connected_at: "2026-02-21".into(),
            scope: "read".into(),
        });
        assert_eq!(plane.clients.len(), 1);
        assert_eq!(plane.config.connected_clients, 1);

        plane.add_client(ConnectedClient {
            id: "2".into(),
            name: "client2".into(),
            connected_at: "2026-02-21".into(),
            scope: "write".into(),
        });
        assert_eq!(plane.clients.len(), 2);

        plane.remove_client("1");
        assert_eq!(plane.clients.len(), 1);
    }

    #[test]
    fn test_gateway_config_default() {
        let config = GatewayConfig::default();
        assert_eq!(config.port, 3000);
        assert!(config.sse_enabled);
        assert!(config.websocket_enabled);
        assert_eq!(config.max_clients, 10);
    }

    #[test]
    fn test_auth_status_default() {
        let status = GatewayAuthStatus::default();
        assert!(matches!(status, GatewayAuthStatus::NotStarted));
    }
}
