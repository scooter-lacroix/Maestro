//! Provider trait definition
//!
//! The Provider trait provides a standard interface for LLM backends.
//! This is a richer interface that can be adapted to the simplified
//! agent::Provider trait.

use async_trait::async_trait;
use futures::Stream;

use super::{ProviderCapabilities, ProviderError};
use crate::session::{ToolCall, Turn, TurnRole};
use crate::tools::ToolSpec;

/// A chat response from a provider
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The generated content
    pub content: String,
    /// Tool calls requested by the assistant (if any)
    pub tool_calls: Vec<ToolCall>,
    /// Model used for the response
    pub model: String,
    /// Usage statistics (prompt tokens, completion tokens)
    pub usage: Option<TokenUsage>,
    /// Finish reason (stop, tool_calls, length, etc.)
    pub finish_reason: Option<String>,
}

impl ChatResponse {
    /// Create a simple text response
    pub fn text(content: String) -> Self {
        Self {
            content,
            tool_calls: Vec::new(),
            model: String::new(),
            usage: None,
            finish_reason: Some("stop".to_string()),
        }
    }

    /// Create a response with tool calls
    pub fn with_tool_calls(tool_calls: Vec<ToolCall>, model: String) -> Self {
        Self {
            content: String::new(),
            tool_calls,
            model,
            usage: None,
            finish_reason: Some("tool_calls".to_string()),
        }
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u64,
    /// Tokens in the completion
    pub completion_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(prompt: u64, completion: u64) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }
}

/// A streaming chunk from a provider
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Delta content (incremental text)
    pub delta: Option<String>,
    /// Tool call delta (for streaming tool calls)
    pub tool_call_delta: Option<ToolCallDelta>,
    /// Finish reason (only in final chunk)
    pub finish_reason: Option<String>,
}

/// Incremental tool call data for streaming
#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    /// Tool call index
    pub index: usize,
    /// Tool call ID (only in first chunk)
    pub id: Option<String>,
    /// Tool name (only in first chunk)
    pub name: Option<String>,
    /// Arguments delta (JSON string)
    pub arguments_delta: Option<String>,
}

/// Provider trait for LLM backends
///
/// This is a rich provider interface with streaming, health checks,
/// and capability introspection. Concrete implementations can be
/// wrapped to implement the simpler agent::Provider trait.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the model identifier
    fn model(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;

    /// Check if provider supports native tool calling
    fn supports_native_tools(&self) -> bool {
        self.capabilities().native_tools
    }

    /// Send a chat completion request
    async fn chat(&self, messages: &[Turn]) -> Result<ChatResponse, ProviderError>;

    /// Send a streaming chat completion request
    async fn stream_chat(
        &self,
        messages: &[Turn],
    ) -> Result<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send + Unpin>, ProviderError>;

    /// Send a chat completion request with tool support
    async fn chat_with_tools(
        &self,
        messages: &[Turn],
        tools: &[ToolSpec],
    ) -> Result<ChatResponse, ProviderError>;

    /// Warm up the provider (validate connection, credentials)
    async fn warmup(&self) -> Result<(), ProviderError>;

    /// Check provider health
    async fn health_check(&self) -> Result<(), ProviderError>;

    /// Convert turns to provider-specific message format
    fn format_messages(&self, messages: &[Turn]) -> Vec<serde_json::Value>;
}

/// Helper function to convert a Turn to a generic message format
pub fn turn_to_message(turn: &Turn) -> serde_json::Value {
    let role = match turn.role {
        TurnRole::User => "user",
        TurnRole::Assistant => "assistant",
        TurnRole::System => "system",
        TurnRole::Tool => "tool",
    };

    let mut msg = serde_json::json!({
        "role": role,
        "content": turn.content,
    });

    // Add tool calls if present (for assistant messages)
    if !turn.tool_calls.is_empty() {
        msg["tool_calls"] = serde_json::to_value(&turn.tool_calls).unwrap();
    }

    // Add tool call ID for tool result messages
    if let Some(ref tool_call_id) = turn.tool_call_id {
        msg["tool_call_id"] = serde_json::json!(tool_call_id);
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_response_text() {
        let response = ChatResponse::text("Hello!".to_string());
        assert_eq!(response.content, "Hello!");
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_chat_response_with_tool_calls() {
        let tool_calls = vec![ToolCall::new(
            "call-1".to_string(),
            "test_tool".to_string(),
            serde_json::json!({"arg": "value"}),
        )];
        let response = ChatResponse::with_tool_calls(tool_calls.clone(), "gpt-4".to_string());
        assert!(response.content.is_empty());
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.finish_reason, Some("tool_calls".to_string()));
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk {
            delta: Some("Hello".to_string()),
            tool_call_delta: None,
            finish_reason: None,
        };
        assert_eq!(chunk.delta, Some("Hello".to_string()));
    }

    #[test]
    fn test_turn_to_message_user() {
        let turn = Turn::new(TurnRole::User, "Hi".to_string());
        let msg = turn_to_message(&turn);
        assert_eq!(msg["role"], "user");
        assert_eq!(msg["content"], "Hi");
    }

    #[test]
    fn test_turn_to_message_assistant() {
        let turn = Turn::new(TurnRole::Assistant, "Hello!".to_string());
        let msg = turn_to_message(&turn);
        assert_eq!(msg["role"], "assistant");
        assert_eq!(msg["content"], "Hello!");
    }

    #[test]
    fn test_turn_to_message_with_tool_call() {
        let mut turn = Turn::new(TurnRole::Assistant, String::new());
        turn.add_tool_call(
            "call-1".to_string(),
            "test".to_string(),
            serde_json::json!({}),
        );
        let msg = turn_to_message(&turn);
        assert!(msg["tool_calls"].is_array());
    }

    #[test]
    fn test_turn_to_message_tool_result() {
        let mut turn = Turn::new(TurnRole::Tool, "result".to_string());
        turn.set_tool_call_id("call-1".to_string());
        let msg = turn_to_message(&turn);
        assert_eq!(msg["role"], "tool");
        assert_eq!(msg["tool_call_id"], "call-1");
    }
}
