//! Providers module for LLM backends
//!
//! This module provides the provider system for MaesterClaw:
//! - `Provider` trait - Interface for LLM providers
//! - `ProviderCapabilities` - Feature flags for providers
//! - `ProviderError` - Error type for provider operations
//! - Concrete implementations (OpenAI, Anthropic, Ollama, OpenRouter)

mod r#trait;
mod capabilities;
mod error;
mod openai;
mod anthropic;
mod ollama;
mod openrouter;

pub use r#trait::{ChatResponse, Provider, StreamChunk, ToolCallDelta, TokenUsage, turn_to_message};
pub use capabilities::ProviderCapabilities;
pub use error::ProviderError;
pub use openai::{OpenAIConfig, OpenAIProvider};
pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use ollama::{OllamaConfig, OllamaProvider};
pub use openrouter::{OpenRouterConfig, OpenRouterProvider};
