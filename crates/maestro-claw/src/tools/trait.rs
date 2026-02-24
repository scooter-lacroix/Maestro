//! Tool trait definition
//!
//! The Tool trait provides a standard interface for tools that can be
//! executed by the agent loop.

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use super::{ToolOutput, ToolSpec};

/// A tool that can be executed by the agent
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name (unique identifier)
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the JSON Schema for parameters
    fn parameters_schema(&self) -> JsonValue;

    /// Execute the tool with given arguments
    async fn execute(&self, arguments: JsonValue) -> ToolOutput;

    /// Convert to provider tool specification
    fn to_spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}
