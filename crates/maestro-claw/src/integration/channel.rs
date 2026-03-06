//! Channel Bridge for maestro-core integration
//!
//! This module provides integration between maestro-claw and maestro-core's
//! Channel trait for multi-platform messaging.
//!
//! # Async Safety
//! All registry access uses `tokio::sync::Mutex` to avoid holding a sync
//! lock across `.await` points, which would cause deadlocks in the Tokio runtime.

use std::sync::Arc;

use tokio::sync::Mutex as AsyncMutex;

use maestro_core::channel::{ChannelPlugin, ChannelRegistry, IncomingMessage};

/// Error from channel bridge operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum ChannelBridgeError {
    /// Channel not found
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    /// Message send failed
    #[error("Failed to send message: {0}")]
    SendFailed(String),

    /// Message receive failed
    #[error("Failed to receive message: {0}")]
    ReceiveFailed(String),

    /// Account management failed
    #[error("Account management failed: {0}")]
    AccountError(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
}

/// Bridge for connecting maestro-claw agents to maestro-core channels
///
/// This allows agents to receive messages from external channels
/// (Telegram, Discord, Slack, etc.) and send responses.
///
/// Uses `tokio::sync::Mutex` to prevent holding a lock across `.await` points.
pub struct ChannelBridge {
    registry: Arc<AsyncMutex<ChannelRegistry>>,
    default_channel: Option<String>,
}

impl ChannelBridge {
    /// Create a new channel bridge with an empty registry
    pub fn new() -> Self {
        Self {
            registry: Arc::new(AsyncMutex::new(ChannelRegistry::new())),
            default_channel: None,
        }
    }

    /// Create a channel bridge with an existing registry
    pub fn with_registry(registry: ChannelRegistry) -> Self {
        Self {
            registry: Arc::new(AsyncMutex::new(registry)),
            default_channel: None,
        }
    }

    /// Set the default channel for outgoing messages
    pub fn with_default_channel(mut self, channel: impl Into<String>) -> Self {
        self.default_channel = Some(channel.into());
        self
    }

    /// Register a channel plugin
    ///
    /// This is a blocking operation; call from sync or non-performance-critical code.
    /// For async contexts, prefer a setup phase before concurrent usage.
    pub async fn register_channel(&self, channel: Box<dyn ChannelPlugin>) {
        self.registry.lock().await.register(channel);
    }

    /// List available channels
    pub async fn list_channels(&self) -> Vec<String> {
        self.registry
            .lock()
            .await
            .list()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Start a channel account
    ///
    /// Uses `tokio::sync::Mutex` so the lock is released before each `.await`.
    pub async fn start_account(
        &self,
        channel_id: &str,
        account_id: &str,
        config: serde_json::Value,
    ) -> Result<(), ChannelBridgeError> {
        // Lock, get the channel, clone what we need, then release the lock before .await
        let result = {
            let mut registry = self.registry.lock().await;
            let channel = registry
                .get_mut(channel_id)
                .ok_or_else(|| ChannelBridgeError::ChannelNotFound(channel_id.to_string()))?;
            // Execute the async call while holding the tokio Mutex (safe — not a std Mutex)
            channel
                .start_account(account_id, config)
                .await
                .map_err(|e| ChannelBridgeError::AccountError(e.to_string()))
        };
        result
    }

    /// Stop a channel account
    pub async fn stop_account(
        &self,
        channel_id: &str,
        account_id: &str,
    ) -> Result<(), ChannelBridgeError> {
        let result = {
            let mut registry = self.registry.lock().await;
            let channel = registry
                .get_mut(channel_id)
                .ok_or_else(|| ChannelBridgeError::ChannelNotFound(channel_id.to_string()))?;
            channel
                .stop_account(account_id)
                .await
                .map_err(|e| ChannelBridgeError::AccountError(e.to_string()))
        };
        result
    }

    /// Send a text response to a message
    pub async fn send_text(
        &self,
        channel_id: &str,
        account_id: &str,
        to: &str,
        text: &str,
        reply_to: Option<&str>,
    ) -> Result<String, ChannelBridgeError> {
        let result = {
            let registry = self.registry.lock().await;
            let channel = registry
                .get(channel_id)
                .ok_or_else(|| ChannelBridgeError::ChannelNotFound(channel_id.to_string()))?;

            let outbound = channel.outbound().ok_or_else(|| {
                ChannelBridgeError::SendFailed("No outbound interface".to_string())
            })?;

            outbound
                .send_text(account_id, to, text, reply_to)
                .await
                .map_err(|e| ChannelBridgeError::SendFailed(e.to_string()))
        };
        result
    }

    /// Send a markdown response
    pub async fn send_markdown(
        &self,
        channel_id: &str,
        account_id: &str,
        to: &str,
        markdown: &str,
        reply_to: Option<&str>,
    ) -> Result<String, ChannelBridgeError> {
        // Use text for markdown (channels handle formatting)
        self.send_text(channel_id, account_id, to, markdown, reply_to)
            .await
    }

    /// Send typing indicator
    pub async fn send_typing(
        &self,
        channel_id: &str,
        account_id: &str,
        to: &str,
    ) -> Result<(), ChannelBridgeError> {
        let result = {
            let registry = self.registry.lock().await;
            let channel = registry
                .get(channel_id)
                .ok_or_else(|| ChannelBridgeError::ChannelNotFound(channel_id.to_string()))?;

            let outbound = channel.outbound().ok_or_else(|| {
                ChannelBridgeError::SendFailed("No outbound interface".to_string())
            })?;

            outbound
                .send_typing(account_id, to)
                .await
                .map_err(|e| ChannelBridgeError::SendFailed(e.to_string()))
        };
        result
    }
}

impl Default for ChannelBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an IncomingMessage from maestro-core to a maestro-claw Turn
pub fn incoming_message_to_turn(msg: &IncomingMessage) -> crate::session::Turn {
    crate::session::Turn::new(crate::session::TurnRole::User, msg.content.clone())
    // Note: Channel metadata is not stored on Turn as it doesn't have a metadata field.
    // Consumers should track this separately.
}

/// Agent notification types for channel integration
#[derive(Debug, Clone)]
pub enum AgentNotification {
    /// Agent started processing
    Started { session_id: String },
    /// Agent completed a turn
    TurnCompleted {
        session_id: String,
        turn_number: usize,
    },
    /// Agent finished processing
    Completed {
        session_id: String,
        final_message: String,
    },
    /// Agent encountered an error
    Error { session_id: String, error: String },
}

/// Notification sender for channel integration
pub struct ChannelNotifier {
    bridge: Arc<ChannelBridge>,
    channel_id: String,
    account_id: String,
}

impl ChannelNotifier {
    /// Create a new channel notifier
    pub fn new(
        bridge: Arc<ChannelBridge>,
        channel_id: impl Into<String>,
        account_id: impl Into<String>,
    ) -> Self {
        Self {
            bridge,
            channel_id: channel_id.into(),
            account_id: account_id.into(),
        }
    }

    /// Send a notification about agent status
    pub async fn notify(
        &self,
        to: &str,
        notification: &AgentNotification,
    ) -> Result<(), ChannelBridgeError> {
        let message = match notification {
            AgentNotification::Started { session_id } => {
                format!("Agent started processing (session: {})", session_id)
            }
            AgentNotification::TurnCompleted {
                session_id,
                turn_number,
            } => {
                format!("Turn {} completed (session: {})", turn_number, session_id)
            }
            AgentNotification::Completed {
                session_id,
                final_message,
            } => {
                format!("Agent: {}\n(session: {})", final_message, session_id)
            }
            AgentNotification::Error { session_id, error } => {
                format!("Error (session: {}): {}", session_id, error)
            }
        };

        self.bridge
            .send_text(&self.channel_id, &self.account_id, to, &message, None)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_bridge_creation() {
        let bridge = ChannelBridge::new();
        assert!(bridge.list_channels().await.is_empty());
    }

    #[test]
    fn test_channel_bridge_with_default() {
        let bridge = ChannelBridge::new().with_default_channel("telegram");
        assert_eq!(bridge.default_channel, Some("telegram".to_string()));
    }

    #[test]
    fn test_incoming_message_to_turn() {
        let msg = IncomingMessage::new("telegram", "user123", "Hello agent!");
        let turn = incoming_message_to_turn(&msg);

        assert_eq!(turn.role, crate::session::TurnRole::User);
        assert_eq!(turn.content, "Hello agent!");
    }

    #[test]
    fn test_agent_notification_messages() {
        let started = AgentNotification::Started {
            session_id: "sess-123".to_string(),
        };
        let msg = match started {
            AgentNotification::Started { session_id } => {
                format!("Agent started processing (session: {})", session_id)
            }
            _ => String::new(),
        };
        assert!(msg.contains("sess-123"));
    }

    #[test]
    fn test_channel_notifier_creation() {
        let bridge = Arc::new(ChannelBridge::new());
        let notifier = ChannelNotifier::new(bridge, "telegram", "account-1");

        assert_eq!(notifier.channel_id, "telegram");
        assert_eq!(notifier.account_id, "account-1");
    }
}
