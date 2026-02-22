//! Telegram channel implementation
//!
//! Provides a Telegram bot integration using the teloxide-like interface.
//! This is a simplified implementation for demonstration; a full implementation
//! would use the actual teloxide library.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{ChannelOutbound, ChannelPlugin, IncomingMessage};

/// Telegram bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token from BotFather
    pub token: String,
    /// Whether to parse markdown in messages
    pub parse_mode: Option<String>,
    /// Webhook URL (optional, for webhook mode)
    pub webhook_url: Option<String>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            parse_mode: Some("Markdown".to_string()),
            webhook_url: None,
        }
    }
}

/// Telegram channel plugin
pub struct TelegramChannel {
    accounts: RwLock<std::collections::HashMap<String, TelegramAccount>>,
}

/// A single Telegram bot account
struct TelegramAccount {
    #[allow(dead_code)]
    config: TelegramConfig,
    running: bool,
}

impl TelegramChannel {
    /// Create a new Telegram channel plugin
    pub fn new() -> Self {
        Self {
            accounts: RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for TelegramChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelPlugin for TelegramChannel {
    fn id(&self) -> &str {
        "telegram"
    }

    fn name(&self) -> &str {
        "Telegram"
    }

    async fn start_account(
        &mut self,
        account_id: &str,
        config: serde_json::Value,
    ) -> anyhow::Result<()> {
        let config: TelegramConfig = serde_json::from_value(config)
            .map_err(|e| anyhow::anyhow!("Invalid Telegram config: {}", e))?;

        info!("Starting Telegram account: {}", account_id);

        let mut accounts = self.accounts.write().await;
        accounts.insert(
            account_id.to_string(),
            TelegramAccount {
                config,
                running: true,
            },
        );

        Ok(())
    }

    async fn stop_account(&mut self, account_id: &str) -> anyhow::Result<()> {
        info!("Stopping Telegram account: {}", account_id);

        let mut accounts = self.accounts.write().await;
        if let Some(account) = accounts.get_mut(account_id) {
            account.running = false;
        }

        Ok(())
    }

    fn outbound(&self) -> Option<&dyn ChannelOutbound> {
        Some(self)
    }
}

#[async_trait]
impl ChannelOutbound for TelegramChannel {
    async fn send_text(
        &self,
        account_id: &str,
        to: &str,
        text: &str,
        _reply_to: Option<&str>,
    ) -> anyhow::Result<String> {
        let accounts = self.accounts.read().await;

        let account = accounts
            .get(account_id)
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id))?;

        if !account.running {
            return Err(anyhow::anyhow!("Account not running: {}", account_id));
        }

        // In a real implementation, this would use teloxide to send the message
        // For now, we simulate success
        debug!(
            "Sending Telegram message to {} via account {}: {} chars",
            to,
            account_id,
            text.len(),
        );

        // Return a simulated message ID
        Ok(format!("tg_msg_{}", uuid::Uuid::new_v4()))
    }

    async fn send_media(
        &self,
        account_id: &str,
        to: &str,
        url: &str,
        caption: Option<&str>,
        _reply_to: Option<&str>,
    ) -> anyhow::Result<String> {
        let accounts = self.accounts.read().await;

        let account = accounts
            .get(account_id)
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id))?;

        if !account.running {
            return Err(anyhow::anyhow!("Account not running: {}", account_id));
        }

        debug!(
            "Sending Telegram media to {} via account {}: {} (caption: {:?})",
            to, account_id, url, caption
        );

        Ok(format!("tg_msg_{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, account_id: &str, to: &str) -> anyhow::Result<()> {
        let accounts = self.accounts.read().await;

        let account = accounts
            .get(account_id)
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id))?;

        if !account.running {
            return Err(anyhow::anyhow!("Account not running: {}", account_id));
        }

        debug!(
            "Sending Telegram typing indicator to {} via account {}",
            to, account_id
        );
        Ok(())
    }
}

/// Telegram message receiver (placeholder)
pub struct TelegramReceiver {
    #[allow(dead_code)]
    account_id: String,
}

impl TelegramReceiver {
    /// Create a new receiver for the given account
    pub fn new(account_id: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
        }
    }

    /// Receive messages (placeholder - returns empty stream)
    pub fn receive(&self) -> impl futures_util::Stream<Item = IncomingMessage> + Send {
        // In a real implementation, this would use teloxide's update stream
        futures_util::stream::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telegram_channel_creation() {
        let channel = TelegramChannel::new();
        assert_eq!(channel.id(), "telegram");
        assert_eq!(channel.name(), "Telegram");
    }

    #[tokio::test]
    async fn test_telegram_account_management() {
        let mut channel = TelegramChannel::new();

        let config = TelegramConfig {
            token: "test_token".to_string(),
            ..Default::default()
        };

        let result = channel
            .start_account("test_account", serde_json::to_value(config).unwrap())
            .await;
        assert!(result.is_ok());

        let result = channel.stop_account("test_account").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_telegram_send_text() {
        let mut channel = TelegramChannel::new();

        let config = TelegramConfig {
            token: "test_token".to_string(),
            ..Default::default()
        };

        channel
            .start_account("test_account", serde_json::to_value(config).unwrap())
            .await
            .unwrap();

        let result = channel
            .send_text("test_account", "12345", "Hello!", None)
            .await;
        assert!(result.is_ok());

        // Test non-existent account
        let result = channel
            .send_text("nonexistent", "12345", "Hello!", None)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_telegram_config_deserialization() {
        let json = r#"{"token": "abc123", "parse_mode": "Markdown"}"#;
        let config: TelegramConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.token, "abc123");
        assert_eq!(config.parse_mode, Some("Markdown".to_string()));
    }
}
