//! Channel trait and shared message types.

use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub reply_target: String,
    pub content: String,
    pub channel: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMessage {
    pub content: String,
    pub recipient: String,
}

impl SendMessage {
    pub fn new(content: impl Into<String>, recipient: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            recipient: recipient.into(),
        }
    }
}

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;

    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyChannel;

    #[async_trait]
    impl Channel for DummyChannel {
        fn name(&self) -> &str {
            "dummy"
        }

        async fn send(&self, _: &SendMessage) -> anyhow::Result<()> {
            Ok(())
        }

        async fn listen(
            &self,
            tx: tokio::sync::mpsc::Sender<ChannelMessage>,
        ) -> anyhow::Result<()> {
            tx.send(ChannelMessage {
                id: "1".into(),
                sender: "test".into(),
                reply_target: "test".into(),
                content: "hello".into(),
                channel: "dummy".into(),
                timestamp: 0,
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
        }
    }

    #[tokio::test]
    async fn dummy_channel_sends_and_receives() {
        let channel = DummyChannel;
        channel.send(&SendMessage::new("hi", "bob")).await.unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        channel.listen(tx).await.unwrap();

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.reply_target, "test");
    }
}
