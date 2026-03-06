//! OMP Tool Provider
//!
//! Implements the ToolProvider trait for OMP tools in Maestro Cockpit.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use super::bridge::{
    OmpBridge, ALL_TOOLS, TOOL_EDIT, TOOL_FIND, TOOL_GREP, TOOL_PYTHON, TOOL_READ, TOOL_WRITE,
};

/// Tool definition for OMP tools
#[derive(Debug, Clone)]
pub struct OmpToolDefinition {
    /// Tool name (e.g., "python", "edit")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    pub input_schema: Value,
}

impl OmpToolDefinition {
    /// Create a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the input schema
    pub fn input_schema(&self) -> &Value {
        &self.input_schema
    }
}

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Output content (markdown)
    pub output: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            error: None,
        }
    }

    /// Create a failed result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            output: String::new(),
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Tool provider trait (simplified version for OMP)
#[async_trait]
pub trait ToolProvider: Send + Sync {
    /// Get list of available tools
    fn tools(&self) -> Vec<OmpToolDefinition>;

    /// Execute a tool
    async fn execute(&self, tool: &str, input: Value) -> Result<ToolResult>;

    /// Check if a tool is available
    fn has_tool(&self, name: &str) -> bool {
        self.tools().iter().any(|t| t.name == name)
    }

    /// Get tool definition by name
    fn get_tool(&self, name: &str) -> Option<OmpToolDefinition> {
        self.tools().into_iter().find(|t| t.name == name)
    }
}

/// OMP tool provider implementation
pub struct OmpToolProvider {
    /// Bridge to OMP worker
    bridge: Arc<OmpBridge>,
    /// Available tool definitions
    tools: Vec<OmpToolDefinition>,
}

impl OmpToolProvider {
    /// Create a new OMP tool provider
    pub fn new(bridge: Arc<OmpBridge>) -> Self {
        let tools = Self::build_tool_definitions();
        Self { bridge, tools }
    }

    /// Build tool definitions for OMP tools
    fn build_tool_definitions() -> Vec<OmpToolDefinition> {
        vec![
            OmpToolDefinition::new(
                TOOL_PYTHON,
                "Execute Python code in an IPython kernel with state persistence",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Python code to execute"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory (optional)"
                        }
                    },
                    "required": ["code"]
                }),
            ),
            OmpToolDefinition::new(
                TOOL_EDIT,
                "Apply a patch-based edit to a file (safer than sed)",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to edit"
                        },
                        "diff": {
                            "type": "string",
                            "description": "Unified diff to apply"
                        }
                    },
                    "required": ["path", "diff"]
                }),
            ),
            OmpToolDefinition::new(
                TOOL_GREP,
                "Search file contents using ripgrep (WASM-accelerated)",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory or file to search"
                        },
                        "glob": {
                            "type": "string",
                            "description": "Glob pattern to filter files"
                        },
                        "ignore_case": {
                            "type": "boolean",
                            "description": "Case-insensitive search"
                        }
                    },
                    "required": ["pattern", "path"]
                }),
            ),
            OmpToolDefinition::new(
                TOOL_FIND,
                "Find files by glob pattern (WASM-accelerated)",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern (e.g., **/*.rs)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Base directory to search"
                        }
                    },
                    "required": ["pattern", "path"]
                }),
            ),
            OmpToolDefinition::new(
                TOOL_READ,
                "Read file contents",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Start line (1-indexed)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum lines to read"
                        }
                    },
                    "required": ["path"]
                }),
            ),
            OmpToolDefinition::new(
                TOOL_WRITE,
                "Write content to a file",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write"
                        }
                    },
                    "required": ["path", "content"]
                }),
            ),
        ]
    }

    /// Get the underlying bridge
    pub fn bridge(&self) -> &OmpBridge {
        &self.bridge
    }

    /// Get list of tool names
    pub fn tool_names(&self) -> Vec<&str> {
        ALL_TOOLS.to_vec()
    }
}

#[async_trait]
impl ToolProvider for OmpToolProvider {
    fn tools(&self) -> Vec<OmpToolDefinition> {
        self.tools.clone()
    }

    async fn execute(&self, tool: &str, input: Value) -> Result<ToolResult> {
        let result = self.bridge.invoke(tool, input).await?;

        Ok(ToolResult {
            output: result.output,
            success: result.success,
            error: result.error,
        })
    }
}

/// Create a default OMP tool provider
pub fn create_omp_provider(bridge: Arc<OmpBridge>) -> Box<dyn ToolProvider> {
    Box::new(OmpToolProvider::new(bridge))
}

/// Create an OMP tool provider with default configuration
pub fn create_default_omp_provider() -> Result<Box<dyn ToolProvider>> {
    let config = super::worker::OmpWorkerConfig::default();
    let bridge = Arc::new(OmpBridge::new(config));
    Ok(create_omp_provider(bridge))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions() {
        let tools = OmpToolProvider::build_tool_definitions();
        assert_eq!(tools.len(), 6);
        assert!(tools.iter().any(|t| t.name == TOOL_PYTHON));
        assert!(tools.iter().any(|t| t.name == TOOL_EDIT));
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("output");
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult::failure("error message");
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
