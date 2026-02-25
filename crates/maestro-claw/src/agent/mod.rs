//! Agent module for AI agent execution
//!
//! This module provides the agent loop and execution context:
//! - `agent_loop` - Core turn-by-turn execution function
//! - `AgentConfig` - Configuration for agent execution
//! - `AgentError` - Errors from agent execution
//! - `AgentResult` - Result from agent execution
//! - `Provider` - Trait for AI providers (simplified)
//! - `ProviderAdapter` - Bridges `providers::Provider` → `agent::Provider` (MED-6)
//! - `ProviderResponse` - Response from providers

mod r#loop;
mod adapter;

pub use r#loop::{agent_loop, AgentConfig, AgentError, AgentResult, ErrorStrategy, Provider, ProviderResponse};
pub use adapter::ProviderAdapter;
