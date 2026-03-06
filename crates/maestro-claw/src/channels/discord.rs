//! Discord bot channel using the gateway websocket.

use async_trait::async_trait;

use crate::channels::{Channel, ChannelMessage, SendMessage};

pub struct DiscordChannel {
    bot_token: String,
    guild_id: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
}

impl DiscordChannel {
    pub fn new(bot_token: String, guild_id: String, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token,
            guild_id,
            allowed_users,
            client: reqwest::Client::new(),
        }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users
            .iter()
            .any(|allowed| allowed == "*" || allowed == user_id)
    }

    fn is_expected_guild(&self, guild_id: &str) -> bool {
        !guild_id.is_empty() && guild_id == self.guild_id
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            message.recipient
        );
        self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&serde_json::json!({ "content": message.content }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        use futures_util::{SinkExt, StreamExt};
        use std::future::pending;
        use tokio::time::{self, Duration, MissedTickBehavior};
        use tokio_tungstenite::connect_async;
        use tokio_tungstenite::tungstenite::Message;

        let (socket, _) = connect_async("wss://gateway.discord.gg/?v=10&encoding=json").await?;
        let (mut write, mut read) = socket.split();
        let mut heartbeat: Option<tokio::time::Interval> = None;
        let mut last_sequence = serde_json::Value::Null;

        loop {
            tokio::select! {
                maybe_frame = read.next() => {
                    let Some(frame) = maybe_frame else {
                        break;
                    };
                    let frame = frame?;

                    if let Message::Text(text) = frame {
                        let payload: serde_json::Value = serde_json::from_str(&text)?;
                        if let Some(sequence) = payload.get("s").cloned() {
                            if !sequence.is_null() {
                                last_sequence = sequence;
                            }
                        }

                        let opcode = payload["op"].as_u64();

                        if opcode == Some(10) {
                            let interval_ms = payload["d"]["heartbeat_interval"]
                                .as_u64()
                                .unwrap_or(45_000);
                            let identify = serde_json::json!({
                                "op": 2,
                                "d": {
                                    "token": self.bot_token,
                                    "intents": 512 | 32768,
                                    "properties": {
                                        "os": std::env::consts::OS,
                                        "browser": "maestroclaw",
                                        "device": "maestroclaw"
                                    }
                                }
                            });
                            write
                                .send(Message::Text(identify.to_string().into()))
                                .await?;

                            let mut interval = time::interval(Duration::from_millis(interval_ms));
                            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                            heartbeat = Some(interval);
                            continue;
                        }

                        if opcode == Some(0) && payload["t"].as_str() == Some("MESSAGE_CREATE") {
                            let data = &payload["d"];
                            let author_id = data["author"]["id"].as_str().unwrap_or("");
                            let is_bot = data["author"]["bot"].as_bool().unwrap_or(false);
                            let content = data["content"].as_str().unwrap_or("");
                            let channel_id = data["channel_id"].as_str().unwrap_or("");
                            let guild_id = data["guild_id"].as_str().unwrap_or("");

                            if is_bot
                                || content.is_empty()
                                || !self.is_expected_guild(guild_id)
                                || !self.is_allowed(author_id)
                            {
                                continue;
                            }

                            let _ = tx
                                .send(ChannelMessage {
                                    id: data["id"].as_str().unwrap_or("").into(),
                                    sender: author_id.into(),
                                    reply_target: channel_id.into(),
                                    content: content.into(),
                                    channel: "discord".into(),
                                    timestamp: 0,
                                })
                                .await;
                        }
                    }
                }
                _ = async {
                    if let Some(interval) = heartbeat.as_mut() {
                        interval.tick().await;
                    } else {
                        pending::<()>().await;
                    }
                } => {
                    let beat = serde_json::json!({ "op": 1, "d": last_sequence });
                    write.send(Message::Text(beat.to_string().into())).await?;
                }
            }
        }

        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", self.bot_token))
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
    fn discord_allowlist_accepts_known_user() {
        let channel = DiscordChannel::new("token".into(), "guild".into(), vec!["42".into()]);
        assert!(channel.is_allowed("42"));
    }

    #[test]
    fn discord_allowlist_rejects_unknown_user() {
        let channel = DiscordChannel::new("token".into(), "guild".into(), vec!["42".into()]);
        assert!(!channel.is_allowed("99"));
    }

    #[test]
    fn discord_allowlist_denies_when_empty() {
        let channel = DiscordChannel::new("token".into(), "guild".into(), Vec::new());
        assert!(!channel.is_allowed("42"));
    }

    #[test]
    fn discord_guild_scope_must_match() {
        let channel = DiscordChannel::new("token".into(), "guild-1".into(), vec!["42".into()]);
        assert!(channel.is_expected_guild("guild-1"));
        assert!(!channel.is_expected_guild("guild-2"));
        assert!(!channel.is_expected_guild(""));
    }
}
