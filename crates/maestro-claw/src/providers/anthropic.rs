//! Anthropic provider implementation
//!
//! Supports Claude models with:
//! - Native tool use (function calling)
//! - Streaming responses
//! - Vision support

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::{ChatResponse, ProviderCapabilities, ProviderError, Provider, StreamChunk, ToolCallDelta, TokenUsage};
use crate::session::{ToolCall, Turn, TurnRole};
use crate::tools::ToolSpec;

/// Anthropic API base URL
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1";

/// LOW-5: Extract the `Retry-After` header value (seconds) before the response
/// body is consumed.  Falls back to 60 seconds when the header is absent.
fn extract_retry_after(response: &reqwest::Response) -> u64 {
    response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
}

/// LOW-12: Map a reqwest network error to the correct `ProviderError` variant.
/// Distinguishes genuine timeouts from other network failures.
fn map_network_error(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout(e.url().map(|_| 120u64).unwrap_or(120))
    } else {
        ProviderError::NetworkError(e.to_string())
    }
}

/// Anthropic provider configuration
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key
    pub api_key: String,
    /// Model to use (e.g., "claude-3-opus-20240229", "claude-3-sonnet-20240229")
    pub model: String,
    /// Base URL (for proxies)
    pub base_url: Option<String>,
    /// Maximum tokens in response
    pub max_tokens: u32,
    /// Temperature (0.0 - 1.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// System prompt (Anthropic uses top-level system)
    pub system: Option<String>,
}

impl AnthropicConfig {
    /// Create a new config with API key and model
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: None,
            max_tokens: 4096,
            temperature: None,
            top_p: None,
            system: None,
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set system prompt
    pub fn with_system(mut self, system: String) -> Self {
        self.system = Some(system);
        self
    }
}

/// Anthropic provider
#[derive(Debug)]
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(config: AnthropicConfig) -> Result<Self, ProviderError> {
        if config.api_key.is_empty() {
            return Err(ProviderError::ConfigurationError(
                "API key is required".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::ConfigurationError(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Create a provider from environment variable
    pub fn from_env(model: Option<&str>) -> Result<Self, ProviderError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ProviderError::ConfigurationError("ANTHROPIC_API_KEY not set".to_string()))?;

        let model = model
            .map(|s| s.to_string())
            .or_else(|| std::env::var("ANTHROPIC_MODEL").ok())
            .unwrap_or_else(|| "claude-3-sonnet-20240229".to_string());

        Self::new(AnthropicConfig::new(api_key, model))
    }

    /// Get the API URL
    fn api_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or(ANTHROPIC_API_URL)
    }

    /// Build the request body for messages API
    fn build_request_body(
        &self,
        messages: Vec<AnthropicMessage>,
        tools: Option<Vec<AnthropicTool>>,
        system: Option<&str>,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
        });

        if let Some(system) = system {
            body["system"] = serde_json::json!(system);
        }

        if let Some(temperature) = self.config.temperature {
            body["temperature"] = serde_json::json!(temperature);
        }

        if let Some(tools) = tools {
            // LOW-1: avoid panicking on serialization failure
            match serde_json::to_value(tools) {
                Ok(v) => { body["tools"] = v; }
                Err(e) => {
                    tracing::warn!("Failed to serialize Anthropic tools: {}", e);
                }
            }
        }

        body
    }

    /// Convert turns to Anthropic message format
    fn convert_messages(&self, turns: &[Turn]) -> (Vec<AnthropicMessage>, Option<String>) {
        let mut messages = Vec::new();
        let mut system = self.config.system.clone();

        for turn in turns {
            match turn.role {
                TurnRole::System => {
                    // LOW-11: Anthropic allows only one top-level system prompt.
                    // Concatenate multiple System turns separated by a blank line
                    // rather than silently dropping all but the first.
                    match system {
                        None => system = Some(turn.content.clone()),
                        Some(ref mut existing) => {
                            existing.push_str("\n\n");
                            existing.push_str(&turn.content);
                        }
                    }
                }
                TurnRole::User => {
                    messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: vec![AnthropicContent::Text {
                            text: turn.content.clone(),
                        }],
                    });
                }
                TurnRole::Assistant => {
                    let mut content = Vec::new();

                    if !turn.content.is_empty() {
                        content.push(AnthropicContent::Text {
                            text: turn.content.clone(),
                        });
                    }

                    for tc in &turn.tool_calls {
                        content.push(AnthropicContent::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        });
                    }

                    messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content,
                    });
                }
                TurnRole::Tool => {
                    // MED-4: Prefer `turn.tool_results` which carries accurate `is_error`.
                    // The agent loop populates this Vec for every tool execution.
                    if !turn.tool_results.is_empty() {
                        for tr in &turn.tool_results {
                            messages.push(AnthropicMessage {
                                role: "user".to_string(),
                                content: vec![AnthropicContent::ToolResult {
                                    tool_use_id: tr.tool_call_id.clone(),
                                    content: tr.content.clone(),
                                    is_error: tr.is_error,
                                }],
                            });
                        }
                    } else if let Some(id) = &turn.tool_call_id {
                        // Fallback for turns created without tool_results (legacy path).
                        // is_error cannot be determined here; default to false.
                        messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: vec![AnthropicContent::ToolResult {
                                tool_use_id: id.clone(),
                                content: turn.content.clone(),
                                is_error: false,
                            }],
                        });
                    } else {
                        // MED-4: Warn instead of silently dropping unidentified tool turns.
                        tracing::warn!(
                            turn_id = %turn.id,
                            "TurnRole::Tool turn has no tool_call_id and no tool_results; \
                             skipping — the tool result will not reach the LLM"
                        );
                    }
                }
            }
        }

        (messages, system)
    }

    /// Parse tool use from Anthropic response
    fn parse_tool_use(content: &[AnthropicResponseContent]) -> Vec<ToolCall> {
        content
            .iter()
            .filter_map(|c| match c {
                AnthropicResponseContent::ToolUse { id, name, input } => Some(ToolCall::new(
                    id.clone(),
                    name.clone(),
                    input.clone(),
                )),
                _ => None,
            })
            .collect()
    }

    /// Extract text from Anthropic response
    fn extract_text(content: &[AnthropicResponseContent]) -> String {
        content
            .iter()
            .filter_map(|c| match c {
                AnthropicResponseContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Handle API error response.
    ///
    /// `retry_after_secs` should be extracted from the `Retry-After` response
    /// header **before** the body is consumed (LOW-5).
    fn handle_error(&self, status: reqwest::StatusCode, body: &str, retry_after_secs: u64) -> ProviderError {
        match status.as_u16() {
            401 => ProviderError::AuthenticationFailed("Invalid API key".to_string()),
            // LOW-5: use server-provided retry delay instead of hardcoded 60
            429 => ProviderError::RateLimitExceeded(retry_after_secs),
            500 | 502 | 503 => ProviderError::Unavailable(format!("Anthropic service error: {}", status)),
            _ => ProviderError::ProviderError(format!("API error ({}): {}", status, body)),
        }
    }
}

/// Anthropic message format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

/// Anthropic content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

/// Anthropic tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

impl From<&ToolSpec> for AnthropicTool {
    fn from(spec: &ToolSpec) -> Self {
        Self {
            name: spec.name.clone(),
            description: spec.description.clone(),
            input_schema: spec.parameters.clone(),
        }
    }
}

/// Anthropic response format
#[derive(Debug, Clone, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicResponseContent>,
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AnthropicResponseContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::anthropic()
    }

    async fn chat(&self, turns: &[Turn]) -> Result<ChatResponse, ProviderError> {
        let (messages, system) = self.convert_messages(turns);
        let body = self.build_request_body(messages, None, system.as_deref());

        let response = self
            .client
            .post(format!("{}/messages", self.api_url()))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_network_error)?;

        let status = response.status();
        if !status.is_success() {
            // LOW-5: extract Retry-After before consuming the body
            let retry_after = extract_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body, retry_after));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let content = Self::extract_text(&anthropic_response.content);
        let tool_calls = Self::parse_tool_use(&anthropic_response.content);
        let finish_reason = anthropic_response.stop_reason.clone();

        Ok(ChatResponse {
            content,
            tool_calls,
            model: anthropic_response.model,
            usage: Some(TokenUsage::new(
                anthropic_response.usage.input_tokens,
                anthropic_response.usage.output_tokens,
            )),
            finish_reason,
        })
    }

    async fn stream_chat(
        &self,
        turns: &[Turn],
    ) -> Result<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send + Unpin>, ProviderError> {
        let (messages, system) = self.convert_messages(turns);
        let body = self.build_request_body(messages, None, system.as_deref());

        let response = self
            .client
            .post(format!("{}/messages", self.api_url()))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .query(&[("stream", "true")])
            .send()
            .await
            .map_err(map_network_error)?;

        let status = response.status();
        if !status.is_success() {
            // LOW-5: extract Retry-After before consuming the body
            let retry_after = extract_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body, retry_after));
        }

        // MED-3: Use flat_map + carryover buffer so that multiple SSE events
        // arriving in a single TCP frame are all emitted, and events that are
        // split across frame boundaries are reassembled correctly.
        let carryover: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

        let stream = response.bytes_stream().flat_map(move |chunk_result| {
            let carryover = carryover.clone();
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut carry = carryover.lock().unwrap_or_else(|e| e.into_inner());
                    carry.push_str(&text);

                    // Extract all complete lines (separated by \n)
                    let mut results: Vec<Result<StreamChunk, ProviderError>> = Vec::new();
                    loop {
                        match carry.find('\n') {
                            Some(pos) => {
                                let line = carry[..pos].trim_end_matches('\r').to_string();
                                let tail = carry[pos + 1..].to_string();
                                *carry = tail;

                                if !line.starts_with("data: ") {
                                    continue;
                                }
                                let data = &line[6..];
                                if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                    match event {
                                        AnthropicStreamEvent::ContentBlockDelta { index, delta } => {
                                            let delta_type = delta
                                                .get("type")
                                                .and_then(|t| t.as_str())
                                                .unwrap_or("");
                                            if delta_type == "text_delta" {
                                                if let Some(t) = delta.get("text").and_then(|t| t.as_str()) {
                                                    results.push(Ok(StreamChunk {
                                                        delta: Some(t.to_string()),
                                                        tool_call_delta: None,
                                                        finish_reason: None,
                                                    }));
                                                }
                                            } else if delta_type == "input_json_delta" {
                                                // LOW-10: surface streaming tool call argument chunks
                                                if let Some(partial) = delta
                                                    .get("partial_json")
                                                    .and_then(|p| p.as_str())
                                                {
                                                    results.push(Ok(StreamChunk {
                                                        delta: None,
                                                        tool_call_delta: Some(ToolCallDelta {
                                                            index: index as usize,
                                                            id: None,
                                                            name: None,
                                                            arguments_delta: Some(partial.to_string()),
                                                        }),
                                                        finish_reason: None,
                                                    }));
                                                }
                                            }
                                        }
                                        // LOW-10: when a tool_use block starts, emit id+name
                                        AnthropicStreamEvent::ContentBlockStart {
                                            index,
                                            content_block,
                                        } => {
                                            if content_block
                                                .get("type")
                                                .and_then(|t| t.as_str())
                                                == Some("tool_use")
                                            {
                                                let id = content_block
                                                    .get("id")
                                                    .and_then(|v| v.as_str())
                                                    .map(String::from);
                                                let name = content_block
                                                    .get("name")
                                                    .and_then(|v| v.as_str())
                                                    .map(String::from);
                                                results.push(Ok(StreamChunk {
                                                    delta: None,
                                                    tool_call_delta: Some(ToolCallDelta {
                                                        index: index as usize,
                                                        id,
                                                        name,
                                                        arguments_delta: None,
                                                    }),
                                                    finish_reason: None,
                                                }));
                                            }
                                        }
                                        AnthropicStreamEvent::MessageStop => {
                                            results.push(Ok(StreamChunk {
                                                delta: None,
                                                tool_call_delta: None,
                                                finish_reason: Some("end_turn".to_string()),
                                            }));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    drop(carry);
                    stream::iter(results)
                }
                Err(e) => stream::iter(vec![Err(map_network_error(e))]),
            }
        });

        Ok(Box::new(stream))
    }

    async fn chat_with_tools(
        &self,
        turns: &[Turn],
        tools: &[ToolSpec],
    ) -> Result<ChatResponse, ProviderError> {
        let (messages, system) = self.convert_messages(turns);
        let anthropic_tools: Vec<AnthropicTool> = tools.iter().map(AnthropicTool::from).collect();
        let body = self.build_request_body(messages, Some(anthropic_tools), system.as_deref());

        let response = self
            .client
            .post(format!("{}/messages", self.api_url()))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_network_error)?;

        let status = response.status();
        if !status.is_success() {
            // LOW-5: extract Retry-After before consuming the body
            let retry_after = extract_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body, retry_after));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let content = Self::extract_text(&anthropic_response.content);
        let tool_calls = Self::parse_tool_use(&anthropic_response.content);
        let finish_reason = anthropic_response.stop_reason.clone();

        Ok(ChatResponse {
            content,
            tool_calls,
            model: anthropic_response.model,
            usage: Some(TokenUsage::new(
                anthropic_response.usage.input_tokens,
                anthropic_response.usage.output_tokens,
            )),
            finish_reason,
        })
    }

    async fn warmup(&self) -> Result<(), ProviderError> {
        let messages = vec![AnthropicMessage {
            role: "user".to_string(),
            content: vec![AnthropicContent::Text {
                text: "Hi".to_string(),
            }],
        }];
        let body = self.build_request_body(messages, None, None);

        let response = self
            .client
            .post(format!("{}/messages", self.api_url()))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if response.status() == 401 {
            return Err(ProviderError::AuthenticationFailed("Invalid API key".to_string()));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Anthropic doesn't have a health endpoint, so we just check auth
        self.warmup().await
    }

    fn format_messages(&self, turns: &[Turn]) -> Vec<serde_json::Value> {
        let (messages, _system) = self.convert_messages(turns);
        // LOW-1: avoid panicking on serialization failure
        messages
            .into_iter()
            .filter_map(|m| match serde_json::to_value(m) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!("Failed to serialize Anthropic message: {}", e);
                    None
                }
            })
            .collect()
    }
}

/// Anthropic streaming event types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: serde_json::Value,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    /// LOW-10: capture content_block so tool_use blocks expose id/name as ToolCallDelta
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: serde_json::Value,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_start")]
    MessageStart { message: serde_json::Value },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: serde_json::Value },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = AnthropicConfig::new("test-key".to_string(), "claude-3-sonnet-20240229".to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "claude-3-sonnet-20240229");
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_provider_requires_api_key() {
        let config = AnthropicConfig::new("".to_string(), "claude-3-sonnet-20240229".to_string());
        let result = AnthropicProvider::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_name_and_model() {
        let config = AnthropicConfig::new("key".to_string(), "claude-3-opus-20240229".to_string());
        let provider = AnthropicProvider::new(config).unwrap();
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.model(), "claude-3-opus-20240229");
    }

    #[test]
    fn test_provider_capabilities() {
        let config = AnthropicConfig::new("key".to_string(), "claude-3-sonnet-20240229".to_string());
        let provider = AnthropicProvider::new(config).unwrap();
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(caps.vision);
    }

    #[test]
    fn test_convert_messages() {
        let config = AnthropicConfig::new("key".to_string(), "claude-3-sonnet-20240229".to_string());
        let provider = AnthropicProvider::new(config).unwrap();

        let turns = vec![
            Turn::new(TurnRole::System, "You are helpful".to_string()),
            Turn::new(TurnRole::User, "Hello".to_string()),
        ];

        let (messages, system) = provider.convert_messages(&turns);
        assert_eq!(messages.len(), 1);
        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(messages[0].role, "user");
    }

    #[test]
    fn test_multiple_system_turns_concatenated() {
        // LOW-11: Multiple System turns must be concatenated rather than dropped.
        let config = AnthropicConfig::new("key".to_string(), "claude-3-sonnet-20240229".to_string());
        let provider = AnthropicProvider::new(config).unwrap();

        let turns = vec![
            Turn::new(TurnRole::System, "Persona: assistant".to_string()),
            Turn::new(TurnRole::System, "Rules: be concise".to_string()),
            Turn::new(TurnRole::User, "Hi".to_string()),
        ];

        let (messages, system) = provider.convert_messages(&turns);
        assert_eq!(messages.len(), 1, "Only the User turn should appear in messages");
        let sys = system.unwrap();
        assert!(sys.contains("Persona: assistant"), "First system content must be present");
        assert!(sys.contains("Rules: be concise"), "Second system content must be concatenated");
    }

    #[test]
    fn test_tool_turn_is_error_propagated() {
        // MED-4: is_error from tool_results must be passed to the provider message.
        let config = AnthropicConfig::new("key".to_string(), "claude-3-sonnet-20240229".to_string());
        let provider = AnthropicProvider::new(config).unwrap();

        let mut tool_turn = Turn::new(TurnRole::Tool, "Tool failed!".to_string());
        tool_turn.set_tool_call_id("call-1".to_string());
        // Simulate agent loop storing is_error=true
        tool_turn.add_tool_result("call-1".to_string(), "Tool failed!".to_string(), true);

        let (messages, _) = provider.convert_messages(&[tool_turn]);
        assert_eq!(messages.len(), 1);
        // Verify the ToolResult content was emitted (is_error is inside the enum variant)
        assert_eq!(messages[0].role, "user");
        assert!(matches!(
            messages[0].content[0],
            AnthropicContent::ToolResult { is_error: true, .. }
        ));
    }

    #[test]
    fn test_tool_turn_no_id_warns() {
        // MED-4: Tool turn with no tool_call_id and no tool_results should be skipped (not panic).
        let config = AnthropicConfig::new("key".to_string(), "claude-3-sonnet-20240229".to_string());
        let provider = AnthropicProvider::new(config).unwrap();

        // Turn with no tool_call_id and no tool_results
        let tool_turn = Turn::new(TurnRole::Tool, "orphan result".to_string());

        let (messages, _) = provider.convert_messages(&[tool_turn]);
        // Should produce no messages (turn is skipped with a warning)
        assert_eq!(messages.len(), 0, "Orphan tool turn must be silently skipped");
    }

    /// LOW-10: Anthropic ContentBlockStart for tool_use must deserialise id and name
    #[test]
    fn test_anthropic_content_block_start_tool_use_parsing() {
        let json = r#"{
            "type":"content_block_start",
            "index":1,
            "content_block":{
                "type":"tool_use",
                "id":"toolu_01A09",
                "name":"get_weather",
                "input":{}
            }
        }"#;

        let event: AnthropicStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            AnthropicStreamEvent::ContentBlockStart { index, content_block } => {
                assert_eq!(index, 1);
                assert_eq!(
                    content_block.get("type").and_then(|t| t.as_str()),
                    Some("tool_use")
                );
                assert_eq!(
                    content_block.get("id").and_then(|v| v.as_str()),
                    Some("toolu_01A09")
                );
                assert_eq!(
                    content_block.get("name").and_then(|v| v.as_str()),
                    Some("get_weather")
                );
            }
            _ => panic!("Expected ContentBlockStart"),
        }
    }

    /// LOW-10: Anthropic input_json_delta must carry partial_json
    #[test]
    fn test_anthropic_input_json_delta_parsing() {
        let json = r#"{
            "type":"content_block_delta",
            "index":1,
            "delta":{"type":"input_json_delta","partial_json":"{\"location\":"}
        }"#;

        let event: AnthropicStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            AnthropicStreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 1);
                assert_eq!(
                    delta.get("type").and_then(|t| t.as_str()),
                    Some("input_json_delta")
                );
                assert_eq!(
                    delta.get("partial_json").and_then(|v| v.as_str()),
                    Some("{\"location\":")
                );
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }
}
