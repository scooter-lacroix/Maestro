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

mod adapter;
pub mod cli_provider;
mod r#loop;
mod runtime;

pub use adapter::ProviderAdapter;
pub use cli_provider::{CliProvider, CliProviderConfig, KNOWN_TOOLS};
pub use r#loop::{
    agent_loop, AgentConfig, AgentError, AgentResult, ErrorStrategy, Provider, ProviderResponse,
};
pub use runtime::{
    build_default_hook_system, build_default_tool_registry,
    build_default_tool_registry_with_extras, build_default_tools, build_tool_registry, run_prompt,
    run_thread,
};
