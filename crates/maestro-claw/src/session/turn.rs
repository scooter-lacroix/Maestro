//! Turn model - Individual message exchange
//!
//! A Turn represents a single request/response cycle in a conversation,
//! supporting all roles (User, Assistant, System, Tool) and tool calls.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role of a turn in the conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnRole {
    /// User message
    User,
    /// Assistant/AI response
    Assistant,
    /// System prompt/instruction
    System,
    /// Tool execution result
    Tool,
}

impl Default for TurnRole {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for TurnRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnRole::User => write!(f, "user"),
            TurnRole::Assistant => write!(f, "assistant"),
            TurnRole::System => write!(f, "system"),
            TurnRole::Tool => write!(f, "tool"),
        }
    }
}

/// A tool call request from the assistant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to invoke
    pub name: String,
    /// Arguments to pass to the tool (JSON)
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(id: String, name: String, arguments: serde_json::Value) -> Self {
        Self { id, name, arguments }
    }
}

/// Result from a tool execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this result is for
    pub tool_call_id: String,
    /// Output content from the tool
    pub content: String,
    /// Whether the tool execution failed
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    /// Create a new tool result
    pub fn new(tool_call_id: String, content: String, is_error: bool) -> Self {
        Self { tool_call_id, content, is_error }
    }
}

/// A single turn in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Unique identifier for this turn
    pub id: String,
    /// Role of the message sender
    pub role: TurnRole,
    /// Content of the message
    pub content: String,
    /// Tool calls requested by the assistant (if any)
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Tool execution results (for Tool role)
    #[serde(default)]
    pub tool_results: Vec<ToolResult>,
    /// ID of the tool call this result is for (for Tool role)
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// When this turn was created
    pub timestamp: DateTime<Utc>,
}

impl Turn {
    /// Create a new turn with the given role and content
    pub fn new(role: TurnRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            tool_call_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a new turn with a specific ID
    pub fn with_id(id: String, role: TurnRole, content: String) -> Self {
        Self {
            id,
            role,
            content,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            tool_call_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Get the turn ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the turn role
    pub fn role(&self) -> &TurnRole {
        &self.role
    }

    /// Get the turn content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the tool calls
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Get the tool results
    pub fn tool_results(&self) -> &[ToolResult] {
        &self.tool_results
    }

    /// Get the tool call ID (for Tool role)
    pub fn tool_call_id(&self) -> Option<&str> {
        self.tool_call_id.as_deref()
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Add a tool call to this turn
    pub fn add_tool_call(&mut self, id: String, name: String, arguments: serde_json::Value) {
        self.tool_calls.push(ToolCall::new(id, name, arguments));
    }

    /// Add a tool result to this turn
    pub fn add_tool_result(&mut self, tool_call_id: String, content: String, is_error: bool) {
        self.tool_results.push(ToolResult::new(tool_call_id, content, is_error));
    }

    /// Set the tool call ID (for Tool role turns)
    pub fn set_tool_call_id(&mut self, id: String) {
        self.tool_call_id = Some(id);
    }
}

impl Default for Turn {
    fn default() -> Self {
        Self::new(TurnRole::User, String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_creation() {
        let turn = Turn::new(TurnRole::User, "Hello".to_string());
        assert_eq!(turn.role(), &TurnRole::User);
        assert_eq!(turn.content(), "Hello");
        assert!(turn.tool_calls().is_empty());
    }

    #[test]
    fn test_turn_role_serialization() {
        let json = serde_json::to_string(&TurnRole::User).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_tool_call_creation() {
        let tc = ToolCall::new(
            "call-1".to_string(),
            "shell".to_string(),
            serde_json::json!({"cmd": "ls"}),
        );
        assert_eq!(tc.id, "call-1");
        assert_eq!(tc.name, "shell");
    }

    #[test]
    fn test_tool_result_creation() {
        let tr = ToolResult::new("call-1".to_string(), "output".to_string(), false);
        assert_eq!(tr.tool_call_id, "call-1");
        assert!(!tr.is_error);
    }
}
