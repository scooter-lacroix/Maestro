//! Slack bot channel using Socket Mode.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::channels::{Channel, ChannelMessage, SendMessage};

pub struct SlackChannel {
    bot_token: String,
    app_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
}

impl SlackChannel {
    pub fn new(bot_token: String, app_token: String, allowed_users: Vec<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("slack http client");

        Self {
            bot_token,
            app_token,
            allowed_users,
            client,
        }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users
            .iter()
            .any(|allowed| allowed == "*" || allowed == user_id)
    }

    async fn socket_mode_url(&self) -> Result<String> {
        let response: serde_json::Value = self
            .client
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(&self.app_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if !response["ok"].as_bool().unwrap_or(false) {
            return Err(anyhow!(
                "slack apps.connections.open failed: {}",
                response["error"].as_str().unwrap_or("unknown")
            ));
        }

        response["url"]
            .as_str()
            .filter(|url| !url.is_empty())
            .map(str::to_string)
            .context("slack apps.connections.open did not return a websocket url")
    }

    fn parse_timestamp(timestamp: &str) -> u64 {
        timestamp
            .split('.')
            .next()
            .and_then(|seconds| seconds.parse::<u64>().ok())
            .unwrap_or_default()
    }

    fn normalize_content(event_type: &str, text: &str) -> Option<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }

        if event_type == "app_mention" {
            let normalized = trimmed
                .split_whitespace()
                .skip_while(|part| part.starts_with("<@") && part.ends_with('>'))
                .collect::<Vec<_>>()
                .join(" ");
            let normalized = normalized.trim();
            if normalized.is_empty() {
                return None;
            }
            return Some(normalized.to_string());
        }

        Some(trimmed.to_string())
    }

    fn parse_socket_message(payload: &serde_json::Value) -> Option<ChannelMessage> {
        let event = payload.get("payload")?.get("event")?;
        let event_type = event.get("type")?.as_str()?;
        if event_type != "message" && event_type != "app_mention" {
            return None;
        }
        if event_type == "message"
            && event
                .get("subtype")
                .and_then(serde_json::Value::as_str)
                .is_some()
        {
            return None;
        }
        if event.get("bot_id").is_some() || event.get("bot_profile").is_some() {
            return None;
        }

        let text = Self::normalize_content(event_type, event.get("text")?.as_str()?)?;

        let sender = event.get("user")?.as_str()?.to_string();
        let reply_target = event.get("channel")?.as_str()?.to_string();
        let id = event
            .get("client_msg_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| event.get("ts").and_then(serde_json::Value::as_str))
            .unwrap_or_default()
            .to_string();
        let timestamp = event
            .get("event_ts")
            .and_then(serde_json::Value::as_str)
            .or_else(|| event.get("ts").and_then(serde_json::Value::as_str))
            .map(Self::parse_timestamp)
            .unwrap_or_default();

        Some(ChannelMessage {
            id,
            sender,
            reply_target,
            content: text,
            channel: "slack".into(),
            timestamp,
        })
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, message: &SendMessage) -> Result<()> {
        let response: serde_json::Value = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&serde_json::json!({
                "channel": message.recipient,
                "text": message.content,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if response["ok"].as_bool().unwrap_or(false) {
            Ok(())
        } else {
            Err(anyhow!(
                "slack chat.postMessage failed: {}",
                response["error"].as_str().unwrap_or("unknown")
            ))
        }
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
        let socket_url = self.socket_mode_url().await?;
        let (socket, _) = connect_async(socket_url).await?;
        let (mut write, mut read) = socket.split();

        while let Some(frame) = read.next().await {
            let frame = frame?;

            if let Message::Text(text) = frame {
                let payload: serde_json::Value = serde_json::from_str(&text)?;

                if let Some(envelope_id) = payload
                    .get("envelope_id")
                    .and_then(serde_json::Value::as_str)
                {
                    let ack = serde_json::json!({ "envelope_id": envelope_id });
                    write.send(Message::Text(ack.to_string().into())).await?;
                }

                if let Some(message) = Self::parse_socket_message(&payload) {
                    if !self.is_allowed(&message.sender) {
                        continue;
                    }

                    if tx.send(message).await.is_err() {
                        return Err(anyhow!("slack dispatcher receiver closed"));
                    }
                }
            }
        }

        Ok(())
    }

    async fn health_check(&self) -> bool {
        match self
            .client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .send()
            .await
        {
            Ok(response) => match response.error_for_status() {
                Ok(response) => match response.json::<serde_json::Value>().await {
                    Ok(json) => json["ok"].as_bool().unwrap_or(false),
                    Err(_) => false,
                },
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_allowlist_accepts_wildcard() {
        let channel = SlackChannel::new("bot".into(), "app".into(), vec!["*".into()]);
        assert!(channel.is_allowed("U123"));
    }

    #[test]
    fn slack_allowlist_rejects_unknown_user() {
        let channel = SlackChannel::new("bot".into(), "app".into(), vec!["U111".into()]);
        assert!(!channel.is_allowed("U123"));
    }

    #[test]
    fn slack_allowlist_denies_when_empty() {
        let channel = SlackChannel::new("bot".into(), "app".into(), Vec::new());
        assert!(!channel.is_allowed("U123"));
    }

    #[test]
    fn slack_parse_timestamp_uses_seconds() {
        assert_eq!(
            SlackChannel::parse_timestamp("1700000000.987654"),
            1700000000
        );
    }

    #[test]
    fn slack_parses_socket_mode_message() {
        let payload = serde_json::json!({
            "envelope_id": "env-1",
            "payload": {
                "event": {
                    "type": "message",
                    "user": "U123",
                    "channel": "C456",
                    "text": "hello",
                    "ts": "1700000000.123456",
                    "client_msg_id": "msg-1"
                }
            }
        });

        let message = SlackChannel::parse_socket_message(&payload).unwrap();
        assert_eq!(message.id, "msg-1");
        assert_eq!(message.sender, "U123");
        assert_eq!(message.reply_target, "C456");
        assert_eq!(message.content, "hello");
        assert_eq!(message.timestamp, 1700000000);
    }

    #[test]
    fn slack_parses_app_mention_message() {
        let payload = serde_json::json!({
            "envelope_id": "env-2",
            "payload": {
                "event": {
                    "type": "app_mention",
                    "user": "U123",
                    "channel": "C456",
                    "text": "<@U999> summarize the logs",
                    "ts": "1700000000.123456",
                    "client_msg_id": "msg-2"
                }
            }
        });

        let message = SlackChannel::parse_socket_message(&payload).unwrap();
        assert_eq!(message.content, "summarize the logs");
    }

    #[test]
    fn slack_ignores_empty_app_mention_after_stripping() {
        let payload = serde_json::json!({
            "payload": {
                "event": {
                    "type": "app_mention",
                    "user": "U123",
                    "channel": "C456",
                    "text": "<@U999>",
                    "ts": "1700000000.123456"
                }
            }
        });

        assert!(SlackChannel::parse_socket_message(&payload).is_none());
    }

    #[test]
    fn slack_ignores_bot_messages() {
        let payload = serde_json::json!({
            "payload": {
                "event": {
                    "type": "message",
                    "bot_id": "B123",
                    "channel": "C456",
                    "text": "hello",
                    "ts": "1700000000.123456"
                }
            }
        });

        assert!(SlackChannel::parse_socket_message(&payload).is_none());
    }

    #[test]
    fn slack_ignores_message_subtypes() {
        let payload = serde_json::json!({
            "payload": {
                "event": {
                    "type": "message",
                    "subtype": "message_changed",
                    "user": "U123",
                    "channel": "C456",
                    "text": "hello",
                    "ts": "1700000000.123456"
                }
            }
        });

        assert!(SlackChannel::parse_socket_message(&payload).is_none());
    }
}
