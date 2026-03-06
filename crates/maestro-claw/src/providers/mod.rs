//! Providers module for LLM backends
//!
//! This module provides the provider system for MaesterClaw:
//! - `Provider` trait - Interface for LLM providers
//! - `ProviderCapabilities` - Feature flags for providers
//! - `ProviderError` - Error type for provider operations
//! - Concrete implementations (OpenAI, Anthropic, Ollama, OpenRouter)

mod anthropic;
mod capabilities;
mod error;
mod ollama;
mod openai;
mod openrouter;
mod r#trait;

pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use capabilities::ProviderCapabilities;
pub use error::ProviderError;
pub use ollama::{OllamaConfig, OllamaProvider};
pub use openai::{OpenAIConfig, OpenAIProvider};
pub use openrouter::{OpenRouterConfig, OpenRouterProvider};
pub use r#trait::{
    turn_to_message, ChatResponse, Provider, StreamChunk, TokenUsage, ToolCallDelta,
};
