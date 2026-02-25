//! Maestro Claw - AI Agent Framework
//!
//! This crate provides the core components for building AI agent applications:
//! - `session` - Conversation state hierarchy (Session, Thread, Turn)
//! - `tools` - Tool trait and Registry
//! - `agent` - Agent loop and execution context
//! - `providers` - LLM provider implementations
//! - `hooks` - Pre/post processing hooks
//! - `integration` - maestro-core integration (optional)
//!
//! ## Features
//!
//! - `providers` (default) - LLM provider implementations (OpenAI, Anthropic, Ollama)
//! - `core-integration` - Integration with maestro-core traits (SecurityPolicy, Memory, Channel)

pub mod agent;
pub mod hooks;
pub mod session;
pub mod tools;

#[cfg(feature = "providers")]
pub mod providers;

#[cfg(feature = "core-integration")]
pub mod integration;

// Re-export commonly used session types
pub use session::{Session, SessionMetadata, Thread, Turn, TurnRole, ToolCall, ToolResult};

// Re-export tools types
pub use tools::{Tool, ToolOutput, ToolRegistry, ToolSpec};
pub use tools::builtin::{FileTool, MemoryTool, ShellTool, CommandRiskLevel};

// Re-export agent types
pub use agent::{agent_loop, AgentConfig, AgentError, AgentResult, ErrorStrategy, Provider, ProviderAdapter, ProviderResponse};

// Re-export hooks types
pub use hooks::{Hook, HookContext, HookError, HookSystem};
pub use hooks::builtin::{LoggingHook, MemoryHook};

// Re-export provider types when feature is enabled
#[cfg(feature = "providers")]
pub use providers::{
    ChatResponse, Provider as LlmProvider, ProviderCapabilities, ProviderError,
    StreamChunk, ToolCallDelta, TokenUsage,
    OpenAIConfig, OpenAIProvider,
    AnthropicConfig, AnthropicProvider,
    OllamaConfig, OllamaProvider,
    OpenRouterConfig, OpenRouterProvider,
};

// Re-export integration types when feature is enabled
#[cfg(feature = "core-integration")]
pub use integration::{
    SecurityPolicyBridge, SecurityPolicyError,
    MemoryBridge, MemoryBridgeError,
    ChannelBridge, ChannelBridgeError,
    AutonomyLevel, SecurityPolicy, SandboxManager, RuntimeAdapter,
    ExecutionRequest, SandboxResult, ResourceLimits,
    Memory, SearchResult,
    Channel, ChannelPlugin, ChannelRegistry, IncomingMessage, OutgoingResponse,
};
