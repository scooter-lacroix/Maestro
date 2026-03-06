//! Tool specification types
//!
//! These types define the tool specification format for providers
//! and the output from tool execution.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Provider-compatible tool specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolSpec {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: JsonValue,
}

impl ToolSpec {
    /// Create a new tool specification
    pub fn new(name: String, description: String, parameters: JsonValue) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

/// Result from tool execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolOutput {
    /// Output content
    pub content: String,
    /// Whether the execution failed
    #[serde(default)]
    pub is_error: bool,
}

impl ToolOutput {
    /// Create a successful output
    pub fn success(content: String) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    /// Create an error output
    pub fn error(content: String) -> Self {
        Self {
            content,
            is_error: true,
        }
    }

    /// Check if this is an error
    pub fn is_error(&self) -> bool {
        self.is_error
    }

    /// Get the content
    pub fn content(&self) -> &str {
        &self.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_spec_creation() {
        let spec = ToolSpec::new(
            "test".to_string(),
            "A test tool".to_string(),
            serde_json::json!({"type": "object"}),
        );
        assert_eq!(spec.name, "test");
        assert_eq!(spec.description, "A test tool");
    }

    #[test]
    fn test_tool_output_success() {
        let output = ToolOutput::success("result".to_string());
        assert_eq!(output.content(), "result");
        assert!(!output.is_error());
    }

    #[test]
    fn test_tool_output_error() {
        let output = ToolOutput::error("failed".to_string());
        assert_eq!(output.content(), "failed");
        assert!(output.is_error());
    }
}
