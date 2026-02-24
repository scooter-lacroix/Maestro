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

pub use r#trait::{ChatResponse, Provider, StreamChunk, TokenUsage, turn_to_message};
pub use capabilities::ProviderCapabilities;
pub use error::ProviderError;
pub use openai::{OpenAIConfig, OpenAIProvider};
pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use openrouter::OpenRouterProvider;
