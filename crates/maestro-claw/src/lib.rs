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
pub mod channels;
pub mod config;
pub mod cost;
pub mod cron;
pub mod daemon;
pub mod doctor;
pub mod health;
pub mod heartbeat;
pub mod hooks;
pub mod observability;
pub mod onboard;
pub mod service;
pub mod session;
pub mod skills;
pub mod tools;

#[cfg(feature = "providers")]
pub mod providers;

#[cfg(feature = "gateway")]
pub mod gateway;

#[cfg(feature = "core-integration")]
pub mod integration;

// Re-export commonly used session types
pub use session::{Session, SessionMetadata, Thread, ToolCall, ToolResult, Turn, TurnRole};

// Re-export tools types
pub use tools::builtin::{
    CommandRiskLevel, CronAddTool, CronListTool, CronRemoveTool, FileTool, FileToolConfig,
    MemoryTool, ShellTool, ShellToolConfig,
};
pub use tools::{Tool, ToolOutput, ToolRegistry, ToolSpec};

// Re-export agent types
pub use agent::{
    agent_loop, build_default_hook_system, build_default_tool_registry,
    build_default_tool_registry_with_extras, build_default_tools, build_tool_registry, run_prompt,
    run_thread, AgentConfig, AgentError, AgentResult, ErrorStrategy, Provider, ProviderAdapter,
    ProviderResponse,
};

// Re-export hooks types
pub use hooks::builtin::{LoggingHook, MemoryHook};
pub use hooks::{Hook, HookContext, HookError, HookSystem};

// Re-export provider types when feature is enabled
#[cfg(feature = "providers")]
pub use providers::{
    AnthropicConfig, AnthropicProvider, ChatResponse, OllamaConfig, OllamaProvider, OpenAIConfig,
    OpenAIProvider, OpenRouterConfig, OpenRouterProvider, Provider as LlmProvider,
    ProviderCapabilities, ProviderError, StreamChunk, TokenUsage, ToolCallDelta,
};

// Re-export integration types when feature is enabled
#[cfg(feature = "core-integration")]
pub use integration::{
    ApprovalCallback, AutonomyLevel, Channel, ChannelBridge, ChannelBridgeError, ChannelPlugin,
    ChannelRegistry, ExecutionRequest, IncomingMessage, Memory, MemoryBridge, MemoryBridgeError,
    OutgoingResponse, ResourceLimits, RuntimeAdapter, SandboxManager, SandboxResult, SearchResult,
    SecurityPolicy, SecurityPolicyBridge, SecurityPolicyError,
};
