//! Channel traits for multi-platform messaging
//!
//! Provides a unified interface for messaging channels like Telegram, Discord, Slack.
//! Based on Moltis ChannelPlugin and IronClaw Channel trait patterns.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use uuid::Uuid;

pub mod telegram;

/// Unique identifier for a message
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a user
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

/// Unique identifier for a conversation/thread
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreadId(pub String);

/// Incoming message from a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Unique message ID
    pub id: MessageId,
    /// Channel that sent this message
    pub channel: String,
    /// User who sent the message
    pub user_id: UserId,
    /// Message content
    pub content: String,
    /// Thread/conversation ID if part of a thread
    pub thread_id: Option<ThreadId>,
    /// Account ID that received the message
    pub account_id: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl IncomingMessage {
    /// Create a new incoming message
    pub fn new(
        channel: impl Into<String>,
        user_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: MessageId::new(),
            channel: channel.into(),
            user_id: UserId(user_id.into()),
            content: content.into(),
            thread_id: None,
            account_id: String::new(),
            metadata: serde_json::Value::Null,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Outgoing response to a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingResponse {
    /// Message ID being responded to
    pub reply_to: MessageId,
    /// Response content
    pub content: ResponseContent,
    /// Thread ID to send to
    pub thread_id: Option<ThreadId>,
}

/// Content for outgoing responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseContent {
    /// Plain text response
    Text { text: String },
    /// Markdown response
    Markdown { text: String },
    /// Media attachment
    Media {
        url: String,
        caption: Option<String>,
    },
    /// Code block
    Code {
        code: String,
        language: Option<String>,
    },
}

/// Stream of incoming messages
pub type MessageStream = Pin<Box<dyn futures_util::Stream<Item = IncomingMessage> + Send>>;

/// Core channel trait for receiving messages
#[async_trait]
pub trait Channel: Send + Sync {
    /// Channel identifier
    fn id(&self) -> &str;

    /// Channel display name
    fn name(&self) -> &str;

    /// Start receiving messages
    async fn receive(&self) -> anyhow::Result<MessageStream>;

    /// Send a response
    async fn send(&self, response: OutgoingResponse) -> anyhow::Result<()>;
}

/// Channel plugin trait for account management
#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    /// Plugin identifier
    fn id(&self) -> &str;

    /// Plugin display name
    fn name(&self) -> &str;

    /// Start an account with the given configuration
    async fn start_account(
        &mut self,
        account_id: &str,
        config: serde_json::Value,
    ) -> anyhow::Result<()>;

    /// Stop an account
    async fn stop_account(&mut self, account_id: &str) -> anyhow::Result<()>;

    /// Get outbound interface for this plugin
    fn outbound(&self) -> Option<&dyn ChannelOutbound>;
}

/// Outbound messaging interface
#[async_trait]
pub trait ChannelOutbound: Send + Sync {
    /// Send a text message
    async fn send_text(
        &self,
        account_id: &str,
        to: &str,
        text: &str,
        reply_to: Option<&str>,
    ) -> anyhow::Result<String>;

    /// Send a media message
    async fn send_media(
        &self,
        account_id: &str,
        to: &str,
        url: &str,
        caption: Option<&str>,
        reply_to: Option<&str>,
    ) -> anyhow::Result<String>;

    /// Send typing indicator
    async fn send_typing(&self, account_id: &str, to: &str) -> anyhow::Result<()>;
}

/// Channel registry for managing multiple channels
pub struct ChannelRegistry {
    channels: std::collections::HashMap<String, Box<dyn ChannelPlugin>>,
}

impl ChannelRegistry {
    /// Create a new channel registry
    pub fn new() -> Self {
        Self {
            channels: std::collections::HashMap::new(),
        }
    }

    /// Register a channel plugin
    pub fn register(&mut self, channel: Box<dyn ChannelPlugin>) {
        let id = channel.id().to_string();
        self.channels.insert(id, channel);
    }

    /// Get a channel by ID
    pub fn get(&self, id: &str) -> Option<&dyn ChannelPlugin> {
        self.channels.get(id).map(|c| c.as_ref())
    }

    /// Get a mutable channel by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn ChannelPlugin>> {
        self.channels.get_mut(id)
    }

    /// List registered channel IDs
    pub fn list(&self) -> Vec<&str> {
        self.channels.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id_creation() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_incoming_message_creation() {
        let msg = IncomingMessage::new("telegram", "user123", "Hello!");
        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.user_id.0, "user123");
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn test_channel_registry() {
        let registry = ChannelRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_response_content_serialization() {
        let content = ResponseContent::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"text"#));
    }
}
