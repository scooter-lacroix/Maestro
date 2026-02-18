use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A conversation context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Context {
    pub messages: Vec<Message>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// LLM Provider Trait
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn generate(&self, context: &Context) -> Result<Message>;
    async fn stream(
        &self,
        context: &Context,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>>;
}

/// Communication Channel Trait (simple base version)
/// For multi-platform messaging, see the `channel` module which provides
/// `Channel`, `ChannelPlugin`, and `ChannelOutbound` traits.
#[async_trait]
pub trait SimpleChannel: Send + Sync {
    fn name(&self) -> &str;
    async fn listen(&self) -> Result<futures::stream::BoxStream<'static, Result<SimpleIncomingMessage>>>;
    async fn send(&self, target_id: &str, message: &Message) -> Result<()>;
}

/// An incoming message from a channel (simple base version)
#[derive(Debug, Clone)]
pub struct SimpleIncomingMessage {
    pub source_id: String,
    pub message: Message,
    pub raw: Value,
}

/// Memory Backend Trait (using Tantivy as backend)
#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(&self, content: &str, metadata: Value) -> Result<String>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
}

/// Result from a memory search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub metadata: Value,
    pub score: f32,
}

/// Tool Trait
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, input: Value) -> Result<Value>;
}

/// Observer Trait for monitoring and security
#[async_trait]
pub trait Observer: Send + Sync {
    async fn observe_tool_execution(
        &self,
        tool_name: &str,
        input: &Value,
        output: &Result<Value>,
    ) -> Result<()>;
    async fn observe_message(&self, message: &Message) -> Result<()>;
    async fn scan_for_leaks(&self, data: &str) -> Result<LeakCheckResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakCheckResult {
    pub is_safe: bool,
    pub findings: Vec<String>,
}
