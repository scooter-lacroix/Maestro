//! Anthropic provider implementation
//!
//! Supports Claude models with:
//! - Native tool use (function calling)
//! - Streaming responses
//! - Vision support

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ChatResponse, ProviderCapabilities, ProviderError, Provider, StreamChunk, TokenUsage};
use crate::session::{ToolCall, Turn, TurnRole};
use crate::tools::ToolSpec;

/// Anthropic API base URL
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1";

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
            body["tools"] = serde_json::to_value(tools).unwrap();
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
                    // Anthropic uses top-level system parameter
                    if system.is_none() {
                        system = Some(turn.content.clone());
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
                    let tool_result = turn.tool_call_id.as_ref().map(|id| {
                        AnthropicContent::ToolResult {
                            tool_use_id: id.clone(),
                            content: turn.content.clone(),
                            is_error: false,
                        }
                    });

                    if let Some(tr) = tool_result {
                        // Anthropic expects tool results in user messages
                        messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: vec![tr],
                        });
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

    /// Handle API error response
    fn handle_error(&self, status: reqwest::StatusCode, body: &str) -> ProviderError {
        match status.as_u16() {
            401 => ProviderError::AuthenticationFailed("Invalid API key".to_string()),
            429 => ProviderError::RateLimitExceeded(60),
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
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body));
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
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body));
        }

        let stream = response.bytes_stream().map(|chunk_result| {
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                match event {
                                    AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                            return Ok(StreamChunk {
                                                delta: Some(text.to_string()),
                                                tool_call_delta: None,
                                                finish_reason: None,
                                            });
                                        }
                                    }
                                    AnthropicStreamEvent::MessageStop => {
                                        return Ok(StreamChunk {
                                            delta: None,
                                            tool_call_delta: None,
                                            finish_reason: Some("end_turn".to_string()),
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok(StreamChunk {
                        delta: None,
                        tool_call_delta: None,
                        finish_reason: None,
                    })
                }
                Err(e) => Err(ProviderError::NetworkError(e.to_string())),
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
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body));
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
        messages.into_iter().map(|m| serde_json::to_value(m).unwrap()).collect()
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
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: u32 },
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
}
