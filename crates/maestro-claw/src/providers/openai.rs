//! OpenAI provider implementation
//!
//! Supports GPT-4, GPT-4-turbo, GPT-3.5-turbo with:
//! - Native function calling
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

/// OpenAI API base URL
const OPENAI_API_URL: &str = "https://api.openai.com/v1";

/// LOW-5: Extract the `Retry-After` header value (seconds) before consuming the body.
fn extract_retry_after(response: &reqwest::Response) -> u64 {
    response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
}

/// LOW-12: Map a reqwest network error to the correct `ProviderError` variant.
fn map_network_error(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout(120)
    } else {
        ProviderError::NetworkError(e.to_string())
    }
}

/// OpenAI provider configuration
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// API key
    pub api_key: String,
    /// Model to use (e.g., "gpt-4", "gpt-3.5-turbo")
    pub model: String,
    /// Base URL (for proxies)
    pub base_url: Option<String>,
    /// Maximum tokens in response
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Organization ID
    pub organization: Option<String>,
}

impl OpenAIConfig {
    /// Create a new config with API key and model
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            organization: None,
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// OpenAI provider
#[derive(Debug)]
pub struct OpenAIProvider {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(config: OpenAIConfig) -> Result<Self, ProviderError> {
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
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::ConfigurationError("OPENAI_API_KEY not set".to_string()))?;

        let model = model
            .map(|s| s.to_string())
            .or_else(|| std::env::var("OPENAI_MODEL").ok())
            .unwrap_or_else(|| "gpt-4".to_string());

        Self::new(OpenAIConfig::new(api_key, model))
    }

    /// Get the API URL
    fn api_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or(OPENAI_API_URL)
    }

    /// Build the request body
    fn build_request_body(
        &self,
        messages: Vec<serde_json::Value>,
        tools: Option<Vec<ToolDefinition>>,
        stream: bool,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": stream,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temperature) = self.config.temperature {
            body["temperature"] = serde_json::json!(temperature);
        }

        if let Some(top_p) = self.config.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(tools) = tools {
            // LOW-1: avoid panicking on serialization failure
            match serde_json::to_value(tools) {
                Ok(v) => { body["tools"] = v; }
                Err(e) => {
                    tracing::warn!("Failed to serialize OpenAI tools: {}", e);
                }
            }
        }

        body
    }

    /// Parse tool calls from OpenAI response
    fn parse_tool_calls(
        tool_calls: &[OpenAIToolCall],
    ) -> Vec<ToolCall> {
        tool_calls
            .iter()
            .map(|tc| {
                let arguments: serde_json::Value = tc
                    .function
                    .arguments
                    .parse()
                    .unwrap_or(serde_json::Value::Null);
                ToolCall::new(tc.id.clone(), tc.function.name.clone(), arguments)
            })
            .collect()
    }

    /// Handle API error response
    /// `retry_after_secs` should be extracted from the `Retry-After` header before
    /// the response body is consumed (LOW-5).
    fn handle_error(&self, status: reqwest::StatusCode, body: &str, retry_after_secs: u64) -> ProviderError {
        match status.as_u16() {
            401 => ProviderError::AuthenticationFailed("Invalid API key".to_string()),
            // LOW-5: use server-provided retry delay
            429 => ProviderError::RateLimitExceeded(retry_after_secs),
            500 | 502 | 503 => ProviderError::Unavailable(format!("OpenAI service error: {}", status)),
            _ => ProviderError::ProviderError(format!("API error ({}): {}", status, body)),
        }
    }
}

/// OpenAI tool definition format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDefinition,
}

impl From<&ToolSpec> for ToolDefinition {
    fn from(spec: &ToolSpec) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: spec.name.clone(),
                description: spec.description.clone(),
                parameters: spec.parameters.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// OpenAI chat response format
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::openai()
    }

    async fn chat(&self, messages: &[Turn]) -> Result<ChatResponse, ProviderError> {
        let formatted = self.format_messages(messages);
        let body = self.build_request_body(formatted, None, false);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_network_error)?;

        let status = response.status();
        if !status.is_success() {
            let retry_after = extract_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body, retry_after));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let choice = openai_response
            .choices
            .first()
            .ok_or_else(|| ProviderError::ParseError("No choices in response".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();
        let tool_calls = choice
            .message
            .tool_calls
            .as_ref()
            .map(|tc| Self::parse_tool_calls(tc))
            .unwrap_or_default();

        let usage = openai_response.usage.map(|u| TokenUsage::new(
            u.prompt_tokens,
            u.completion_tokens,
        ));

        Ok(ChatResponse {
            content,
            tool_calls,
            model: openai_response.model,
            usage,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn stream_chat(
        &self,
        messages: &[Turn],
    ) -> Result<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send + Unpin>, ProviderError> {
        let formatted = self.format_messages(messages);
        let body = self.build_request_body(formatted, None, true);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_network_error)?;

        let status = response.status();
        if !status.is_success() {
            let retry_after = extract_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body, retry_after));
        }

        // MED-3: flat_map + carryover buffer so multiple SSE events per TCP
        // frame are all emitted and split-frame events are reassembled.
        let carryover: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

        let stream = response.bytes_stream().flat_map(move |chunk_result| {
            let carryover = carryover.clone();
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut carry = carryover.lock().unwrap_or_else(|e| e.into_inner());
                    carry.push_str(&text);

                    let mut results: Vec<Result<StreamChunk, ProviderError>> = Vec::new();
                    loop {
                        match carry.find('\n') {
                            Some(pos) => {
                                let line = carry[..pos].trim_end_matches('\r').to_string();
                                carry.drain(..=pos);

                                if !line.starts_with("data: ") {
                                    continue;
                                }
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    results.push(Ok(StreamChunk {
                                        delta: None,
                                        tool_call_delta: None,
                                        finish_reason: Some("stop".to_string()),
                                    }));
                                } else if let Ok(chunk) =
                                    serde_json::from_str::<OpenAIStreamResponse>(data)
                                {
                                    if let Some(choice) = chunk.choices.first() {
                                        // LOW-10: surface tool call deltas so callers can
                                        // reconstruct streaming tool calls by index
                                        let tool_call_delta = choice
                                            .delta
                                            .tool_calls
                                            .as_ref()
                                            .and_then(|tcs| tcs.first())
                                            .map(|tc| ToolCallDelta {
                                                index: tc.index,
                                                id: tc.id.clone(),
                                                name: tc
                                                    .function
                                                    .as_ref()
                                                    .and_then(|f| f.name.clone()),
                                                arguments_delta: tc
                                                    .function
                                                    .as_ref()
                                                    .and_then(|f| f.arguments.clone()),
                                            });
                                        results.push(Ok(StreamChunk {
                                            delta: choice.delta.content.clone(),
                                            tool_call_delta,
                                            finish_reason: choice.finish_reason.clone(),
                                        }));
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
        messages: &[Turn],
        tools: &[ToolSpec],
    ) -> Result<ChatResponse, ProviderError> {
        let formatted = self.format_messages(messages);
        let tool_defs: Vec<ToolDefinition> = tools.iter().map(ToolDefinition::from).collect();
        let body = self.build_request_body(formatted, Some(tool_defs), false);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_network_error)?;

        let status = response.status();
        if !status.is_success() {
            let retry_after = extract_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body, retry_after));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let choice = openai_response
            .choices
            .first()
            .ok_or_else(|| ProviderError::ParseError("No choices in response".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();
        let tool_calls = choice
            .message
            .tool_calls
            .as_ref()
            .map(|tc| Self::parse_tool_calls(tc))
            .unwrap_or_default();

        let usage = openai_response.usage.map(|u| TokenUsage::new(
            u.prompt_tokens,
            u.completion_tokens,
        ));

        Ok(ChatResponse {
            content,
            tool_calls,
            model: openai_response.model,
            usage,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn warmup(&self) -> Result<(), ProviderError> {
        // LOW-6: Previously made an expensive chat/completions call that wasted tokens.
        // Delegate to health_check() which uses the /models list endpoint instead.
        self.health_check().await
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple models list check
        let response = self
            .client
            .get(format!("{}/models", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(map_network_error)?;

        if response.status() == 401 {
            return Err(ProviderError::AuthenticationFailed("Invalid API key".to_string()));
        }

        if !response.status().is_success() {
            return Err(ProviderError::Unavailable("OpenAI API unavailable".to_string()));
        }

        Ok(())
    }

    fn format_messages(&self, turns: &[Turn]) -> Vec<serde_json::Value> {
        turns
            .iter()
            .map(|turn| {
                let role = match turn.role {
                    TurnRole::User => "user",
                    TurnRole::Assistant => "assistant",
                    TurnRole::System => "system",
                    TurnRole::Tool => "tool",
                };

                let mut msg = serde_json::json!({
                    "role": role,
                    "content": &turn.content,
                });

                // Add tool calls for assistant messages
                if !turn.tool_calls.is_empty() {
                    let tc_list = turn.tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "id": &tc.id,
                            "type": "function",
                            "function": {
                                "name": &tc.name,
                                "arguments": tc.arguments.to_string()
                            }
                        })
                    }).collect::<Vec<_>>();
                    // LOW-1: avoid panicking on serialization failure
                    match serde_json::to_value(tc_list) {
                        Ok(v) => { msg["tool_calls"] = v; }
                        Err(e) => {
                            tracing::warn!("Failed to serialize OpenAI tool_calls: {}", e);
                        }
                    }
                }

                // Add tool call ID for tool messages
                if let Some(ref tool_call_id) = turn.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(tool_call_id);
                }

                msg
            })
            .collect()
    }
}

/// OpenAI streaming response format
#[derive(Debug, Clone, Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCallDelta>>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIToolCallDelta {
    index: usize,
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAIFunctionCallDelta>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIFunctionCallDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = OpenAIConfig::new("test-key".to_string(), "gpt-4".to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "gpt-4");
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAIConfig::new("key".to_string(), "gpt-4".to_string())
            .with_max_tokens(1000)
            .with_temperature(0.7);

        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.temperature, Some(0.7));
    }

    #[test]
    fn test_provider_creation_requires_api_key() {
        let config = OpenAIConfig::new("".to_string(), "gpt-4".to_string());
        let result = OpenAIProvider::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_name_and_model() {
        let config = OpenAIConfig::new("key".to_string(), "gpt-4-turbo".to_string());
        let provider = OpenAIProvider::new(config).unwrap();
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.model(), "gpt-4-turbo");
    }

    #[test]
    fn test_provider_capabilities() {
        let config = OpenAIConfig::new("key".to_string(), "gpt-4".to_string());
        let provider = OpenAIProvider::new(config).unwrap();
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(caps.vision);
    }

    #[test]
    fn test_format_messages() {
        let config = OpenAIConfig::new("key".to_string(), "gpt-4".to_string());
        let provider = OpenAIProvider::new(config).unwrap();

        let turns = vec![
            Turn::new(TurnRole::System, "You are helpful".to_string()),
            Turn::new(TurnRole::User, "Hello".to_string()),
        ];

        let messages = provider.format_messages(&turns);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
    }

    #[test]
    fn test_tool_definition_from_spec() {
        let spec = ToolSpec::new(
            "test".to_string(),
            "A test tool".to_string(),
            serde_json::json!({"type": "object"}),
        );
        let tool_def = ToolDefinition::from(&spec);
        assert_eq!(tool_def.function.name, "test");
        assert_eq!(tool_def.tool_type, "function");
    }

    #[test]
    fn test_parse_tool_calls() {
        let openai_tool_calls = vec![OpenAIToolCall {
            id: "call-1".to_string(),
            tool_type: "function".to_string(),
            function: OpenAIFunctionCall {
                name: "test".to_string(),
                arguments: r#"{"arg": "value"}"#.to_string(),
            },
        }];

        let tool_calls = OpenAIProvider::parse_tool_calls(&openai_tool_calls);
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call-1");
        assert_eq!(tool_calls[0].name, "test");
    }

    /// LOW-10: OpenAI streaming SSE with tool_calls delta must produce ToolCallDelta
    #[test]
    fn test_openai_stream_tool_call_delta_parsing() {
        // Simulate parsing the SSE delta JSON the way the stream loop does it
        let sse_json = r#"{
            "id":"chatcmpl-1",
            "choices":[{
                "index":0,
                "delta":{
                    "tool_calls":[{
                        "index":0,
                        "id":"call_abc123",
                        "type":"function",
                        "function":{"name":"bash","arguments":""}
                    }]
                },
                "finish_reason":null
            }]
        }"#;

        let chunk: OpenAIStreamResponse = serde_json::from_str(sse_json).unwrap();
        let choice = chunk.choices.first().unwrap();
        let tc = choice.delta.tool_calls.as_ref().unwrap().first().unwrap();

        assert_eq!(tc.index, 0);
        assert_eq!(tc.id.as_deref(), Some("call_abc123"));
        assert_eq!(
            tc.function.as_ref().and_then(|f| f.name.as_deref()),
            Some("bash")
        );
    }

    /// LOW-10: subsequent argument chunks use index without id/name
    #[test]
    fn test_openai_stream_tool_call_arguments_delta() {
        let sse_json = r#"{
            "id":"chatcmpl-2",
            "choices":[{
                "index":0,
                "delta":{
                    "tool_calls":[{
                        "index":0,
                        "function":{"arguments":"{\"cmd\":\"ls\"}"}
                    }]
                },
                "finish_reason":null
            }]
        }"#;

        let chunk: OpenAIStreamResponse = serde_json::from_str(sse_json).unwrap();
        let choice = chunk.choices.first().unwrap();
        let tc = choice.delta.tool_calls.as_ref().unwrap().first().unwrap();

        assert_eq!(tc.index, 0);
        assert!(tc.id.is_none(), "subsequent chunks have no id");
        assert_eq!(
            tc.function.as_ref().and_then(|f| f.arguments.as_deref()),
            Some("{\"cmd\":\"ls\"}")
        );
    }
}
