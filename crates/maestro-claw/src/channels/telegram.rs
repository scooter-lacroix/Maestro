//! Telegram bot channel using long polling.

use std::time::Duration;

use async_trait::async_trait;

use crate::channels::{Channel, ChannelMessage, SendMessage};

pub struct TelegramChannel {
    bot_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
}

impl TelegramChannel {
    pub fn new(bot_token: String, allowed_users: Vec<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("telegram http client");

        Self {
            bot_token,
            allowed_users,
            client,
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{method}", self.bot_token)
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users
            .iter()
            .any(|allowed| allowed == "*" || allowed == user_id)
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.client
            .post(self.api_url("sendMessage"))
            .json(&serde_json::json!({
                "chat_id": message.recipient,
                "text": message.content,
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let mut offset: i64 = 0;

        loop {
            let url = format!("{}?offset={offset}&timeout=30", self.api_url("getUpdates"));
            let response: serde_json::Value = match self.client.get(&url).send().await {
                Ok(resp) => resp.json().await.unwrap_or_default(),
                Err(error) => {
                    tracing::warn!("telegram poll error: {error}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Some(updates) = response["result"].as_array() {
                for update in updates {
                    if let Some(update_id) = update["update_id"].as_i64() {
                        offset = update_id + 1;
                    }

                    let message = &update["message"];
                    let text = message["text"].as_str().unwrap_or("");
                    let chat_id = message["chat"]["id"].as_i64().unwrap_or(0);
                    let user_id = message["from"]["id"].as_i64().unwrap_or(0);

                    if text.is_empty() || !self.is_allowed(&user_id.to_string()) {
                        continue;
                    }

                    let _ = tx
                        .send(ChannelMessage {
                            id: update["update_id"].to_string(),
                            sender: user_id.to_string(),
                            reply_target: chat_id.to_string(),
                            content: text.to_string(),
                            channel: "telegram".into(),
                            timestamp: message["date"].as_u64().unwrap_or(0),
                        })
                        .await;
                }
            }
        }
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(self.api_url("getMe"))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_allowlist_accepts_wildcard() {
        let channel = TelegramChannel::new("token".into(), vec!["*".into()]);
        assert!(channel.is_allowed("42"));
    }

    #[test]
    fn telegram_allowlist_rejects_unknown_user() {
        let channel = TelegramChannel::new("token".into(), vec!["7".into()]);
        assert!(!channel.is_allowed("42"));
    }

    #[test]
    fn telegram_allowlist_denies_when_empty() {
        let channel = TelegramChannel::new("token".into(), Vec::new());
        assert!(!channel.is_allowed("42"));
    }
}
