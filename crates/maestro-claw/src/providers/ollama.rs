//! Ollama provider implementation
//!
//! Supports local models via Ollama with:
//! - Native tool calling (model dependent)
//! - Streaming responses
//! - Local inference

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ChatResponse, ProviderCapabilities, ProviderError, Provider, StreamChunk, TokenUsage};
use crate::session::{ToolCall, Turn, TurnRole};
use crate::tools::ToolSpec;

/// Ollama API base URL
const OLLAMA_API_URL: &str = "http://localhost:11434";

/// Ollama provider configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Base URL for Ollama server
    pub base_url: String,
    /// Model to use (e.g., "llama3", "mistral", "codellama")
    pub model: String,
    /// Temperature (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Number of tokens to predict
    pub num_predict: Option<i32>,
    /// Context window size
    pub num_ctx: Option<u32>,
    /// Whether to use GPU
    pub use_gpu: Option<bool>,
}

impl OllamaConfig {
    /// Create a new config with model
    pub fn new(model: String) -> Self {
        Self {
            base_url: OLLAMA_API_URL.to_string(),
            model,
            temperature: None,
            top_p: None,
            num_predict: None,
            num_ctx: None,
            use_gpu: None,
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set num_predict
    pub fn with_num_predict(mut self, num_predict: i32) -> Self {
        self.num_predict = Some(num_predict);
        self
    }

    /// Set context window size
    pub fn with_num_ctx(mut self, num_ctx: u32) -> Self {
        self.num_ctx = Some(num_ctx);
        self
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self::new("llama3".to_string())
    }
}

/// Ollama provider
#[derive(Debug)]
pub struct OllamaProvider {
    config: OllamaConfig,
    client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: OllamaConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // Longer timeout for local models
            .build()
            .map_err(|e| ProviderError::ConfigurationError(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Create a provider from environment variable
    pub fn from_env(model: Option<&str>) -> Result<Self, ProviderError> {
        let model = model
            .map(|s| s.to_string())
            .or_else(|| std::env::var("OLLAMA_MODEL").ok())
            .unwrap_or_else(|| "llama3".to_string());

        let base_url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_API_URL.to_string());

        let config = OllamaConfig::new(model).with_base_url(base_url);
        Self::new(config)
    }

    /// Build the request options
    fn build_options(&self) -> serde_json::Value {
        let mut options = serde_json::json!({});

        if let Some(temp) = self.config.temperature {
            options["temperature"] = serde_json::json!(temp);
        }

        if let Some(top_p) = self.config.top_p {
            options["top_p"] = serde_json::json!(top_p);
        }

        if let Some(num_predict) = self.config.num_predict {
            options["num_predict"] = serde_json::json!(num_predict);
        }

        if let Some(num_ctx) = self.config.num_ctx {
            options["num_ctx"] = serde_json::json!(num_ctx);
        }

        if let Some(use_gpu) = self.config.use_gpu {
            options["use_gpu"] = serde_json::json!(use_gpu);
        }

        options
    }

    /// Convert turns to Ollama message format
    fn convert_messages(&self, turns: &[Turn]) -> Vec<OllamaMessage> {
        turns
            .iter()
            .filter_map(|turn| {
                let role = match turn.role {
                    TurnRole::User => "user",
                    TurnRole::Assistant => "assistant",
                    TurnRole::System => "system",
                    TurnRole::Tool => return None, // Ollama doesn't have tool role
                };

                let mut images = Vec::new();

                // Handle tool calls for assistant messages
                let content = if !turn.tool_calls.is_empty() && turn.content.is_empty() {
                    // Format tool calls as text for models that don't support native tools
                    turn.tool_calls
                        .iter()
                        .map(|tc| format!("[Tool: {}({})]", tc.name, tc.arguments))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    turn.content.clone()
                };

                Some(OllamaMessage {
                    role: role.to_string(),
                    content,
                    images: if images.is_empty() { None } else { Some(images) },
                    tool_calls: if turn.tool_calls.is_empty() {
                        None
                    } else {
                        Some(
                            turn.tool_calls
                                .iter()
                                .map(|tc| OllamaToolCall {
                                    function: OllamaFunction {
                                        name: tc.name.clone(),
                                        arguments: tc.arguments.clone(),
                                    },
                                })
                                .collect(),
                        )
                    },
                })
            })
            .collect()
    }

    /// Parse tool calls from Ollama response
    fn parse_tool_calls(tool_calls: Option<&[OllamaResponseToolCall]>) -> Vec<ToolCall> {
        tool_calls
            .map(|tcs| {
                tcs.iter()
                    .map(|tc| {
                        ToolCall::new(
                            uuid::Uuid::new_v4().to_string(), // Ollama doesn't provide IDs
                            tc.function.name.clone(),
                            tc.function.arguments.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Handle API error response
    fn handle_error(&self, status: reqwest::StatusCode, body: &str) -> ProviderError {
        match status.as_u16() {
            404 => ProviderError::ModelNotFound("Model not found. Run 'ollama pull <model>'".to_string()),
            500 => ProviderError::ProviderError(format!("Ollama server error: {}", body)),
            _ => ProviderError::ProviderError(format!("API error ({}): {}", status, body)),
        }
    }
}

/// Ollama message format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolCall {
    function: OllamaFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaFunction {
    name: String,
    arguments: serde_json::Value,
}

/// Ollama request format
#[derive(Debug, Clone, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
    stream: bool,
    options: serde_json::Value,
}

/// Ollama tool definition
#[derive(Debug, Clone, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaToolDefinition,
}

impl From<&ToolSpec> for OllamaTool {
    fn from(spec: &ToolSpec) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: OllamaToolDefinition {
                name: spec.name.clone(),
                description: spec.description.clone(),
                parameters: spec.parameters.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct OllamaToolDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Ollama response format
#[derive(Debug, Clone, Deserialize)]
struct OllamaResponse {
    model: String,
    message: Option<OllamaResponseMessage>,
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Vec<OllamaResponseToolCall>,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaResponseToolCall {
    function: OllamaResponseFunction,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaResponseFunction {
    name: String,
    arguments: serde_json::Value,
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::ollama()
    }

    async fn chat(&self, turns: &[Turn]) -> Result<ChatResponse, ProviderError> {
        let messages = self.convert_messages(turns);
        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages,
            tools: None,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let message = ollama_response
            .message
            .ok_or_else(|| ProviderError::ParseError("No message in response".to_string()))?;

        let tool_calls = Self::parse_tool_calls(
            if message.tool_calls.is_empty() {
                None
            } else {
                Some(&message.tool_calls)
            },
        );

        let usage = ollama_response.eval_count.map(|completion| {
            let prompt = ollama_response.prompt_eval_count.unwrap_or(0);
            TokenUsage::new(prompt, completion)
        });

        Ok(ChatResponse {
            content: message.content,
            tool_calls,
            model: ollama_response.model,
            usage,
            finish_reason: if ollama_response.done {
                Some("stop".to_string())
            } else {
                None
            },
        })
    }

    async fn stream_chat(
        &self,
        turns: &[Turn],
    ) -> Result<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send + Unpin>, ProviderError> {
        let messages = self.convert_messages(turns);
        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages,
            tools: None,
            stream: true,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
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
                    // Ollama streams JSON objects, one per line
                    for line in text.lines() {
                        if let Ok(resp) = serde_json::from_str::<OllamaResponse>(line) {
                            let delta = resp.message.map(|m| m.content);

                            return Ok(StreamChunk {
                                delta,
                                tool_call_delta: None,
                                finish_reason: if resp.done {
                                    Some("stop".to_string())
                                } else {
                                    None
                                },
                            });
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
        let messages = self.convert_messages(turns);
        let ollama_tools: Vec<OllamaTool> = tools.iter().map(OllamaTool::from).collect();

        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages,
            tools: Some(ollama_tools),
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.handle_error(status, &body));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let message = ollama_response
            .message
            .ok_or_else(|| ProviderError::ParseError("No message in response".to_string()))?;

        let tool_calls = Self::parse_tool_calls(
            if message.tool_calls.is_empty() {
                None
            } else {
                Some(&message.tool_calls)
            },
        );

        let usage = ollama_response.eval_count.map(|completion| {
            let prompt = ollama_response.prompt_eval_count.unwrap_or(0);
            TokenUsage::new(prompt, completion)
        });

        Ok(ChatResponse {
            content: message.content,
            tool_calls,
            model: ollama_response.model,
            usage,
            finish_reason: if ollama_response.done {
                Some("stop".to_string())
            } else {
                None
            },
        })
    }

    async fn warmup(&self) -> Result<(), ProviderError> {
        // Just check if Ollama is running
        self.health_check().await
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.config.base_url))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(format!("Cannot connect to Ollama: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProviderError::Unavailable(
                "Ollama server not responding. Is it running?".to_string(),
            ));
        }

        Ok(())
    }

    fn format_messages(&self, turns: &[Turn]) -> Vec<serde_json::Value> {
        self.convert_messages(turns)
            .into_iter()
            .map(|m| serde_json::to_value(m).unwrap())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = OllamaConfig::new("llama3".to_string());
        assert_eq!(config.model, "llama3");
        assert_eq!(config.base_url, OLLAMA_API_URL);
    }

    #[test]
    fn test_config_builder() {
        let config = OllamaConfig::new("mistral".to_string())
            .with_temperature(0.7)
            .with_num_ctx(4096);

        assert_eq!(config.model, "mistral");
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.num_ctx, Some(4096));
    }

    #[test]
    fn test_provider_name_and_model() {
        let config = OllamaConfig::new("codellama".to_string());
        let provider = OllamaProvider::new(config).unwrap();
        assert_eq!(provider.name(), "ollama");
        assert_eq!(provider.model(), "codellama");
    }

    #[test]
    fn test_provider_capabilities() {
        let config = OllamaConfig::new("llama3".to_string());
        let provider = OllamaProvider::new(config).unwrap();
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(!caps.parallel_tool_calls);
    }

    #[test]
    fn test_convert_messages() {
        let config = OllamaConfig::new("llama3".to_string());
        let provider = OllamaProvider::new(config).unwrap();

        let turns = vec![
            Turn::new(TurnRole::System, "You are helpful".to_string()),
            Turn::new(TurnRole::User, "Hello".to_string()),
        ];

        let messages = provider.convert_messages(&turns);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }
}
