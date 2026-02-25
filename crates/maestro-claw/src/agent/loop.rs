//! Agent loop - Core turn-by-turn execution
//!
//! The agent_loop function orchestrates the conversation with an AI provider,
//! handling tool calls, hooks, and termination conditions.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use crate::hooks::{HookContext, HookError, HookSystem};
use crate::session::{Thread, ToolCall, Turn, TurnRole};
use crate::tools::ToolRegistry;

/// Configuration for agent execution
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Maximum number of turns before termination
    pub max_turns: usize,
    /// Timeout per turn (in seconds)
    pub turn_timeout_secs: u64,
    /// Error handling strategy
    pub error_strategy: ErrorStrategy,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 20,
            turn_timeout_secs: 60,
            error_strategy: ErrorStrategy::Retry(3),
        }
    }
}

impl AgentConfig {
    /// Create a new agent config with max_turns
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Create a new agent config with timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.turn_timeout_secs = secs;
        self
    }

    /// Create a new agent config with error strategy
    pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }
}

/// Error handling strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorStrategy {
    /// Retry N times before failing
    Retry(usize),
    /// Skip errors and continue
    Skip,
    /// Abort immediately on any error
    Abort,
}

/// Result from agent execution
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Final turn from the agent
    pub final_turn: Turn,
    /// Total turns executed
    pub total_turns: usize,
    /// Tool calls executed
    pub tool_calls_executed: usize,
    /// Whether the agent completed normally
    pub completed_normally: bool,
    /// Termination reason
    pub termination_reason: String,
}

impl AgentResult {
    /// Get the final response content
    pub fn content(&self) -> &str {
        &self.final_turn.content
    }
}

/// Error from agent execution
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// Maximum turns exceeded
    #[error("Maximum turns exceeded: {0}")]
    MaxTurnsExceeded(usize),

    /// Turn timeout exceeded
    #[error("Turn timeout exceeded after {0}s")]
    TimeoutExceeded(u64),

    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Tool execution error
    #[error("Tool execution error for '{tool}': {message}")]
    ToolError {
        /// Tool name
        tool: String,
        /// Error message
        message: String,
    },

    /// Hook error
    #[error("Hook error: {0}")]
    HookError(#[from] HookError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// No provider configured
    #[error("No provider configured")]
    NoProvider,
}

/// Provider trait for agent loop (simplified interface)
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Execute a request with the thread context and available tools
    async fn execute(
        &self,
        messages: Vec<crate::session::ProviderMessage>,
        tools: Vec<crate::tools::ToolSpec>,
    ) -> Result<ProviderResponse, AgentError>;
}

/// Response from a provider
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    /// Response content (may be empty if tool calls present)
    pub content: String,
    /// Tool calls requested by the assistant
    pub tool_calls: Vec<ToolCall>,
    /// Whether the provider finished (text response without tool calls)
    pub is_finished: bool,
}

impl ProviderResponse {
    /// Create a text-only response
    pub fn text(content: String) -> Self {
        Self {
            content,
            tool_calls: Vec::new(),
            is_finished: true,
        }
    }

    /// Create a response with tool calls
    pub fn with_tools(content: String, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content,
            tool_calls,
            is_finished: false,
        }
    }
}

/// Execute the agent loop
///
/// This is the core function that orchestrates the conversation:
/// 1. Get messages from thread
/// 2. Execute pre-hooks
/// 3. Call provider
/// 4. Execute post-hooks
/// 5. Detect and execute tool calls
/// 6. Add turns and repeat until finished
///
/// # Arguments
/// * `thread` - The conversation thread
/// * `provider` - The AI provider to use
/// * `tools` - Tool registry for tool execution
/// * `hooks` - Hook system for pre/post processing
/// * `config` - Agent configuration
///
/// # Returns
/// The final result of the agent execution
pub async fn agent_loop(
    thread: &mut Thread,
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    hooks: Arc<HookSystem>,
    config: AgentConfig,
) -> Result<AgentResult, AgentError> {
    let mut current_turn = 0;
    let mut tool_calls_executed = 0;

    loop {
        // Check max turns
        if current_turn >= config.max_turns {
            return Ok(AgentResult {
                final_turn: thread
                    .turns
                    .last()
                    .cloned()
                    .unwrap_or_else(|| Turn::new(TurnRole::Assistant, String::new())),
                total_turns: current_turn,
                tool_calls_executed,
                completed_normally: false,
                termination_reason: format!("Max turns ({}) exceeded", config.max_turns),
            });
        }

        // Build hook context
        let hook_context = HookContext::new(
            current_turn,
            config.max_turns,
            thread.session_id().to_string(),
            thread.id().to_string(),
            "provider".to_string(), // Could be from provider.name() if trait had it
        );

        // Execute pre-hooks on the current user/input turn (last turn in thread)
        // This allows hooks to log, inject memory context, or modify the input before provider call
        if let Some(last_turn) = thread.turns.last() {
            let _ = hooks.execute_pre(&hook_context, last_turn)?;
        }

        // Get messages from thread
        let messages = thread.to_messages();
        let tool_specs = tools.to_tool_specs();

        // Execute provider with timeout
        let provider_result = timeout(
            Duration::from_secs(config.turn_timeout_secs),
            provider.execute(messages, tool_specs),
        )
        .await
        .map_err(|_| AgentError::TimeoutExceeded(config.turn_timeout_secs))??;

        // Create assistant turn
        let mut assistant_turn = Turn::new(TurnRole::Assistant, provider_result.content.clone());
        for tc in provider_result.tool_calls.clone() {
            assistant_turn.add_tool_call(tc.id, tc.name, tc.arguments);
        }

        // Execute post-hooks on assistant turn
        assistant_turn = hooks.execute_post(&hook_context, &assistant_turn)?;

        // Add assistant turn to thread
        thread.add_turn(assistant_turn.clone());

        // Check if finished (text response without tool calls)
        if provider_result.is_finished || provider_result.tool_calls.is_empty() {
            return Ok(AgentResult {
                final_turn: assistant_turn,
                total_turns: current_turn + 1,
                tool_calls_executed,
                completed_normally: true,
                termination_reason: "Text response received".to_string(),
            });
        }

        // Execute tool calls
        for tool_call in &provider_result.tool_calls {
            let tool = match tools.get(&tool_call.name) {
                Some(t) => t,
                None => {
                    // Tool not found - create error result
                    let error_content = format!(
                        "Tool '{}' not found in registry",
                        tool_call.name
                    );
                    let mut tool_turn = Turn::new(TurnRole::Tool, error_content.clone());
                    tool_turn.set_tool_call_id(tool_call.id.clone());
                    // MED-4: store is_error=true so providers can relay error status
                    tool_turn.add_tool_result(tool_call.id.clone(), error_content, true);
                    thread.add_turn(tool_turn);
                    continue;
                }
            };

            // Execute tool - ToolOutput already contains is_error flag
            let tool_result = tool.execute(tool_call.arguments.clone()).await;

            tool_calls_executed += 1;

            // Add tool result turn
            // MED-4: store is_error in tool_results so providers can relay accurate error status
            let mut tool_turn = Turn::new(TurnRole::Tool, tool_result.content.clone());
            tool_turn.set_tool_call_id(tool_call.id.clone());
            tool_turn.add_tool_result(
                tool_call.id.clone(),
                tool_result.content.clone(),
                tool_result.is_error,
            );
            thread.add_turn(tool_turn);
        }

        current_turn += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ProviderMessage;
    use async_trait::async_trait;

    /// Mock provider that returns predefined responses
    struct MockProvider {
        responses: Vec<ProviderResponse>,
        call_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl MockProvider {
        fn new(responses: Vec<ProviderResponse>) -> Self {
            Self {
                responses,
                call_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn execute(
            &self,
            _messages: Vec<ProviderMessage>,
            _tools: Vec<crate::tools::ToolSpec>,
        ) -> Result<ProviderResponse, AgentError> {
            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < self.responses.len() {
                Ok(self.responses[count].clone())
            } else {
                Ok(ProviderResponse::text("Default response".to_string()))
            }
        }
    }

    fn create_test_thread() -> Thread {
        let mut thread = Thread::new("test-session".to_string());
        thread.add_turn(Turn::new(TurnRole::User, "Hello".to_string()));
        thread
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.max_turns, 20);
        assert_eq!(config.turn_timeout_secs, 60);
    }

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::default()
            .with_max_turns(5)
            .with_timeout(30);
        assert_eq!(config.max_turns, 5);
        assert_eq!(config.turn_timeout_secs, 30);
    }

    #[test]
    fn test_error_strategy() {
        assert_eq!(ErrorStrategy::Retry(3), ErrorStrategy::Retry(3));
        assert_ne!(ErrorStrategy::Retry(3), ErrorStrategy::Skip);
    }

    #[test]
    fn test_provider_response_text() {
        let response = ProviderResponse::text("Hello".to_string());
        assert!(response.is_finished);
        assert!(response.tool_calls.is_empty());
    }

    #[test]
    fn test_provider_response_with_tools() {
        let response = ProviderResponse::with_tools(
            "".to_string(),
            vec![ToolCall::new(
                "call-1".to_string(),
                "test".to_string(),
                serde_json::json!({}),
            )],
        );
        assert!(!response.is_finished);
        assert_eq!(response.tool_calls.len(), 1);
    }

    #[tokio::test]
    async fn test_agent_loop_text_response() {
        let mut thread = create_test_thread();
        let provider = Arc::new(MockProvider::new(vec![ProviderResponse::text(
            "Hello there!".to_string(),
        )]));
        let tools = Arc::new(ToolRegistry::new());
        let hooks = Arc::new(HookSystem::new());
        let config = AgentConfig::default().with_max_turns(5);

        let result = agent_loop(&mut thread, provider, tools, hooks, config)
            .await
            .unwrap();

        assert!(result.completed_normally);
        assert_eq!(result.content(), "Hello there!");
        assert_eq!(result.total_turns, 1);
    }

    #[tokio::test]
    async fn test_agent_loop_max_turns() {
        let mut thread = create_test_thread();
        // Provider that always returns tool calls (never finishes)
        // Need enough responses to exceed max_turns
        let responses: Vec<ProviderResponse> = (0..10)
            .map(|_| {
                ProviderResponse::with_tools(
                    "".to_string(),
                    vec![ToolCall::new(
                        "call-1".to_string(),
                        "nonexistent".to_string(),
                        serde_json::json!({}),
                    )],
                )
            })
            .collect();
        let provider = Arc::new(MockProvider::new(responses));
        let tools = Arc::new(ToolRegistry::new());
        let hooks = Arc::new(HookSystem::new());
        let config = AgentConfig::default().with_max_turns(2);

        let result = agent_loop(&mut thread, provider, tools, hooks, config)
            .await
            .unwrap();

        assert!(!result.completed_normally, "should not complete normally");
        assert!(
            result.termination_reason.contains("Max turns"),
            "termination reason should mention Max turns, got: {}",
            result.termination_reason
        );
    }

    #[test]
    fn test_agent_result_content() {
        let turn = Turn::new(TurnRole::Assistant, "Test response".to_string());
        let result = AgentResult {
            final_turn: turn,
            total_turns: 1,
            tool_calls_executed: 0,
            completed_normally: true,
            termination_reason: "Test".to_string(),
        };
        assert_eq!(result.content(), "Test response");
    }
}
