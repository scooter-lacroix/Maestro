//! Sub-Agent Delegation Tool (`spawn_agent`)
//!
//! This module implements sub-agent delegation following patterns from ZeroClaw and Moltis:
//! - `zeroclaw/src/tools/delegate.rs` - DelegateTool with depth limiting
//! - `moltis/crates/agents/src/runner.rs` - SubAgentStart/SubAgentEnd events
//!
//! Key features:
//! - Depth limiting prevents infinite delegation loops
//! - Timeout protection with configurable limits
//! - Provider isolation per sub-agent
//! - Context injection for passing information to sub-agents

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::traits::{Context, Message, Provider, Tool};

/// Configuration for a delegatable sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateAgentConfig {
    /// Provider name (e.g., "openai", "anthropic").
    pub provider: String,
    /// Model identifier (e.g., "gpt-4", "claude-3-opus").
    pub model: String,
    /// Optional system prompt override.
    pub system_prompt: Option<String>,
    /// Optional API key override.
    pub api_key: Option<String>,
    /// Temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Maximum recursion depth for delegation.
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    /// Maximum iterations for the agent loop.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_depth() -> u32 {
    3
}

fn default_max_iterations() -> u32 {
    10
}

impl Default for DelegateAgentConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: default_temperature(),
            max_depth: default_max_depth(),
            max_iterations: default_max_iterations(),
        }
    }
}

/// Result of a sub-agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    /// Whether the sub-agent completed successfully.
    pub success: bool,
    /// The final output from the sub-agent.
    pub output: String,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Number of tool calls made.
    pub tool_calls_made: usize,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Configuration for the delegate tool.
#[derive(Debug, Clone)]
pub struct DelegateConfig {
    /// Default timeout for sub-agent execution.
    pub timeout: Duration,
    /// Maximum depth for nested delegation.
    pub max_depth: u32,
    /// Whether to allow sub-agents to spawn more sub-agents.
    pub allow_nested_delegation: bool,
}

impl Default for DelegateConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            max_depth: 3,
            allow_nested_delegation: true,
        }
    }
}

/// Sub-Agent Delegation Tool
///
/// Implements the `spawn_agent` tool for delegating tasks to specialized sub-agents.
/// Based on ZeroClaw's `DelegateTool` pattern with:
/// - Depth limiting to prevent infinite delegation chains
/// - Timeout protection
/// - Provider isolation
pub struct DelegateTool {
    /// Available agent configurations by name.
    agents: Arc<HashMap<String, DelegateAgentConfig>>,
    /// Tool configuration.
    config: DelegateConfig,
    /// Current delegation depth.
    current_depth: u32,
    /// Optional fallback credential.
    fallback_credential: Option<String>,
}

impl DelegateTool {
    /// Create a new delegate tool with the given agent configurations.
    pub fn new(agents: HashMap<String, DelegateAgentConfig>, config: DelegateConfig) -> Self {
        Self {
            agents: Arc::new(agents),
            config,
            current_depth: 0,
            fallback_credential: None,
        }
    }

    /// Create a new delegate tool with default configuration.
    pub fn with_agents(agents: HashMap<String, DelegateAgentConfig>) -> Self {
        Self::new(agents, DelegateConfig::default())
    }

    /// Set the fallback credential for sub-agents without explicit API keys.
    pub fn with_fallback_credential(mut self, credential: String) -> Self {
        self.fallback_credential = Some(credential);
        self
    }

    /// Set the current delegation depth (used for nested delegation).
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.current_depth = depth;
        self
    }

    /// Check if delegation is allowed at the current depth.
    pub fn can_delegate(&self) -> bool {
        self.current_depth < self.config.max_depth
    }

    /// Get the agent configuration by name.
    pub fn get_agent(&self, name: &str) -> Option<&DelegateAgentConfig> {
        self.agents.get(name)
    }

    /// Execute a sub-agent with the given task.
    ///
    /// This is the core delegation logic following ZeroClaw patterns:
    /// 1. Validate depth limits
    /// 2. Get agent configuration
    /// 3. Build context with injected system prompt
    /// 4. Execute with timeout
    /// 5. Track iterations and tool calls
    pub async fn execute_agent(
        &self,
        agent_name: &str,
        task: &str,
        context: Option<&Context>,
        provider: Arc<dyn Provider>,
    ) -> anyhow::Result<SubAgentResult> {
        // Check depth limit
        if !self.can_delegate() {
            return Ok(SubAgentResult {
                success: false,
                output: String::new(),
                iterations: 0,
                tool_calls_made: 0,
                error: Some(format!(
                    "Maximum delegation depth ({}) exceeded",
                    self.config.max_depth
                )),
            });
        }

        // Get agent configuration
        let agent_config = self.agents.get(agent_name).ok_or_else(|| {
            anyhow::anyhow!("Unknown agent: {}", agent_name)
        })?;

        // Build context
        let mut exec_context = context.cloned().unwrap_or_default();

        // Inject system prompt if provided
        if let Some(ref system_prompt) = agent_config.system_prompt {
            exec_context.metadata.insert(
                "system_prompt".to_string(),
                system_prompt.clone(),
            );
        }

        // Add task as user message
        exec_context.messages.push(Message {
            role: "user".to_string(),
            content: task.to_string(),
            timestamp: chrono::Utc::now(),
        });

        // Execute with timeout
        let timeout = self.config.timeout;
        let result = tokio::time::timeout(
            timeout,
            self.run_agent_loop(&agent_config, exec_context, provider),
        )
        .await;

        match result {
            Ok(Ok(res)) => Ok(res),
            Ok(Err(e)) => Ok(SubAgentResult {
                success: false,
                output: String::new(),
                iterations: 0,
                tool_calls_made: 0,
                error: Some(e.to_string()),
            }),
            Err(_) => Ok(SubAgentResult {
                success: false,
                output: String::new(),
                iterations: 0,
                tool_calls_made: 0,
                error: Some(format!("Agent execution timed out after {:?}", timeout)),
            }),
        }
    }

    /// Run the agent loop with bounded iterations.
    async fn run_agent_loop(
        &self,
        config: &DelegateAgentConfig,
        mut context: Context,
        provider: Arc<dyn Provider>,
    ) -> anyhow::Result<SubAgentResult> {
        let mut iterations = 0;
        let tool_calls_made = 0; // Will be incremented when tool execution is implemented
        let max_iterations = config.max_iterations;

        while iterations < max_iterations {
            iterations += 1;

            // Generate response from provider
            let response = provider.generate(&context).await?;

            // Add assistant message to context
            context.messages.push(response.clone());

            // Check for completion (simple heuristic: no tool calls in response)
            // In a full implementation, this would check for tool call requests
            let content = response.content.to_lowercase();

            if content.contains("task complete")
                || content.contains("done")
                || content.contains("finished") {
                return Ok(SubAgentResult {
                    success: true,
                    output: response.content,
                    iterations: iterations as usize,
                    tool_calls_made,
                    error: None,
                });
            }
        }

        // Max iterations reached
        let last_message = context.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        Ok(SubAgentResult {
            success: true,
            output: last_message,
            iterations: iterations as usize,
            tool_calls_made,
            error: Some(format!("Max iterations ({}) reached", max_iterations)),
        })
    }
}

#[async_trait]
impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "spawn_agent"
    }

    fn description(&self) -> &str {
        "Spawn a specialized sub-agent to handle a task. \
         Use this for complex tasks that benefit from focused attention \
         or different expertise. Supports nested delegation with depth limits."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Name of the agent to spawn"
                },
                "task": {
                    "type": "string",
                    "description": "The task for the sub-agent to complete"
                },
                "context": {
                    "type": "object",
                    "description": "Optional additional context for the agent",
                    "additionalProperties": true
                }
            },
            "required": ["agent", "task"]
        })
    }

    async fn execute(&self, input: Value) -> anyhow::Result<Value> {
        // Parse input
        let agent_name = input["agent"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'agent' parameter"))?
            .to_string();

        let task = input["task"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task' parameter"))?
            .to_string();

        // Check depth limit
        if !self.can_delegate() {
            return Ok(serde_json::json!({
                "success": false,
                "error": format!("Maximum delegation depth ({}) exceeded", self.config.max_depth)
            }));
        }

        // Get agent config
        let agent_config = match self.agents.get(&agent_name) {
            Some(config) => config.clone(),
            None => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": format!("Unknown agent: {}", agent_name)
                }));
            }
        };

        // Return a result indicating the delegation request
        // The actual execution is handled by the engine with proper provider
        Ok(serde_json::json!({
            "success": true,
            "delegation_request": {
                "agent": agent_name,
                "task": task,
                "config": {
                    "provider": agent_config.provider,
                    "model": agent_config.model,
                    "max_iterations": agent_config.max_iterations
                },
                "depth": self.current_depth + 1
            }
        }))
    }
}

/// Builder for creating delegate tools.
pub struct DelegateToolBuilder {
    agents: HashMap<String, DelegateAgentConfig>,
    config: DelegateConfig,
    fallback_credential: Option<String>,
    depth: u32,
}

impl DelegateToolBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            config: DelegateConfig::default(),
            fallback_credential: None,
            depth: 0,
        }
    }

    /// Add an agent configuration.
    pub fn agent(mut self, name: impl Into<String>, config: DelegateAgentConfig) -> Self {
        self.agents.insert(name.into(), config);
        self
    }

    /// Set the timeout for sub-agent execution.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the maximum delegation depth.
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Set whether nested delegation is allowed.
    pub fn allow_nested_delegation(mut self, allow: bool) -> Self {
        self.config.allow_nested_delegation = allow;
        self
    }

    /// Set the fallback credential.
    pub fn fallback_credential(mut self, credential: impl Into<String>) -> Self {
        self.fallback_credential = Some(credential.into());
        self
    }

    /// Set the current depth.
    pub fn depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Build the delegate tool.
    pub fn build(self) -> DelegateTool {
        let mut tool = DelegateTool::new(self.agents, self.config);
        if let Some(cred) = self.fallback_credential {
            tool = tool.with_fallback_credential(cred);
        }
        tool = tool.with_depth(self.depth);
        tool
    }
}

impl Default for DelegateToolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delegate_agent_config_default() {
        let config = DelegateAgentConfig::default();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.max_iterations, 10);
    }

    #[test]
    fn test_delegate_config_default() {
        let config = DelegateConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.max_depth, 3);
        assert!(config.allow_nested_delegation);
    }

    #[test]
    fn test_delegate_tool_creation() {
        let mut agents = HashMap::new();
        agents.insert(
            "researcher".to_string(),
            DelegateAgentConfig {
                provider: "anthropic".to_string(),
                model: "claude-3-opus".to_string(),
                system_prompt: Some("You are a research assistant.".to_string()),
                ..Default::default()
            },
        );

        let tool = DelegateTool::with_agents(agents);
        assert_eq!(tool.name(), "spawn_agent");
        assert!(tool.can_delegate());
        assert!(tool.get_agent("researcher").is_some());
        assert!(tool.get_agent("unknown").is_none());
    }

    #[test]
    fn test_delegate_tool_depth_limit() {
        let tool = DelegateTool::with_agents(HashMap::new()).with_depth(3);
        assert!(!tool.can_delegate());

        let tool = DelegateTool::with_agents(HashMap::new()).with_depth(2);
        assert!(tool.can_delegate());
    }

    #[test]
    fn test_input_schema() {
        let tool = DelegateTool::with_agents(HashMap::new());
        let schema = tool.input_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["agent"].is_object());
        assert!(schema["properties"]["task"].is_object());
    }

    #[tokio::test]
    async fn test_execute_missing_agent() {
        let tool = DelegateTool::with_agents(HashMap::new());
        let result = tool.execute(serde_json::json!({
            "agent": "unknown",
            "task": "test"
        }))
        .await
        .unwrap();

        assert!(!result["success"].as_bool().unwrap());
        assert!(result["error"].as_str().unwrap().contains("Unknown agent"));
    }

    #[tokio::test]
    async fn test_execute_depth_exceeded() {
        let tool = DelegateTool::with_agents(HashMap::new()).with_depth(3);
        let result = tool.execute(serde_json::json!({
            "agent": "test",
            "task": "test"
        }))
        .await
        .unwrap();

        assert!(!result["success"].as_bool().unwrap());
        assert!(result["error"].as_str().unwrap().contains("depth"));
    }

    #[test]
    fn test_delegate_tool_builder() {
        let tool = DelegateToolBuilder::new()
            .agent(
                "researcher",
                DelegateAgentConfig {
                    provider: "anthropic".to_string(),
                    model: "claude-3-opus".to_string(),
                    ..Default::default()
                },
            )
            .agent(
                "coder",
                DelegateAgentConfig {
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    ..Default::default()
                },
            )
            .timeout(Duration::from_secs(60))
            .max_depth(5)
            .depth(1)
            .build();

        assert_eq!(tool.name(), "spawn_agent");
        assert!(tool.can_delegate());
        assert!(tool.get_agent("researcher").is_some());
        assert!(tool.get_agent("coder").is_some());
    }

    #[tokio::test]
    async fn test_execute_returns_delegation_request() {
        let mut agents = HashMap::new();
        agents.insert(
            "test_agent".to_string(),
            DelegateAgentConfig {
                provider: "test_provider".to_string(),
                model: "test_model".to_string(),
                max_iterations: 5,
                ..Default::default()
            },
        );

        let tool = DelegateTool::with_agents(agents);
        let result = tool.execute(serde_json::json!({
            "agent": "test_agent",
            "task": "Do something"
        }))
        .await
        .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["delegation_request"].is_object());
        assert_eq!(result["delegation_request"]["agent"], "test_agent");
        assert_eq!(result["delegation_request"]["task"], "Do something");
        assert_eq!(result["delegation_request"]["depth"], 1);
    }
}
