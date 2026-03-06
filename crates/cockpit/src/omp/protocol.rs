//! OMP Protocol Definitions
//!
//! JSON-RPC style protocol for IPC communication between Maestro and OMP worker.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// OMP JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpRequest {
    /// Request ID for correlation
    pub id: u64,
    /// Method name (e.g., "invoke_tool", "get_status")
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

/// OMP JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpResponse {
    /// Request ID being responded to
    pub id: u64,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OmpError>,
}

/// OMP error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl OmpError {
    /// Create a new error with code and message
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a parse error
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::new(PARSE_ERROR, msg)
    }

    /// Create an invalid request error
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::new(INVALID_REQUEST, msg)
    }

    /// Create a method not found error
    pub fn method_not_found(method: &str) -> Self {
        Self::new(METHOD_NOT_FOUND, format!("Method not found: {}", method))
    }

    /// Create an invalid params error
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, msg)
    }

    /// Create an internal error
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, msg)
    }

    /// Create a tool not found error
    pub fn tool_not_found(tool: &str) -> Self {
        Self::new(TOOL_NOT_FOUND, format!("Tool not found: {}", tool))
    }

    /// Create a tool execution failed error
    pub fn tool_execution_failed(tool: &str, reason: &str) -> Self {
        Self::new(
            TOOL_EXECUTION_FAILED,
            format!("Tool '{}' failed: {}", tool, reason),
        )
    }

    /// Create a worker not ready error
    pub fn worker_not_ready() -> Self {
        Self::new(WORKER_NOT_READY, "Worker not ready")
    }

    /// Create a session expired error
    pub fn session_expired(session_id: &str) -> Self {
        Self::new(SESSION_EXPIRED, format!("Session expired: {}", session_id))
    }
}

impl fmt::Display for OmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for OmpError {}

/// Tool invocation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpToolResult {
    /// Tool name that was invoked
    pub tool: String,
    /// Output content (markdown)
    pub output: String,
    /// Whether the tool call succeeded
    pub success: bool,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Structured data returned by tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Worker initialization config (sent from Maestro to OMP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpWorkerInit {
    /// Session ID for this worker
    pub session_id: String,
    /// Project path (working directory)
    pub project_path: String,
    /// Model to use (e.g., "claude-3-5-sonnet")
    pub model: String,
    /// Enabled tools
    pub tools: Vec<String>,
    /// Pre-warmed LSP pool socket (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lsp_pool_socket: Option<String>,
    /// Pre-warmed MCP pool socket (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_pool_socket: Option<String>,
    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Worker status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpWorkerStatus {
    /// Whether worker is ready to accept requests
    pub ready: bool,
    /// Current model in use
    pub model: String,
    /// Session ID
    pub session_id: String,
    /// Number of pending requests
    pub pending_requests: usize,
    /// Memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<usize>,
    /// Uptime in seconds
    pub uptime_secs: u64,
}

impl OmpWorkerStatus {
    /// Check if worker is healthy
    pub fn is_healthy(&self) -> bool {
        self.ready && self.uptime_secs > 0
    }

    /// Create a default status for an uninitialized worker
    pub fn uninitialized() -> Self {
        Self {
            ready: false,
            model: String::new(),
            session_id: String::new(),
            pending_requests: 0,
            memory_usage: None,
            uptime_secs: 0,
        }
    }
}

/// Streaming event from OMP worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmpStreamEvent {
    /// Event type
    pub kind: OmpStreamEventKind,
    /// Event content
    pub content: String,
    /// Timestamp (Unix millis)
    pub ts: u64,
}

impl OmpStreamEvent {
    /// Create a new stream event
    pub fn new(kind: OmpStreamEventKind, content: impl Into<String>) -> Self {
        Self {
            kind,
            content: content.into(),
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    /// Create a tool start event
    pub fn tool_start(tool: &str) -> Self {
        Self::new(
            OmpStreamEventKind::ToolStart,
            format!("Starting tool: {}", tool),
        )
    }

    /// Create a tool output event
    pub fn tool_output(output: &str) -> Self {
        Self::new(OmpStreamEventKind::ToolOutput, output)
    }

    /// Create a tool complete event
    pub fn tool_complete(tool: &str, success: bool) -> Self {
        let status = if success { "completed" } else { "failed" };
        Self::new(
            OmpStreamEventKind::ToolComplete,
            format!("Tool {} {}", tool, status),
        )
    }

    /// Create a thinking event
    pub fn thinking(content: &str) -> Self {
        Self::new(OmpStreamEventKind::Thinking, content)
    }

    /// Create an error event
    pub fn error(message: &str) -> Self {
        Self::new(OmpStreamEventKind::Error, message)
    }

    /// Create a status event
    pub fn status(message: &str) -> Self {
        Self::new(OmpStreamEventKind::Status, message)
    }
}

/// Types of stream events
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OmpStreamEventKind {
    /// Tool invocation started
    ToolStart,
    /// Tool output chunk
    ToolOutput,
    /// Tool invocation completed
    ToolComplete,
    /// Agent thinking/reasoning
    Thinking,
    /// Error occurred
    Error,
    /// Status update
    Status,
}

impl fmt::Display for OmpStreamEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToolStart => write!(f, "tool_start"),
            Self::ToolOutput => write!(f, "tool_output"),
            Self::ToolComplete => write!(f, "tool_complete"),
            Self::Thinking => write!(f, "thinking"),
            Self::Error => write!(f, "error"),
            Self::Status => write!(f, "status"),
        }
    }
}

// Standard JSON-RPC error codes
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

// OMP-specific error codes (start at -32000)
pub const TOOL_NOT_FOUND: i32 = -32001;
pub const TOOL_EXECUTION_FAILED: i32 = -32002;
pub const WORKER_NOT_READY: i32 = -32003;
pub const SESSION_EXPIRED: i32 = -32004;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omp_error_codes() {
        let err = OmpError::tool_not_found("python");
        assert_eq!(err.code, TOOL_NOT_FOUND);
        assert!(err.message.contains("python"));
    }

    #[test]
    fn test_omp_error_display() {
        let err = OmpError::worker_not_ready();
        let displayed = format!("{}", err);
        assert!(displayed.contains("-32003"));
    }

    #[test]
    fn test_stream_event_creation() {
        let event = OmpStreamEvent::tool_start("edit");
        assert_eq!(event.kind, OmpStreamEventKind::ToolStart);
        assert!(event.content.contains("edit"));
        assert!(event.ts > 0);
    }

    #[test]
    fn test_worker_status() {
        let status = OmpWorkerStatus::uninitialized();
        assert!(!status.is_healthy());
        assert!(!status.ready);
    }
}
