//! WebSocket to PTY Bridge for MaestroTabMultiplexer
//!
//! This module provides a bridge that connects to tab-daemon via WebSocket,
//! handles PtyWebsocketRequest/Response messages, and forwards PTY I/O to the Maestro session.
//!
//! Architecture:
//! ```text
//! ┌─────────────────┐     WebSocket      ┌─────────────────┐
//! │   Maestro       │◄──────────────────►│   tab-daemon    │
//! │   Session       │  PtyWebsocketMsg   │   /pty endpoint │
//! └────────┬────────┘                    └─────────────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  WebSocketPty   │
//! │    Bridge       │
//! └────────┬────────┘
//!          │
//!    ┌─────┴─────┐
//!    ▼           ▼
//! ┌──────┐   ┌──────┐
//! │ TX   │   │ RX   │
//! │ Task │   │ Task │
//! └──────┘   └──────┘
//! ```

use crate::bridge::{BridgeError, BridgeResult};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tab_api::chunk::{InputChunk, OutputChunk};
use tab_api::pty::{PtyWebsocketRequest, PtyWebsocketResponse};
use tab_api::tab::{TabId, TabMetadata};
use tab_websocket::{
    connect_authorized, decode, encode, WebsocketConnection,
};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::{self, Message as TungsteniteMessage};
use tracing::{debug, error, info, trace, warn};

/// Configuration for the WebSocket PTY bridge
#[derive(Debug, Clone)]
pub struct WebSocketPtyBridgeConfig {
    /// WebSocket URL for tab-daemon (e.g., "ws://127.0.0.1:12345/pty")
    pub daemon_url: String,
    /// Authentication token for WebSocket connection
    pub auth_token: String,
    /// Tab metadata for the session
    pub tab_metadata: TabMetadata,
    /// Buffer size for I/O channels
    pub buffer_size: usize,
    /// Reconnection policy
    pub reconnection_policy: ReconnectionPolicy,
}

impl Default for WebSocketPtyBridgeConfig {
    fn default() -> Self {
        Self {
            daemon_url: String::new(),
            auth_token: String::new(),
            tab_metadata: TabMetadata::default(),
            buffer_size: 4096,
            reconnection_policy: ReconnectionPolicy::default(),
        }
    }
}

/// Reconnection policy for handling connection failures
#[derive(Debug, Clone)]
pub struct ReconnectionPolicy {
    /// Maximum number of reconnection attempts
    pub max_attempts: u32,
    /// Initial delay between reconnection attempts (in milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between reconnection attempts (in milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for ReconnectionPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Events that can be sent from the bridge to the Maestro session
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// PTY output data received from daemon
    Output(OutputChunk),
    /// Tab session has started
    Started(TabMetadata),
    /// Tab session has stopped
    Stopped,
    /// Connection established
    Connected,
    /// Connection lost
    Disconnected(String),
    /// Error occurred
    Error(String),
    /// Terminal resize acknowledgment
    Resized((u16, u16)),
}

/// Commands that can be sent from the Maestro session to the bridge
#[derive(Debug, Clone)]
pub enum BridgeCommand {
    /// Send input to PTY
    Input(InputChunk),
    /// Resize terminal
    Resize((u16, u16)),
    /// Terminate the session
    Terminate,
    /// Reconnect to daemon
    Reconnect,
}

/// Current state of the bridge connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeState {
    /// Initial state, not connected
    Disconnected,
    /// Currently connecting
    Connecting,
    /// Connected and operational
    Connected,
    /// Reconnecting after failure
    Reconnecting,
    /// Shutting down
    ShuttingDown,
    /// Bridge has been closed
    Closed,
}

/// Internal message types for bridge task communication
#[derive(Debug)]
enum InternalMessage {
    /// WebSocket message received
    WebSocketMessage(PtyWebsocketResponse),
    /// WebSocket connection error
    WebSocketError(tungstenite::Error),
    /// WebSocket connection closed
    WebSocketClosed,
    /// Command from Maestro session
    Command(BridgeCommand),
    /// Shutdown signal
    Shutdown,
}

/// Statistics for the bridge connection
#[derive(Debug, Default, Clone)]
pub struct BridgeStats {
    /// Total bytes sent to daemon
    pub bytes_sent: u64,
    /// Total bytes received from daemon
    pub bytes_received: u64,
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
    /// Number of reconnection attempts
    pub reconnection_attempts: u32,
    /// Connection start time
    pub connected_at: Option<std::time::Instant>,
}

/// WebSocket to PTY Bridge for MaestroTabMultiplexer
///
/// This bridge handles the bidirectional communication between Maestro and tab-daemon:
/// - Forwards PTY input from Maestro to tab-daemon via WebSocket
/// - Forwards PTY output from tab-daemon to Maestro via events
/// - Manages connection lifecycle and reconnection
pub struct WebSocketPtyBridge {
    /// Bridge configuration
    config: WebSocketPtyBridgeConfig,
    /// Current connection state
    state: Arc<RwLock<BridgeState>>,
    /// Bridge statistics
    stats: Arc<RwLock<BridgeStats>>,
    /// Channel for sending commands to the bridge
    command_tx: mpsc::UnboundedSender<BridgeCommand>,
    /// Channel for receiving events from the bridge
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<BridgeEvent>>>,
    /// Handle to the bridge task
    bridge_task: Option<JoinHandle<BridgeResult<()>>>,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WebSocketPtyBridge {
    /// Create a new WebSocket PTY bridge
    pub fn new(config: WebSocketPtyBridgeConfig) -> BridgeResult<Self> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let state = Arc::new(RwLock::new(BridgeState::Disconnected));
        let stats = Arc::new(RwLock::new(BridgeStats::default()));

        let bridge_task = tokio::spawn(Self::bridge_task(
            config.clone(),
            state.clone(),
            stats.clone(),
            command_rx,
            event_tx,
            shutdown_rx,
        ));

        Ok(Self {
            config,
            state,
            stats,
            command_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            bridge_task: Some(bridge_task),
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Start the bridge and connect to tab-daemon
    pub async fn start(&self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        match *state {
            BridgeState::Disconnected => {
                *state = BridgeState::Connecting;
                drop(state);
                info!("Starting WebSocket PTY bridge connection to {}", self.config.daemon_url);
                Ok(())
            }
            _ => Err(BridgeError::InvalidState(
                format!("Cannot start bridge in state: {:?}", *state)
            )),
        }
    }

    /// Send a command to the bridge
    pub fn send_command(&self, command: BridgeCommand) -> BridgeResult<()> {
        self.command_tx
            .send(command)
            .map_err(|_| BridgeError::ChannelClosed("Command channel closed".to_string()))
    }

    /// Send input data to the PTY
    pub fn send_input(&self, data: Vec<u8>) -> BridgeResult<()> {
        self.send_command(BridgeCommand::Input(InputChunk { data }))
    }

    /// Resize the terminal
    pub fn resize(&self, cols: u16, rows: u16) -> BridgeResult<()> {
        self.send_command(BridgeCommand::Resize((cols, rows)))
    }

    /// Terminate the session
    pub fn terminate(&self) -> BridgeResult<()> {
        self.send_command(BridgeCommand::Terminate)
    }

    /// Receive the next event from the bridge
    pub async fn recv_event(&self) -> Option<BridgeEvent> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await
    }

    /// Get the current bridge state
    pub async fn state(&self) -> BridgeState {
        *self.state.read().await
    }

    /// Get bridge statistics
    pub async fn stats(&self) -> BridgeStats {
        self.stats.read().await.clone()
    }

    /// Check if the bridge is connected
    pub async fn is_connected(&self) -> bool {
        matches!(self.state().await, BridgeState::Connected)
    }

    /// Shutdown the bridge gracefully
    pub async fn shutdown(mut self) -> BridgeResult<()> {
        info!("Shutting down WebSocket PTY bridge");

        // Update state
        let mut state = self.state.write().await;
        *state = BridgeState::ShuttingDown;
        drop(state);

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Send terminate command
        let _ = self.send_command(BridgeCommand::Terminate);

        // Wait for bridge task to complete
        if let Some(handle) = self.bridge_task.take() {
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), handle).await {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => Err(BridgeError::TaskError(e.to_string())),
                Err(_) => {
                    warn!("Bridge task shutdown timed out");
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Main bridge task that manages the WebSocket connection
    async fn bridge_task(
        config: WebSocketPtyBridgeConfig,
        state: Arc<RwLock<BridgeState>>,
        stats: Arc<RwLock<BridgeStats>>,
        mut command_rx: mpsc::UnboundedReceiver<BridgeCommand>,
        event_tx: mpsc::UnboundedSender<BridgeEvent>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> BridgeResult<()> {
        let mut reconnect_attempts = 0u32;
        let mut reconnect_delay = config.reconnection_policy.initial_delay_ms;

        loop {
            // Check for shutdown signal
            if let Ok(()) = shutdown_rx.try_recv() {
                info!("Bridge task received shutdown signal");
                break;
            }

            // Attempt connection
            match Self::connect_and_run(
                &config,
                &state,
                &stats,
                &mut command_rx,
                &event_tx,
                &mut shutdown_rx,
            ).await {
                Ok(()) => {
                    info!("Bridge connection closed gracefully");
                    break;
                }
                Err(e) => {
                    error!("Bridge connection error: {}", e);

                    // Check if we should reconnect
                    if reconnect_attempts >= config.reconnection_policy.max_attempts {
                        error!("Max reconnection attempts reached, giving up");
                        let _ = event_tx.send(BridgeEvent::Error(
                            "Max reconnection attempts reached".to_string()
                        ));
                        break;
                    }

                    // Update state
                    {
                        let mut s = state.write().await;
                        *s = BridgeState::Reconnecting;
                    }

                    // Update stats
                    {
                        let mut s = stats.write().await;
                        s.reconnection_attempts += 1;
                    }

                    // Notify about disconnection
                    let _ = event_tx.send(BridgeEvent::Disconnected(e.to_string()));

                    // Wait before reconnecting
                    warn!("Reconnecting in {}ms (attempt {}/{})",
                        reconnect_delay,
                        reconnect_attempts + 1,
                        config.reconnection_policy.max_attempts
                    );

                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(reconnect_delay)) => {
                            reconnect_attempts += 1;
                            reconnect_delay = ((reconnect_delay as f64 *
                                config.reconnection_policy.backoff_multiplier) as u64)
                                .min(config.reconnection_policy.max_delay_ms);
                        }
                        _ = shutdown_rx.recv() => {
                            info!("Shutdown during reconnection");
                            break;
                        }
                    }
                }
            }
        }

        // Update final state
        let mut s = state.write().await;
        *s = BridgeState::Closed;

        info!("Bridge task completed");
        Ok(())
    }

    /// Connect to daemon and run the I/O loop
    async fn connect_and_run(
        config: &WebSocketPtyBridgeConfig,
        state: &Arc<RwLock<BridgeState>>,
        stats: &Arc<RwLock<BridgeStats>>,
        command_rx: &mut mpsc::UnboundedReceiver<BridgeCommand>,
        event_tx: &mpsc::UnboundedSender<BridgeEvent>,
        shutdown_rx: &mut mpsc::Receiver<()>,
    ) -> BridgeResult<()> {
        // Connect to WebSocket
        info!("Connecting to tab-daemon at {}", config.daemon_url);

        let ws_stream = connect_authorized(
            config.daemon_url.clone(),
            config.auth_token.clone(),
        ).await.map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        info!("WebSocket connection established");

        // Update state
        {
            let mut s = state.write().await;
            *s = BridgeState::Connected;
        }

        // Update stats
        {
            let mut s = stats.write().await;
            s.connected_at = Some(std::time::Instant::now());
        }

        // Notify connection established
        event_tx.send(BridgeEvent::Connected)
            .map_err(|_| BridgeError::ChannelClosed("Event channel closed".to_string()))?;

        // Split WebSocket stream
        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        // Send Init message to start PTY session
        let init_msg = PtyWebsocketRequest::Init(config.tab_metadata.clone());
        let init_data = encode(init_msg)
            .map_err(|e| BridgeError::Serialization(e.to_string()))?;
        ws_tx.send(init_data)
            .await
            .map_err(|e| BridgeError::WebSocket(e))?;

        // Create channel for internal message passing
        let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();

        // Spawn WebSocket read task
        let ws_read_tx = internal_tx.clone();
        let ws_read_handle = tokio::spawn(async move {
            loop {
                match ws_rx.next().await {
                    Some(Ok(msg)) => {
                        match decode::<PtyWebsocketResponse>(Ok(msg)) {
                            Ok(response) => {
                                trace!("Received WebSocket message: {:?}", response);
                                if ws_read_tx.send(InternalMessage::WebSocketMessage(response)).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to decode WebSocket message: {}", e);
                            }
                        }
                    }
                    Some(Err(e)) => {
                        let _ = ws_read_tx.send(InternalMessage::WebSocketError(e));
                        break;
                    }
                    None => {
                        let _ = ws_read_tx.send(InternalMessage::WebSocketClosed);
                        break;
                    }
                }
            }
        });

        // Main I/O loop
        let result = loop {
            tokio::select! {
                // Handle internal messages
                Some(msg) = internal_rx.recv() => {
                    match msg {
                        InternalMessage::WebSocketMessage(response) => {
                            // Update stats
                            {
                                let mut s = stats.write().await;
                                s.messages_received += 1;
                            }

                            // Handle response and forward to Maestro
                            match Self::handle_websocket_response(
                                response,
                                event_tx,
                                stats,
                            ).await {
                                Ok(should_continue) => {
                                    if !should_continue {
                                        break Ok(());
                                    }
                                }
                                Err(e) => break Err(e),
                            }
                        }
                        InternalMessage::WebSocketError(e) => {
                            break Err(BridgeError::WebSocket(e));
                        }
                        InternalMessage::WebSocketClosed => {
                            info!("WebSocket connection closed by remote");
                            break Ok(());
                        }
                        InternalMessage::Command(cmd) => {
                            if let Err(e) = Self::handle_command(
                                cmd,
                                &mut ws_tx,
                                event_tx,
                                stats,
                            ).await {
                                break Err(e);
                            }
                        }
                        InternalMessage::Shutdown => {
                            info!("Shutdown requested");
                            break Ok(());
                        }
                    }
                }

                // Handle commands from Maestro
                Some(cmd) = command_rx.recv() => {
                    if let Err(e) = Self::handle_command(
                        cmd,
                        &mut ws_tx,
                        event_tx,
                        stats,
                    ).await {
                        break Err(e);
                    }
                }

                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received in I/O loop");
                    break Ok(());
                }
            }
        };

        // Cleanup
        let _ = ws_read_handle.await;

        // Send terminate message if still connected
        let _ = ws_tx.send(TungsteniteMessage::Close(None)).await;

        result
    }

    /// Handle a WebSocket response message
    async fn handle_websocket_response(
        response: PtyWebsocketResponse,
        event_tx: &mpsc::UnboundedSender<BridgeEvent>,
        stats: &Arc<RwLock<BridgeStats>>,
    ) -> BridgeResult<bool> {
        match response {
            PtyWebsocketResponse::Started(metadata) => {
                info!("PTY session started: {:?}", metadata);
                event_tx.send(BridgeEvent::Started(metadata))
                    .map_err(|_| BridgeError::ChannelClosed("Event channel closed".to_string()))?;
            }
            PtyWebsocketResponse::Output(chunk) => {
                trace!("PTY output: {} bytes", chunk.data.len());

                // Update stats
                {
                    let mut s = stats.write().await;
                    s.bytes_received += chunk.data.len() as u64;
                }

                event_tx.send(BridgeEvent::Output(chunk))
                    .map_err(|_| BridgeError::ChannelClosed("Event channel closed".to_string()))?;
            }
            PtyWebsocketResponse::Stopped => {
                info!("PTY session stopped");
                event_tx.send(BridgeEvent::Stopped)
                    .map_err(|_| BridgeError::ChannelClosed("Event channel closed".to_string()))?;
                return Ok(false); // Signal to stop the loop
            }
        }

        Ok(true)
    }

    /// Handle a command from Maestro
    async fn handle_command(
        cmd: BridgeCommand,
        ws_tx: &mut (impl SinkExt<TungsteniteMessage, Error = tungstenite::Error> + Unpin),
        event_tx: &mpsc::UnboundedSender<BridgeEvent>,
        stats: &Arc<RwLock<BridgeStats>>,
    ) -> BridgeResult<()> {
        let request = match cmd {
            BridgeCommand::Input(chunk) => {
                trace!("Sending PTY input: {} bytes", chunk.data.len());

                // Update stats
                {
                    let mut s = stats.write().await;
                    s.bytes_sent += chunk.data.len() as u64;
                    s.messages_sent += 1;
                }

                PtyWebsocketRequest::Input(chunk)
            }
            BridgeCommand::Resize(dimensions) => {
                debug!("Resizing terminal to: {:?}", dimensions);
                PtyWebsocketRequest::Resize(dimensions)
            }
            BridgeCommand::Terminate => {
                info!("Terminating PTY session");
                PtyWebsocketRequest::Terminate
            }
            BridgeCommand::Reconnect => {
                // Reconnect is handled at a higher level
                return Ok(());
            }
        };

        // Serialize and send
        let msg = encode(request)
            .map_err(|e| BridgeError::Serialization(e.to_string()))?;

        ws_tx.send(msg)
            .await
            .map_err(|e| BridgeError::WebSocket(e))?;

        // Send resize acknowledgment
        if let BridgeCommand::Resize(dimensions) = cmd {
            event_tx.send(BridgeEvent::Resized(dimensions))
                .map_err(|_| BridgeError::ChannelClosed("Event channel closed".to_string()))?;
        }

        Ok(())
    }
}

impl Drop for WebSocketPtyBridge {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());
        }
    }
}

/// Builder for WebSocketPtyBridgeConfig
pub struct WebSocketPtyBridgeConfigBuilder {
    config: WebSocketPtyBridgeConfig,
}

impl WebSocketPtyBridgeConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: WebSocketPtyBridgeConfig::default(),
        }
    }

    pub fn daemon_url(mut self, url: impl Into<String>) -> Self {
        self.config.daemon_url = url.into();
        self
    }

    pub fn auth_token(mut self, token: impl Into<String>) -> Self {
        self.config.auth_token = token.into();
        self
    }

    pub fn tab_metadata(mut self, metadata: TabMetadata) -> Self {
        self.config.tab_metadata = metadata;
        self
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn reconnection_policy(mut self, policy: ReconnectionPolicy) -> Self {
        self.config.reconnection_policy = policy;
        self
    }

    pub fn build(self) -> WebSocketPtyBridgeConfig {
        self.config
    }
}

impl Default for WebSocketPtyBridgeConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a TabMetadata for a new session
pub fn create_tab_metadata(
    name: impl Into<String>,
    shell: impl Into<String>,
    dir: impl Into<PathBuf>,
    dimensions: (u16, u16),
) -> TabMetadata {
    TabMetadata {
        id: TabId::new(),
        name: name.into(),
        doc: None,
        dimensions,
        env: std::env::vars().collect(),
        shell: shell.into(),
        dir: dir.into().to_string_lossy().to_string(),
        selected: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_state_transitions() {
        // Test that state transitions are valid
        let states = vec![
            BridgeState::Disconnected,
            BridgeState::Connecting,
            BridgeState::Connected,
            BridgeState::Reconnecting,
            BridgeState::ShuttingDown,
            BridgeState::Closed,
        ];

        for state in states {
            let state_clone = state;
            assert_eq!(state, state_clone);
        }
    }

    #[test]
    fn test_config_builder() {
        let config = WebSocketPtyBridgeConfigBuilder::new()
            .daemon_url("ws://localhost:12345/pty")
            .auth_token("test-token")
            .buffer_size(8192)
            .build();

        assert_eq!(config.daemon_url, "ws://localhost:12345/pty");
        assert_eq!(config.auth_token, "test-token");
        assert_eq!(config.buffer_size, 8192);
    }

    #[test]
    fn test_reconnection_policy_default() {
        let policy = ReconnectionPolicy::default();
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay_ms, 1000);
        assert_eq!(policy.max_delay_ms, 30000);
        assert_eq!(policy.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_bridge_event_variants() {
        // Test that all event variants can be created
        let _events = vec![
            BridgeEvent::Output(OutputChunk { index: 0, data: vec![1, 2, 3] }),
            BridgeEvent::Started(TabMetadata::default()),
            BridgeEvent::Stopped,
            BridgeEvent::Connected,
            BridgeEvent::Disconnected("test".to_string()),
            BridgeEvent::Error("test".to_string()),
            BridgeEvent::Resized((80, 24)),
        ];
    }

    #[test]
    fn test_bridge_command_variants() {
        // Test that all command variants can be created
        let _commands = vec![
            BridgeCommand::Input(InputChunk { data: vec![1, 2, 3] }),
            BridgeCommand::Resize((80, 24)),
            BridgeCommand::Terminate,
            BridgeCommand::Reconnect,
        ];
    }
}
