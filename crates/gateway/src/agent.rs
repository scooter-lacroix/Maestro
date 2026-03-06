//! Agent types for Gateway API
//!
//! This module provides request and response types for agent execution
//! endpoints in the gateway, including session management and event streaming.

use maestro_core::{ApprovalDecision, AuthTokenType, OAuthConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::{TokenInfo, TokenType};

// ============================================================================
// Agent Execution Types
// ============================================================================

/// Request to execute an agent prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecuteRequest {
    /// Existing session ID (creates new session if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The prompt to send to the agent
    pub prompt: String,
    /// Provider to use (e.g., "openai", "anthropic")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model to use (e.g., "gpt-4", "claude-3")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Maximum turns before stopping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<usize>,
    /// Enable streaming response
    #[serde(default)]
    pub stream: bool,
}

impl AgentExecuteRequest {
    /// Create a new agent execute request
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            session_id: None,
            prompt: prompt.into(),
            provider: None,
            model: None,
            max_turns: None,
            stream: false,
        }
    }

    /// Set the session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the provider
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set max turns
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// Enable streaming
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

/// Response from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecuteResponse {
    /// Session ID for this conversation
    pub session_id: String,
    /// Thread ID for this execution
    pub thread_id: String,
    /// Final response content
    pub content: String,
    /// Number of turns used
    pub turns_used: usize,
    /// Number of tool calls made
    pub tool_calls: usize,
    /// Whether execution completed normally
    pub completed_normally: bool,
    /// Termination reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_reason: Option<String>,
}

impl AgentExecuteResponse {
    /// Create a new agent execute response
    pub fn new(
        session_id: impl Into<String>,
        thread_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            thread_id: thread_id.into(),
            content: content.into(),
            turns_used: 1,
            tool_calls: 0,
            completed_normally: true,
            termination_reason: None,
        }
    }
}

// ============================================================================
// Session Management Types
// ============================================================================

/// Request to create a new session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    /// Session metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Default provider for this session
    pub provider: String,
    /// Default model for this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Response from session creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateResponse {
    /// Created session ID
    pub session_id: String,
    /// Creation timestamp
    pub created_at: String,
}

/// Request to delete a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDeleteRequest {
    /// Session ID to delete
    pub session_id: String,
}

/// Response from session deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDeleteResponse {
    /// Whether deletion was successful
    pub deleted: bool,
}

/// Session information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub id: String,
    /// Number of threads
    pub thread_count: usize,
    /// Total turns across all threads
    pub turn_count: usize,
    /// Creation timestamp
    pub created_at: String,
    /// Session status
    pub status: String,
}

/// Response for session listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListResponse {
    /// List of sessions
    pub sessions: Vec<SessionInfo>,
    /// Total count
    pub total: usize,
}

impl SessionListResponse {
    /// Create an empty response
    pub fn empty() -> Self {
        Self {
            sessions: Vec::new(),
            total: 0,
        }
    }

    /// Create a response from sessions
    pub fn from_sessions(sessions: Vec<SessionInfo>) -> Self {
        let total = sessions.len();
        Self { sessions, total }
    }
}

// ============================================================================
// Approval and Tool Auth Types
// ============================================================================

/// Serializable approval decision used by gateway APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecisionValue {
    Approve,
    Reject,
    Always,
}

impl From<ApprovalDecisionValue> for ApprovalDecision {
    fn from(value: ApprovalDecisionValue) -> Self {
        match value {
            ApprovalDecisionValue::Approve => ApprovalDecision::Approve,
            ApprovalDecisionValue::Reject => ApprovalDecision::Reject,
            ApprovalDecisionValue::Always => ApprovalDecision::Always,
        }
    }
}

impl From<ApprovalDecision> for ApprovalDecisionValue {
    fn from(value: ApprovalDecision) -> Self {
        match value {
            ApprovalDecision::Approve => ApprovalDecisionValue::Approve,
            ApprovalDecision::Reject => ApprovalDecisionValue::Reject,
            ApprovalDecision::Always => ApprovalDecisionValue::Always,
        }
    }
}

/// Approval lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingApprovalStatus {
    Pending,
    Approved,
    Rejected,
    AlwaysApproved,
    Expired,
}

/// Pending or recently-resolved approval request metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub request_id: String,
    pub session_id: String,
    pub thread_id: String,
    pub tool_name: String,
    pub operation: String,
    pub details: serde_json::Value,
    pub created_at: String,
    pub status: PendingApprovalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<ApprovalDecisionValue>,
}

/// Request body for resolving a pending approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecisionRequest {
    pub decision: ApprovalDecisionValue,
}

/// Response body for a resolved approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecisionResponse {
    pub approval: PendingApproval,
}

/// Queue response for dashboard and WS approval listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalQueueResponse {
    pub pending: Vec<PendingApproval>,
    pub count: usize,
}

impl ApprovalQueueResponse {
    pub fn new(pending: Vec<PendingApproval>) -> Self {
        let count = pending.len();
        Self { pending, count }
    }
}

/// Serializable token/auth kind used by gateway APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayAuthTokenType {
    Bearer,
    ApiKey,
    Oauth,
}

impl GatewayAuthTokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GatewayAuthTokenType::Bearer => "bearer",
            GatewayAuthTokenType::ApiKey => "api_key",
            GatewayAuthTokenType::Oauth => "oauth",
        }
    }
}

impl From<GatewayAuthTokenType> for AuthTokenType {
    fn from(value: GatewayAuthTokenType) -> Self {
        match value {
            GatewayAuthTokenType::Bearer => AuthTokenType::Bearer,
            GatewayAuthTokenType::ApiKey => AuthTokenType::ApiKey,
            GatewayAuthTokenType::Oauth => AuthTokenType::OAuth,
        }
    }
}

impl From<AuthTokenType> for GatewayAuthTokenType {
    fn from(value: AuthTokenType) -> Self {
        match value {
            AuthTokenType::Bearer => GatewayAuthTokenType::Bearer,
            AuthTokenType::ApiKey => GatewayAuthTokenType::ApiKey,
            AuthTokenType::OAuth => GatewayAuthTokenType::Oauth,
        }
    }
}

/// Pending tool/MCP auth lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingAuthStatus {
    Pending,
    Submitted,
    Connected,
    Failed,
}

/// Pending tool/MCP auth request metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingToolAuth {
    pub request_id: String,
    pub server_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub token_type: GatewayAuthTokenType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthConfig>,
    pub created_at: String,
    pub status: PendingAuthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Queue response for MCP/tool auth requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingToolAuthResponse {
    pub pending: Vec<PendingToolAuth>,
    pub count: usize,
}

impl PendingToolAuthResponse {
    pub fn new(pending: Vec<PendingToolAuth>) -> Self {
        let count = pending.len();
        Self { pending, count }
    }
}

/// Request body for submitting a token for a pending auth request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuthSubmitRequest {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<GatewayAuthTokenType>,
}

/// Response body after accepting a tool/MCP auth token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuthSubmitResponse {
    pub auth: PendingToolAuth,
    pub connected: bool,
}

/// Agent/gateway status snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub status: String,
    pub sessions: usize,
    pub active_runs: usize,
    pub pending_approvals: usize,
    pub pending_auth: usize,
}

// ============================================================================
// Pairing and Token Types
// ============================================================================

/// Request to initiate a pairing challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInitiateRequest {
    /// Device name requesting pairing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    /// Scopes being requested (comma-separated or array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<serde_json::Value>,
}

impl PairingInitiateRequest {
    /// Parse scopes from the request into a HashSet.
    pub fn parse_scopes(&self) -> std::collections::HashSet<String> {
        let mut scopes = std::collections::HashSet::new();
        if let Some(ref scopes_val) = self.scopes {
            if let Some(arr) = scopes_val.as_array() {
                for scope in arr {
                    if let Some(s) = scope.as_str() {
                        scopes.insert(s.to_string());
                    }
                }
            } else if let Some(s) = scopes_val.as_str() {
                for part in s.split(',') {
                    let part = part.trim();
                    if !part.is_empty() {
                        scopes.insert(part.to_string());
                    }
                }
            }
        }
        // Default to all scopes if none specified
        if scopes.is_empty() {
            scopes.insert("sessions".to_string());
            scopes.insert("approvals".to_string());
            scopes.insert("tools".to_string());
            scopes.insert("cron".to_string());
            scopes.insert("system".to_string());
        }
        scopes
    }
}

/// Response to a pairing initiation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInitiateResponse {
    /// The 6-digit verification code
    pub code: String,
    /// When this code expires (RFC3339)
    pub expires_at: String,
    /// Challenge ID for internal tracking
    pub challenge_id: String,
    /// Message explaining next steps
    pub message: String,
}

/// Request to list issued tokens (admin only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenListRequest {
    /// Filter by token type (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<TokenType>,
}

/// Request to revoke a token by ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRevokeRequest {
    /// Token ID to revoke
    pub token_id: String,
}

/// Response for token listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenListResponse {
    pub tokens: Vec<TokenInfo>,
    pub total: usize,
}

impl TokenListResponse {
    pub fn new(tokens: Vec<TokenInfo>) -> Self {
        let total = tokens.len();
        Self { tokens, total }
    }
}

/// Response for token revocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRevokeResponse {
    pub revoked: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingVerifyRequest {
    /// The 6-digit verification code
    pub code: String,
    /// Optional token TTL in seconds (default: 3600)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
}

/// Response to a successful pairing verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingVerifyResponse {
    /// The issued access token
    pub access_token: String,
    /// Stable token identifier for later management actions
    pub token_id: String,
    /// Token type (always "issued" for pairing flow)
    pub token_type: String,
    /// Scopes granted
    pub scopes: Vec<String>,
    /// Device name
    pub device_name: Option<String>,
    /// When the token expires (RFC3339)
    pub expires_at: String,
    /// Session ID created for this pairing
    pub session_id: String,
}

/// Response for listing pending pairing challenges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingListResponse {
    pub pending: Vec<PendingPairingInfo>,
    pub count: usize,
}

impl PairingListResponse {
    pub fn new(pending: Vec<PendingPairingInfo>) -> Self {
        let count = pending.len();
        Self { pending, count }
    }
}

/// Info about a pending pairing (for admin listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPairingInfo {
    pub code: String,
    pub device_name: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: String,
    pub created_at: String,
}

/// Request to register or update an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRegisterRequest {
    /// Server name/identifier
    pub name: String,
    /// Server URL (for SSE/HTTP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Command to spawn (for stdio)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments for stdio command
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// Whether OAuth is required
    #[serde(default)]
    pub requires_auth: bool,
    /// OAuth configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_config: Option<OAuthConfig>,
}

/// Response after registering/updating an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRegisterResponse {
    pub name: String,
    pub registered: bool,
    pub updated: bool,
}

/// Response after removing an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRemoveResponse {
    pub name: String,
    pub removed: bool,
}

/// Response listing all managed MCP servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerListResponse {
    pub servers: Vec<McpServerInfo>,
    pub count: usize,
}

/// Info about a managed MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub url: Option<String>,
    pub command: Option<String>,
    pub requires_auth: bool,
    pub has_oauth: bool,
    pub state: String,
    pub connected: bool,
    pub has_auth_token: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_updated_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub tools_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_auth: Option<PendingToolAuth>,
}

// ============================================================================
// Event Streaming Types
// ============================================================================

/// Compact summary of a single tool call, carried by `AgentTurnEvent` (LOW-13).
///
/// Using a typed struct (rather than bare `Vec<String>`) lets clients correlate
/// turn events with `ToolExecutionEvent` entries via the `id` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    /// Tool call ID (matches `ToolExecutionEvent::tool_call_id`)
    pub id: String,
    /// Tool name (matches `ToolExecutionEvent::tool_name`)
    pub name: String,
}

impl ToolCallSummary {
    /// Convenience constructor
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Event for agent turn completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnEvent {
    /// Event type: "agent.turn"
    pub event_type: String,
    /// Session ID
    pub session_id: String,
    /// Thread ID
    pub thread_id: String,
    /// Turn index in thread
    pub turn_index: usize,
    /// Role (user, assistant, tool)
    pub role: String,
    /// Content preview (truncated)
    pub content_preview: String,
    /// Tool calls made in this turn — structured so clients can correlate with
    /// `ToolExecutionEvent` by matching on `id` (LOW-13)
    #[serde(default)]
    pub tool_calls: Vec<ToolCallSummary>,
}

impl AgentTurnEvent {
    /// Create a new turn event (tool_calls defaults to empty)
    pub fn new(
        session_id: impl Into<String>,
        thread_id: impl Into<String>,
        turn_index: usize,
        role: impl Into<String>,
        content_preview: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "agent.turn".to_string(),
            session_id: session_id.into(),
            thread_id: thread_id.into(),
            turn_index,
            role: role.into(),
            content_preview: content_preview.into(),
            tool_calls: Vec::new(),
        }
    }

    /// Attach structured tool call summaries to this turn event
    pub fn with_tool_calls(mut self, calls: Vec<ToolCallSummary>) -> Self {
        self.tool_calls = calls;
        self
    }
}

/// Event for agent status change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusEvent {
    /// Event type: "agent.status"
    pub event_type: String,
    /// Session ID
    pub session_id: String,
    /// Previous status
    pub old_status: String,
    /// New status
    pub new_status: String,
}

impl AgentStatusEvent {
    /// Create a new status event
    pub fn new(
        session_id: impl Into<String>,
        old_status: impl Into<String>,
        new_status: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "agent.status".to_string(),
            session_id: session_id.into(),
            old_status: old_status.into(),
            new_status: new_status.into(),
        }
    }
}

/// Event for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionEvent {
    /// Event type: "tool.execute"
    pub event_type: String,
    /// Session ID
    pub session_id: String,
    /// Tool name
    pub tool_name: String,
    /// Tool call ID
    pub tool_call_id: String,
    /// Execution status (started, completed, error)
    pub status: String,
    /// Preview of tool arguments or result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

impl ToolExecutionEvent {
    /// Create a new tool execution event
    pub fn new(
        session_id: impl Into<String>,
        tool_name: impl Into<String>,
        tool_call_id: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "tool.execute".to_string(),
            session_id: session_id.into(),
            tool_name: tool_name.into(),
            tool_call_id: tool_call_id.into(),
            status: status.into(),
            preview: None,
        }
    }

    /// Add a preview
    pub fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }
}

/// Streaming chunk for real-time responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    /// Session ID
    pub session_id: String,
    /// Thread ID
    pub thread_id: String,
    /// Content delta
    pub delta: String,
    /// Whether this is the final chunk
    pub is_finished: bool,
}

impl StreamingChunk {
    /// Create a content chunk
    pub fn content(
        session_id: impl Into<String>,
        thread_id: impl Into<String>,
        delta: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            thread_id: thread_id.into(),
            delta: delta.into(),
            is_finished: false,
        }
    }

    /// Create a finished chunk
    pub fn finished(session_id: impl Into<String>, thread_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            thread_id: thread_id.into(),
            delta: String::new(),
            is_finished: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_execute_request_builder() {
        let req = AgentExecuteRequest::new("Hello")
            .with_session("sess-1")
            .with_provider("openai")
            .with_model("gpt-4")
            .with_max_turns(10)
            .with_stream(true);

        assert_eq!(req.prompt, "Hello");
        assert_eq!(req.session_id, Some("sess-1".to_string()));
        assert_eq!(req.provider, Some("openai".to_string()));
        assert!(req.stream);
    }

    #[test]
    fn test_agent_execute_request_serialization() {
        let req = AgentExecuteRequest::new("Test prompt").with_stream(true);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["prompt"], "Test prompt");
        assert!(json["stream"].as_bool().unwrap());
    }

    #[test]
    fn test_session_list_response() {
        let sessions = vec![SessionInfo {
            id: "sess-1".into(),
            thread_count: 2,
            turn_count: 10,
            created_at: "2026-02-23T12:00:00Z".into(),
            status: "active".into(),
        }];
        let response = SessionListResponse::from_sessions(sessions);

        assert_eq!(response.total, 1);
        assert_eq!(response.sessions.len(), 1);
    }

    #[test]
    fn test_agent_turn_event() {
        let event = AgentTurnEvent::new("sess-1", "thread-1", 0, "user", "Hello");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event_type"], "agent.turn");
        assert_eq!(json["session_id"], "sess-1");
        assert_eq!(json["turn_index"], 0);
    }

    #[test]
    fn test_agent_status_event() {
        let event = AgentStatusEvent::new("sess-1", "idle", "running");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event_type"], "agent.status");
        assert_eq!(json["old_status"], "idle");
        assert_eq!(json["new_status"], "running");
    }

    #[test]
    fn test_tool_execution_event() {
        let event =
            ToolExecutionEvent::new("sess-1", "bash", "call-1", "started").with_preview("ls -la");

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event_type"], "tool.execute");
        assert_eq!(json["tool_name"], "bash");
        assert_eq!(json["preview"], "ls -la");
    }

    #[test]
    fn test_streaming_chunk_content() {
        let chunk = StreamingChunk::content("sess-1", "thread-1", "Hello");

        assert!(!chunk.is_finished);
        assert_eq!(chunk.delta, "Hello");
    }

    #[test]
    fn test_streaming_chunk_finished() {
        let chunk = StreamingChunk::finished("sess-1", "thread-1");

        assert!(chunk.is_finished);
        assert!(chunk.delta.is_empty());
    }

    #[test]
    fn approval_decision_round_trips() {
        let value = ApprovalDecisionValue::Always;
        let core: ApprovalDecision = value.into();
        assert_eq!(
            ApprovalDecisionValue::from(core),
            ApprovalDecisionValue::Always
        );
    }

    #[test]
    fn auth_token_type_round_trips() {
        let value = GatewayAuthTokenType::ApiKey;
        let core: AuthTokenType = value.into();
        assert_eq!(
            GatewayAuthTokenType::from(core),
            GatewayAuthTokenType::ApiKey
        );
    }

    #[test]
    fn approval_queue_response_counts_entries() {
        let pending = vec![PendingApproval {
            request_id: "req-1".into(),
            session_id: "sess-1".into(),
            thread_id: "thread-1".into(),
            tool_name: "shell".into(),
            operation: "shell_exec".into(),
            details: serde_json::json!({"command": "pwd"}),
            created_at: "2026-03-05T00:00:00Z".into(),
            status: PendingApprovalStatus::Pending,
            decision: None,
        }];

        let response = ApprovalQueueResponse::new(pending);
        assert_eq!(response.count, 1);
    }
}
