//! OpenRouter provider implementation
//!
//! Supports multiple LLM providers through OpenRouter with:
//! - Multi-provider routing
//! - Cost tracking
//! - Streaming responses
//! - Fallback support

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ChatResponse, ProviderCapabilities, ProviderError, Provider, StreamChunk, TokenUsage};
use crate::session::{ToolCall, Turn, TurnRole};
use crate::tools::ToolSpec;

/// OpenRouter API base URL
const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1";

/// OpenRouter provider configuration
#[derive(Debug, Clone)]
pub struct OpenRouterConfig {
    /// API key
    pub api_key: String,
    /// Model to use (e.g., "openai/gpt-4", "anthropic/claude-3-opus")
    pub model: String,
    /// Base URL (for proxies)
    pub base_url: Option<String>,
    /// Site URL for rankings
    pub site_url: Option<String>,
    /// Site name
    pub site_name: Option<String>,
    /// Maximum tokens
    pub max_tokens: Option<u32>,
    /// Temperature
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Provider routing preferences
    pub provider: Option<ProviderRouting>,
}

/// Provider routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRouting {
    /// Only use these providers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Ignore these providers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Allow fallbacks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Require parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Data collection policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<String>,
}

impl OpenRouterConfig {
    /// Create a new config with API key and model
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: None,
            site_url: None,
            site_name: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            provider: None,
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set site URL
    pub fn with_site_url(mut self, url: String) -> Self {
        self.site_url = Some(url);
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

    /// Set provider routing
    pub fn with_provider_routing(mut self, provider: ProviderRouting) -> Self {
        self.provider = Some(provider);
        self
    }
}

/// OpenRouter provider
#[derive(Debug)]
pub struct OpenRouterProvider {
    config: OpenRouterConfig,
    client: Client,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider
    pub fn new(config: OpenRouterConfig) -> Result<Self, ProviderError> {
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
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| ProviderError::ConfigurationError("OPENROUTER_API_KEY not set".to_string()))?;

        let model = model
            .map(|s| s.to_string())
            .or_else(|| std::env::var("OPENROUTER_MODEL").ok())
            .unwrap_or_else(|| "openai/gpt-4o-mini".to_string());

        Self::new(OpenRouterConfig::new(api_key, model))
    }

    /// Get the API URL
    fn api_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or(OPENROUTER_API_URL)
    }

    /// Build the request body
    fn build_request_body(
        &self,
        messages: Vec<serde_json::Value>,
        tools: Option<Vec<OpenRouterTool>>,
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
            body["tools"] = serde_json::to_value(tools).unwrap();
        }

        if let Some(ref provider) = self.config.provider {
            body["provider"] = serde_json::to_value(provider).unwrap();
        }

        // Add transforms for proper tool calling
        body["transforms"] = serde_json::json!(["middle-out"]);

        body
    }

    /// Parse tool calls from response
    fn parse_tool_calls(tool_calls: Option<&[OpenRouterToolCall]>) -> Vec<ToolCall> {
        tool_calls
            .map(|tcs| {
                tcs.iter()
                    .map(|tc| {
                        let arguments: serde_json::Value = tc
                            .function
                            .arguments
                            .parse()
                            .unwrap_or(serde_json::Value::Null);
                        ToolCall::new(tc.id.clone(), tc.function.name.clone(), arguments)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Handle API error response
    fn handle_error(&self, status: reqwest::StatusCode, body: &str) -> ProviderError {
        match status.as_u16() {
            401 => ProviderError::AuthenticationFailed("Invalid API key".to_string()),
            402 => ProviderError::ProviderError("Insufficient credits".to_string()),
            429 => ProviderError::RateLimitExceeded(60),
            500 | 502 | 503 => ProviderError::Unavailable(format!("OpenRouter service error: {}", status)),
            _ => ProviderError::ProviderError(format!("API error ({}): {}", status, body)),
        }
    }
}

/// OpenRouter tool definition
#[derive(Debug, Clone, Serialize)]
struct OpenRouterTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenRouterFunctionDef,
}

impl From<&ToolSpec> for OpenRouterTool {
    fn from(spec: &ToolSpec) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: OpenRouterFunctionDef {
                name: spec.name.clone(),
                description: spec.description.clone(),
                parameters: spec.parameters.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct OpenRouterFunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// OpenRouter response format (OpenAI-compatible)
#[derive(Debug, Clone, Deserialize)]
struct OpenRouterResponse {
    id: String,
    provider: Option<String>,
    model: String,
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterChoice {
    index: u32,
    message: OpenRouterMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OpenRouterToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenRouterFunction,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    #[serde(default)]
    cost: Option<f64>,
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::openrouter()
    }

    async fn chat(&self, turns: &[Turn]) -> Result<ChatResponse, ProviderError> {
        let formatted = self.format_messages(turns);
        let body = self.build_request_body(formatted, None, false);

        let mut request = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        if let Some(ref site_url) = self.config.site_url {
            request = request.header("HTTP-Referer", site_url);
        }
        if let Some(ref site_name) = self.config.site_name {
            request = request.header("X-Title", site_name);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body));
        }

        let or_response: OpenRouterResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let choice = or_response
            .choices
            .first()
            .ok_or_else(|| ProviderError::ParseError("No choices in response".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();
        let tool_calls = Self::parse_tool_calls(choice.message.tool_calls.as_ref().map(|v| v.as_slice()));
        let usage = or_response.usage.map(|u| TokenUsage::new(
            u.prompt_tokens,
            u.completion_tokens,
        ));

        Ok(ChatResponse {
            content,
            tool_calls,
            model: or_response.model,
            usage,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn stream_chat(
        &self,
        turns: &[Turn],
    ) -> Result<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send + Unpin>, ProviderError> {
        let formatted = self.format_messages(turns);
        let body = self.build_request_body(formatted, None, true);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
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

        let stream = response.bytes_stream().map(|chunk_result| {
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                return Ok(StreamChunk {
                                    delta: None,
                                    tool_call_delta: None,
                                    finish_reason: Some("stop".to_string()),
                                });
                            }

                            if let Ok(chunk) = serde_json::from_str::<OpenRouterStreamResponse>(data) {
                                if let Some(choice) = chunk.choices.first() {
                                    let delta = choice.delta.content.clone();
                                    return Ok(StreamChunk {
                                        delta,
                                        tool_call_delta: None,
                                        finish_reason: choice.finish_reason.clone(),
                                    });
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
        let formatted = self.format_messages(turns);
        let or_tools: Vec<OpenRouterTool> = tools.iter().map(OpenRouterTool::from).collect();
        let body = self.build_request_body(formatted, Some(or_tools), false);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
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

        let or_response: OpenRouterResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let choice = or_response
            .choices
            .first()
            .ok_or_else(|| ProviderError::ParseError("No choices in response".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();
        let tool_calls = Self::parse_tool_calls(choice.message.tool_calls.as_ref().map(|v| v.as_slice()));
        let usage = or_response.usage.map(|u| TokenUsage::new(
            u.prompt_tokens,
            u.completion_tokens,
        ));

        Ok(ChatResponse {
            content,
            tool_calls,
            model: or_response.model,
            usage,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn warmup(&self) -> Result<(), ProviderError> {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "Hi"
        })];
        let body = self.build_request_body(messages, None, false);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
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
        let response = self
            .client
            .get(format!("{}/models", self.api_url()))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if response.status() == 401 {
            return Err(ProviderError::AuthenticationFailed("Invalid API key".to_string()));
        }

        if !response.status().is_success() {
            return Err(ProviderError::Unavailable("OpenRouter API unavailable".to_string()));
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

                if !turn.tool_calls.is_empty() {
                    msg["tool_calls"] = serde_json::to_value(
                        turn.tool_calls.iter().map(|tc| {
                            serde_json::json!({
                                "id": &tc.id,
                                "type": "function",
                                "function": {
                                    "name": &tc.name,
                                    "arguments": tc.arguments.to_string()
                                }
                            })
                        }).collect::<Vec<_>>()
                    ).unwrap();
                }

                if let Some(ref tool_call_id) = turn.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(tool_call_id);
                }

                msg
            })
            .collect()
    }
}

/// OpenRouter streaming response (OpenAI-compatible)
#[derive(Debug, Clone, Deserialize)]
struct OpenRouterStreamResponse {
    choices: Vec<OpenRouterStreamChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterStreamChoice {
    delta: OpenRouterDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterDelta {
    #[serde(default)]
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = OpenRouterConfig::new("test-key".to_string(), "openai/gpt-4".to_string());
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "openai/gpt-4");
    }

    #[test]
    fn test_provider_requires_api_key() {
        let config = OpenRouterConfig::new("".to_string(), "openai/gpt-4".to_string());
        let result = OpenRouterProvider::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_name_and_model() {
        let config = OpenRouterConfig::new("key".to_string(), "anthropic/claude-3-opus".to_string());
        let provider = OpenRouterProvider::new(config).unwrap();
        assert_eq!(provider.name(), "openrouter");
        assert_eq!(provider.model(), "anthropic/claude-3-opus");
    }

    #[test]
    fn test_provider_capabilities() {
        let config = OpenRouterConfig::new("key".to_string(), "openai/gpt-4".to_string());
        let provider = OpenRouterProvider::new(config).unwrap();
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(caps.vision);
    }

    #[test]
    fn test_provider_routing() {
        let routing = ProviderRouting {
            only: Some(vec!["openai".to_string()]),
            ignore: None,
            allow_fallbacks: Some(true),
            require_parameters: None,
            data_collection: None,
        };

        let config = OpenRouterConfig::new("key".to_string(), "openai/gpt-4".to_string())
            .with_provider_routing(routing);

        assert!(config.provider.is_some());
        let provider = config.provider.unwrap();
        assert_eq!(provider.only, Some(vec!["openai".to_string()]));
    }

    #[test]
    fn test_format_messages() {
        let config = OpenRouterConfig::new("key".to_string(), "openai/gpt-4".to_string());
        let provider = OpenRouterProvider::new(config).unwrap();

        let turns = vec![
            Turn::new(TurnRole::System, "You are helpful".to_string()),
            Turn::new(TurnRole::User, "Hello".to_string()),
        ];

        let messages = provider.format_messages(&turns);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
    }
}
