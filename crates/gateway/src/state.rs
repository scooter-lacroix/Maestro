//! Shared gateway state
//!
//! Contains all shared state for the gateway including connections, managers, and event bus.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maestro_core::{
    ApprovalDecision, ApprovalManager, AuthToken, CronJob, McpManager, SandboxManager,
    SecurityPolicy,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, watch};
use tracing::debug;

use crate::agent::{
    GatewayAuthTokenType, PendingApproval, PendingApprovalStatus, PendingAuthStatus,
    PendingToolAuth,
};
use crate::protocol::EventFrame;
use crate::rate_limit::{RateLimitConfig, SlidingWindowRateLimiter};
use crate::ws::MethodRegistry;

// ---------------------------------------------------------------------------
// Auth Token Types
// ---------------------------------------------------------------------------

/// Token type indicating whether it's the master key or an issued access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// The master agent_api_key from config (implies all scopes)
    Master,
    /// A short-lived issued token with specific scopes
    Issued,
}

/// Scopes that can be granted to issued tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenScope {
    /// Agent session management (execute, list, create, status)
    Sessions,
    /// Approval management (list, resolve)
    Approvals,
    /// MCP/tool authentication (list, submit)
    Tools,
    /// Cron management and cron event visibility
    Cron,
    /// System methods (methods/list, session/status)
    System,
    /// All scopes (only for master key or explicit full-access tokens)
    FullAccess,
}

impl TokenScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenScope::Sessions => "sessions",
            TokenScope::Approvals => "approvals",
            TokenScope::Tools => "tools",
            TokenScope::Cron => "cron",
            TokenScope::System => "system",
            TokenScope::FullAccess => "full_access",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sessions" => Some(TokenScope::Sessions),
            "approvals" => Some(TokenScope::Approvals),
            "tools" => Some(TokenScope::Tools),
            "cron" => Some(TokenScope::Cron),
            "system" => Some(TokenScope::System),
            "full_access" => Some(TokenScope::FullAccess),
            _ => None,
        }
    }
}

/// An issued access token with scopes and expiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedToken {
    /// The token value (randomly generated)
    pub token: String,
    /// Stable token identifier (UUID)
    pub token_id: String,
    /// Token type
    pub token_type: TokenType,
    /// Scopes granted to this token
    pub scopes: HashSet<String>,
    /// When this token expires (RFC3339)
    pub expires_at: String,
    /// Device name that requested this token (if applicable)
    pub device_name: Option<String>,
    /// When this token was issued (RFC3339)
    pub issued_at: String,
}

impl IssuedToken {
    /// Check if this token is expired.
    pub fn is_expired(&self) -> bool {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&self.expires_at) {
            chrono::Utc::now() > expires.with_timezone(&chrono::Utc)
        } else {
            true // Parse error means treat as expired
        }
    }

    /// Check if this token has a specific scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(scope) || self.scopes.contains("full_access")
    }

    /// Create a new issued token with the given scopes and TTL.
    pub fn new(scopes: HashSet<String>, ttl_seconds: u64, device_name: Option<String>) -> Self {
        let now = chrono::Utc::now();
        let expires_at = now + chrono::TimeDelta::seconds(ttl_seconds as i64);
        Self {
            token: format!("sk_{}", uuid::Uuid::new_v4()),
            token_id: uuid::Uuid::new_v4().to_string(),
            token_type: TokenType::Issued,
            scopes,
            expires_at: expires_at.to_rfc3339(),
            device_name,
            issued_at: now.to_rfc3339(),
        }
    }
}

/// A pending pairing challenge waiting for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPairing {
    /// The 6-digit verification code
    pub code: String,
    /// Device name requesting pairing
    pub device_name: Option<String>,
    /// Scopes being requested
    pub scopes: HashSet<String>,
    /// When this pairing challenge expires (RFC3339)
    pub expires_at: String,
    /// When this pairing was created (RFC3339)
    pub created_at: String,
    /// Challenge ID for internal tracking
    pub challenge_id: String,
}

impl PendingPairing {
    /// Default TTL for pairing challenges (5 minutes)
    pub const DEFAULT_TTL_SECS: u64 = 300;

    /// Create a new pending pairing challenge.
    pub fn new(device_name: Option<String>, scopes: HashSet<String>) -> Self {
        let now = chrono::Utc::now();
        let expires_at = now + chrono::TimeDelta::seconds(Self::DEFAULT_TTL_SECS as i64);
        // Generate 6-digit code
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
        Self {
            code,
            device_name,
            scopes,
            expires_at: expires_at.to_rfc3339(),
            created_at: now.to_rfc3339(),
            challenge_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Check if this pairing challenge is expired.
    pub fn is_expired(&self) -> bool {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&self.expires_at) {
            chrono::Utc::now() > expires.with_timezone(&chrono::Utc)
        } else {
            true
        }
    }
}

/// Authentication context for a validated request.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The type of token used
    pub token_type: TokenType,
    /// Scopes granted to the requester (empty for master key which implies all)
    pub scopes: HashSet<String>,
    /// The device name if this was an issued token
    pub device_name: Option<String>,
}

impl AuthContext {
    /// Create auth context for master key (implies all scopes).
    pub fn master() -> Self {
        Self {
            token_type: TokenType::Master,
            scopes: HashSet::new(), // Empty means all scopes
            device_name: None,
        }
    }

    /// Create auth context for an issued token.
    pub fn issued(scopes: HashSet<String>, device_name: Option<String>) -> Self {
        Self {
            token_type: TokenType::Issued,
            scopes,
            device_name,
        }
    }

    /// Check if this context has a specific scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        // Master key has all scopes
        if self.token_type == TokenType::Master {
            return true;
        }
        self.scopes.contains(scope) || self.scopes.contains(TokenScope::FullAccess.as_str())
    }

    /// Intersect requested scopes with authorized scopes.
    pub fn intersect_scopes(&self, requested: &HashSet<String>) -> HashSet<String> {
        // Master key gets all requested scopes
        if self.token_type == TokenType::Master {
            return requested.clone();
        }
        if self.scopes.contains(TokenScope::FullAccess.as_str()) {
            return requested.clone();
        }
        // Issued token only gets what they have
        requested.intersection(&self.scopes).cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Agent Session Store types (Rec-4)
// ---------------------------------------------------------------------------

/// An in-memory record of a stored agent session.
///
/// Holds the active thread plus the metadata required by the session
/// list/get endpoints.
pub struct StoredAgentSession {
    /// Metadata returned by `/api/agent/sessions` endpoints
    pub info: crate::agent::SessionInfo,
    /// Active thread for the session, preserving summary and tool-turn state
    pub thread: maestro_claw::Thread,
    /// Provider name originally used to create this session
    pub provider: String,
    /// Model originally used to create this session
    pub model: String,
    /// Most recent pending approval request tied to this session, if any
    pub pending_approval_id: Option<String>,
    /// Most recent pending tool/MCP auth request tied to this session, if any
    pub pending_auth_id: Option<String>,
}

/// Configuration for the gateway
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Maximum connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Event broadcast channel capacity
    pub event_channel_capacity: usize,
    /// WebSocket rate limit config
    pub ws_rate_limit: RateLimitConfig,
    /// Broadcast rate limit config (per sender)
    pub broadcast_rate_limit: RateLimitConfig,
    /// CORS allowed origins (empty = deny all, ["*"] = allow all)
    pub cors_allowed_origins: Vec<String>,
    /// CORS allowed methods
    pub cors_allowed_methods: Vec<String>,
    /// CORS allowed headers
    pub cors_allowed_headers: Vec<String>,
    /// Optional workspace path used for sandbox defaults and MCP persistence.
    pub workspace_path: Option<PathBuf>,

    // ---- Agent execution configuration (MED-7, MED-8) ----
    /// Bearer token required by `/api/agent/*` endpoints (MED-8).
    ///
    /// `None` disables authentication — suitable for local development only.
    pub agent_api_key: Option<String>,

    /// Default LLM provider name: "openai" | "anthropic" | "ollama" | "openrouter"
    pub default_llm_provider: String,

    /// OpenAI API key used when provider = "openai"
    pub openai_api_key: Option<String>,

    /// Anthropic API key used when provider = "anthropic"
    pub anthropic_api_key: Option<String>,

    /// Default model name; provider-specific default applies when `None`
    pub default_model: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 100,
            request_timeout_secs: 30,
            max_body_size: 64 * 1024, // 64KB
            event_channel_capacity: 256,
            ws_rate_limit: RateLimitConfig {
                limit: 60, // 60 messages per minute
                window: Duration::from_secs(60),
                include_retry_after: true,
            },
            broadcast_rate_limit: RateLimitConfig {
                limit: 10, // 10 broadcasts per minute
                window: Duration::from_secs(60),
                include_retry_after: true,
            },
            // SECURITY: Default to restrictive CORS (localhost only for development)
            cors_allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ],
            cors_allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "OPTIONS".to_string(),
            ],
            cors_allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            workspace_path: None,

            // Agent config — unauthenticated by default; callers must set keys to enable providers
            agent_api_key: None,
            default_llm_provider: "openai".to_string(),
            openai_api_key: None,
            anthropic_api_key: None,
            default_model: None,
        }
    }
}

/// Shared state for the gateway
pub struct GatewayState {
    /// Configuration
    pub config: GatewayConfig,
    /// When the gateway started
    pub start_time: Instant,
    /// Active connection count
    pub connection_count: AtomicUsize,
    /// Event sequence number
    pub event_seq: AtomicU64,
    /// Event broadcast channel
    pub event_bus: broadcast::Sender<EventFrame>,
    /// Number of active agent executions
    pub active_runs: AtomicUsize,
    /// Method registry for WebSocket RPC
    pub method_registry: MethodRegistry,
    /// MCP manager from maestro-core
    pub mcp_manager: McpManager,
    /// Sandbox manager from maestro-core
    pub sandbox_manager: SandboxManager,
    /// Cron jobs list (sync snapshot)
    pub cron_jobs: Vec<CronJob>,
    /// WebSocket rate limiter
    pub ws_rate_limiter: Arc<SlidingWindowRateLimiter>,
    /// Broadcast rate limiter (per-sender)
    pub broadcast_rate_limiter: Arc<SlidingWindowRateLimiter>,
    /// Broadcast tracking: last broadcast time per sender
    pub broadcast_tracking: Mutex<std::collections::HashMap<String, Instant>>,
    /// In-memory agent session store (Rec-4)
    ///
    /// Keyed by session ID.  DashMap provides lock-free concurrent reads and
    /// shard-locked concurrent writes so no extra `Mutex` is needed here.
    /// The `StoredAgentSession` value is only mutated briefly (to append
    /// turns after an `execute` completes) and never across an `.await`, so
    /// DashMap's shard locks are safe to use.
    pub session_store: dashmap::DashMap<String, StoredAgentSession>,
    /// Pending approval requests keyed by request ID.
    pub pending_approvals: dashmap::DashMap<String, PendingApproval>,
    /// Approval decision store for repeat "always" approvals on interactive sessions.
    pub approval_manager: ApprovalManager,
    /// Waiters for approval resolutions keyed by request ID.
    approval_waiters: dashmap::DashMap<String, watch::Sender<Option<ApprovalDecision>>>,
    /// Pending MCP/tool auth requests keyed by request ID.
    pub pending_tool_auth: dashmap::DashMap<String, PendingToolAuth>,
    /// Stored MCP/tool auth tokens keyed by server name.
    mcp_auth_tokens: dashmap::DashMap<String, AuthToken>,
    /// Issued access tokens keyed by token string.
    pub issued_tokens: dashmap::DashMap<String, IssuedToken>,
    /// Pending pairing challenges keyed by 6-digit code.
    pub pending_pairings: dashmap::DashMap<String, PendingPairing>,
}

impl GatewayState {
    /// Create a new gateway state with default configuration
    pub fn new() -> Self {
        Self::with_config(GatewayConfig::default())
    }

    /// Create a new gateway state with the given configuration
    pub fn with_config(config: GatewayConfig) -> Self {
        let (event_tx, _) = broadcast::channel(config.event_channel_capacity);

        let mut method_registry = MethodRegistry::new();
        for (method, handler) in crate::ws::builtin_handlers() {
            method_registry.register(method, handler);
        }

        let ws_rate_limit = config.ws_rate_limit.clone();
        let broadcast_rate_limit = config.broadcast_rate_limit.clone();
        let sandbox_policy = workspace_security_policy(config.workspace_path.as_deref());

        Self {
            config,
            start_time: Instant::now(),
            connection_count: AtomicUsize::new(0),
            event_seq: AtomicU64::new(0),
            event_bus: event_tx,
            active_runs: AtomicUsize::new(0),
            method_registry,
            mcp_manager: McpManager::new(),
            sandbox_manager: SandboxManager::new(sandbox_policy),
            cron_jobs: Vec::new(),
            ws_rate_limiter: Arc::new(SlidingWindowRateLimiter::new(ws_rate_limit)),
            broadcast_rate_limiter: Arc::new(SlidingWindowRateLimiter::new(broadcast_rate_limit)),
            broadcast_tracking: Mutex::new(std::collections::HashMap::new()),
            session_store: dashmap::DashMap::new(),
            pending_approvals: dashmap::DashMap::new(),
            approval_manager: ApprovalManager::new_empty(),
            approval_waiters: dashmap::DashMap::new(),
            pending_tool_auth: dashmap::DashMap::new(),
            mcp_auth_tokens: dashmap::DashMap::new(),
            issued_tokens: dashmap::DashMap::new(),
            pending_pairings: dashmap::DashMap::new(),
        }
    }

    /// Get the next event sequence number
    pub fn next_seq(&self) -> u64 {
        self.event_seq.fetch_add(1, Ordering::SeqCst)
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: &str, payload: serde_json::Value) {
        let frame = EventFrame::new(event, Some(payload), Some(self.next_seq()));
        let _ = self.event_bus.send(frame);
        debug!("Broadcast event: {}", event);
    }

    /// Increment connection count
    pub fn add_connection(&self) {
        self.connection_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement connection count
    pub fn remove_connection(&self) {
        self.connection_count.fetch_sub(1, Ordering::SeqCst);
    }

    /// Check if we can accept more connections
    pub fn can_accept_connection(&self) -> bool {
        self.connection_count.load(Ordering::SeqCst) < self.config.max_connections
    }

    /// Check if a broadcast is allowed from the given sender
    pub fn can_broadcast(&self, sender_id: &str) -> (bool, Option<u64>) {
        let (allowed, _remaining, retry_after) = self.broadcast_rate_limiter.check(sender_id);
        (allowed, retry_after)
    }

    /// Increment active agent execution count.
    pub fn add_active_run(&self) {
        self.active_runs.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement active agent execution count.
    pub fn remove_active_run(&self) {
        self.active_runs.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get the active agent execution count.
    pub fn active_run_count(&self) -> usize {
        self.active_runs.load(Ordering::SeqCst)
    }

    /// Set a session status string if the session exists.
    pub fn set_session_status(&self, session_id: &str, status: impl Into<String>) {
        if let Some(mut entry) = self.session_store.get_mut(session_id) {
            entry.value_mut().info.status = status.into();
        }
    }

    /// Garbage-collect stale sessions (TTL: 1 hour, max: 100)
    pub fn gc_sessions(&self) {
        let now = chrono::Utc::now();
        let ttl = chrono::TimeDelta::hours(1);

        // Remove sessions older than TTL
        self.session_store.retain(|_, session| {
            if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&session.info.created_at) {
                now.signed_duration_since(created) < ttl
            } else {
                true // Keep sessions with unparseable timestamps
            }
        });

        // LRU eviction: if still too many, remove oldest
        if self.session_store.len() > 100 {
            let mut entries: Vec<(String, String)> = self
                .session_store
                .iter()
                .map(|e| (e.key().clone(), e.value().info.created_at.clone()))
                .collect();
            entries.sort_by(|a, b| a.1.cmp(&b.1));
            let to_remove = entries.len() - 100;
            for (id, _) in entries.into_iter().take(to_remove) {
                self.session_store.remove(&id);
            }
        }
    }

    /// Enqueue a pending approval request and return the public metadata.
    pub fn enqueue_approval(
        &self,
        session_id: &str,
        thread_id: &str,
        tool_name: &str,
        operation: &str,
        details: serde_json::Value,
    ) -> PendingApproval {
        let request_id = uuid::Uuid::new_v4().to_string();
        let approval = PendingApproval {
            request_id: request_id.clone(),
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
            tool_name: tool_name.to_string(),
            operation: operation.to_string(),
            details: details.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            status: PendingApprovalStatus::Pending,
            decision: None,
        };
        let (tx, _) = watch::channel(None);
        self.pending_approvals
            .insert(request_id.clone(), approval.clone());
        self.approval_waiters.insert(request_id.clone(), tx);

        if let Some(mut entry) = self.session_store.get_mut(session_id) {
            entry.value_mut().pending_approval_id = Some(request_id.clone());
            entry.value_mut().info.status = "awaiting_approval".to_string();
        }

        self.broadcast(
            "exec.approval.requested",
            serde_json::json!({
                "request_id": approval.request_id,
                "session_id": approval.session_id,
                "thread_id": approval.thread_id,
                "tool_name": approval.tool_name,
                "operation": approval.operation,
                "details": approval.details,
            }),
        );

        approval
    }

    /// Subscribe to approval resolution for a specific request.
    pub fn subscribe_approval_resolution(
        &self,
        request_id: &str,
    ) -> Option<watch::Receiver<Option<ApprovalDecision>>> {
        self.approval_waiters
            .get(request_id)
            .map(|sender| sender.subscribe())
    }

    /// List currently pending approval requests.
    pub fn list_pending_approvals(&self) -> Vec<PendingApproval> {
        let mut pending: Vec<_> = self
            .pending_approvals
            .iter()
            .filter_map(|entry| {
                let approval = entry.value();
                (approval.status == PendingApprovalStatus::Pending).then(|| approval.clone())
            })
            .collect();
        pending.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        pending
    }

    /// Resolve a pending approval request.
    pub fn resolve_approval(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> Result<PendingApproval, String> {
        let mut entry = self
            .pending_approvals
            .get_mut(request_id)
            .ok_or_else(|| format!("Approval request not found: {request_id}"))?;

        entry.status = match decision {
            ApprovalDecision::Approve => PendingApprovalStatus::Approved,
            ApprovalDecision::Reject => PendingApprovalStatus::Rejected,
            ApprovalDecision::Always => PendingApprovalStatus::AlwaysApproved,
        };
        entry.decision = Some(decision.into());

        let approval = entry.value().clone();
        let session_id = approval.session_id.clone();
        drop(entry);

        if matches!(decision, ApprovalDecision::Always) {
            self.approval_manager.record_decision(
                approval.operation.clone(),
                maestro_core::ChannelType::Cli,
                decision,
            );
        }

        if let Some(sender) = self.approval_waiters.get(request_id) {
            let _ = sender.send(Some(decision));
        }
        self.approval_waiters.remove(request_id);

        if let Some(mut session) = self.session_store.get_mut(&session_id) {
            session.value_mut().pending_approval_id = None;
            session.value_mut().info.status = match decision {
                ApprovalDecision::Reject => "error".to_string(),
                ApprovalDecision::Approve | ApprovalDecision::Always => "active".to_string(),
            };
        }

        self.broadcast(
            "exec.approval.resolved",
            serde_json::json!({
                "request_id": approval.request_id,
                "session_id": approval.session_id,
                "thread_id": approval.thread_id,
                "tool_name": approval.tool_name,
                "status": approval.status,
                "decision": approval.decision,
            }),
        );

        Ok(approval)
    }

    /// Expire a pending approval request without an external decision.
    pub fn expire_approval(&self, request_id: &str) -> Option<PendingApproval> {
        let mut entry = self.pending_approvals.get_mut(request_id)?;
        entry.status = PendingApprovalStatus::Expired;
        entry.decision = None;
        let approval = entry.value().clone();
        let session_id = approval.session_id.clone();
        drop(entry);

        self.approval_waiters.remove(request_id);
        if let Some(mut session) = self.session_store.get_mut(&session_id) {
            session.value_mut().pending_approval_id = None;
            session.value_mut().info.status = "error".to_string();
        }

        Some(approval)
    }

    /// Count currently pending approval requests.
    pub fn pending_approval_count(&self) -> usize {
        self.list_pending_approvals().len()
    }

    /// Check whether a tool has a recorded "always approve" decision.
    pub fn should_auto_approve_tool(&self, tool_name: &str) -> bool {
        self.approval_manager
            .should_auto_approve(tool_name, maestro_core::ChannelType::Cli)
    }

    /// Enqueue an MCP/tool auth request, reusing an existing pending request for the same server.
    pub fn enqueue_tool_auth(
        &self,
        server_name: &str,
        session_id: Option<&str>,
        token_type: GatewayAuthTokenType,
        message: impl Into<String>,
        oauth: Option<maestro_core::OAuthConfig>,
    ) -> PendingToolAuth {
        if let Some(existing) = self.pending_tool_auth.iter().find_map(|entry| {
            let auth = entry.value();
            (auth.server_name == server_name
                && auth.session_id.as_deref() == session_id
                && matches!(
                    auth.status,
                    PendingAuthStatus::Pending
                        | PendingAuthStatus::Submitted
                        | PendingAuthStatus::Failed
                ))
            .then(|| auth.clone())
        }) {
            if let Some(session_id) = session_id {
                if let Some(mut session) = self.session_store.get_mut(session_id) {
                    session.value_mut().pending_auth_id = Some(existing.request_id.clone());
                    session.value_mut().info.status = "awaiting_auth".to_string();
                }
            }
            return existing;
        }

        let auth = PendingToolAuth {
            request_id: uuid::Uuid::new_v4().to_string(),
            server_name: server_name.to_string(),
            session_id: session_id.map(str::to_string),
            token_type,
            message: message.into(),
            oauth,
            created_at: chrono::Utc::now().to_rfc3339(),
            status: PendingAuthStatus::Pending,
            last_error: None,
        };
        self.pending_tool_auth
            .insert(auth.request_id.clone(), auth.clone());

        if let Some(session_id) = session_id {
            if let Some(mut session) = self.session_store.get_mut(session_id) {
                session.value_mut().pending_auth_id = Some(auth.request_id.clone());
                session.value_mut().info.status = "awaiting_auth".to_string();
            }
        }

        self.broadcast(
            "exec.auth.requested",
            serde_json::json!({
                "request_id": auth.request_id,
                "server_name": auth.server_name,
                "session_id": auth.session_id,
                "token_type": auth.token_type.as_str(),
                "message": auth.message,
                "oauth": auth.oauth,
            }),
        );
        auth
    }

    /// List pending tool/MCP auth requests.
    pub fn list_pending_tool_auth(&self) -> Vec<PendingToolAuth> {
        let mut pending: Vec<_> = self
            .pending_tool_auth
            .iter()
            .filter_map(|entry| {
                let auth = entry.value();
                matches!(
                    auth.status,
                    PendingAuthStatus::Pending
                        | PendingAuthStatus::Submitted
                        | PendingAuthStatus::Failed
                )
                .then(|| auth.clone())
            })
            .collect();
        pending.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        pending
    }

    /// Store a submitted MCP/tool auth token for a pending auth request.
    pub fn submit_tool_auth_token(
        &self,
        request_id: &str,
        token: AuthToken,
    ) -> Result<PendingToolAuth, String> {
        let mut entry = self
            .pending_tool_auth
            .get_mut(request_id)
            .ok_or_else(|| format!("Tool auth request not found: {request_id}"))?;
        if matches!(entry.status, PendingAuthStatus::Connected) {
            return Err(format!(
                "Tool auth request is already connected: {request_id}"
            ));
        }
        entry.status = PendingAuthStatus::Submitted;
        entry.last_error = None;
        self.mcp_auth_tokens
            .insert(entry.server_name.clone(), token);
        let auth = entry.value().clone();
        drop(entry);

        self.broadcast(
            "exec.auth.resolved",
            serde_json::json!({
                "request_id": auth.request_id,
                "server_name": auth.server_name,
                "session_id": auth.session_id,
                "status": auth.status,
                "token_type": auth.token_type.as_str(),
            }),
        );
        Ok(auth)
    }

    /// Mark an MCP/tool auth request as connected after a successful server connection.
    pub fn mark_tool_auth_connected(&self, request_id: &str) -> Result<PendingToolAuth, String> {
        let mut entry = self
            .pending_tool_auth
            .get_mut(request_id)
            .ok_or_else(|| format!("Tool auth request not found: {request_id}"))?;
        entry.status = PendingAuthStatus::Connected;
        entry.last_error = None;
        let auth = entry.value().clone();
        drop(entry);

        if let Some(session_id) = auth.session_id.as_deref() {
            if let Some(mut session) = self.session_store.get_mut(session_id) {
                session.value_mut().pending_auth_id = None;
                session.value_mut().info.status = "active".to_string();
            }
        }

        self.broadcast(
            "exec.auth.resolved",
            serde_json::json!({
                "request_id": auth.request_id,
                "server_name": auth.server_name,
                "session_id": auth.session_id,
                "status": auth.status,
                "token_type": auth.token_type.as_str(),
            }),
        );
        Ok(auth)
    }

    /// Mark all unresolved auth requests for a server as connected.
    pub fn mark_tool_auth_connected_for_server(
        &self,
        server_name: &str,
    ) -> Result<Vec<PendingToolAuth>, String> {
        let request_ids: Vec<_> = self
            .pending_tool_auth
            .iter()
            .filter_map(|entry| {
                let auth = entry.value();
                (auth.server_name == server_name
                    && matches!(
                        auth.status,
                        PendingAuthStatus::Pending
                            | PendingAuthStatus::Submitted
                            | PendingAuthStatus::Failed
                    ))
                .then(|| auth.request_id.clone())
            })
            .collect();

        if request_ids.is_empty() {
            return Err(format!(
                "Tool auth request not found for server: {server_name}"
            ));
        }

        let mut resolved = Vec::with_capacity(request_ids.len());
        for request_id in request_ids {
            resolved.push(self.mark_tool_auth_connected(&request_id)?);
        }
        Ok(resolved)
    }

    /// Mark an MCP/tool auth request as failed but keep it retryable.
    pub fn mark_tool_auth_failed(
        &self,
        request_id: &str,
        error: impl Into<String>,
    ) -> Result<PendingToolAuth, String> {
        let error = error.into();
        let mut entry = self
            .pending_tool_auth
            .get_mut(request_id)
            .ok_or_else(|| format!("Tool auth request not found: {request_id}"))?;
        entry.status = PendingAuthStatus::Failed;
        entry.last_error = Some(error.clone());
        let auth = entry.value().clone();
        drop(entry);

        if let Some(session_id) = auth.session_id.as_deref() {
            if let Some(mut session) = self.session_store.get_mut(session_id) {
                session.value_mut().pending_auth_id = Some(auth.request_id.clone());
                session.value_mut().info.status = "awaiting_auth".to_string();
            }
        }

        self.broadcast(
            "exec.auth.failed",
            serde_json::json!({
                "request_id": auth.request_id,
                "server_name": auth.server_name,
                "session_id": auth.session_id,
                "status": auth.status,
                "token_type": auth.token_type.as_str(),
                "error": error,
            }),
        );

        Ok(auth)
    }

    /// Mark all unresolved auth requests for a server as failed.
    pub fn mark_tool_auth_failed_for_server(
        &self,
        server_name: &str,
        error: impl Into<String>,
    ) -> Result<Vec<PendingToolAuth>, String> {
        let error = error.into();
        let request_ids: Vec<_> = self
            .pending_tool_auth
            .iter()
            .filter_map(|entry| {
                let auth = entry.value();
                (auth.server_name == server_name
                    && matches!(
                        auth.status,
                        PendingAuthStatus::Pending
                            | PendingAuthStatus::Submitted
                            | PendingAuthStatus::Failed
                    ))
                .then(|| auth.request_id.clone())
            })
            .collect();

        if request_ids.is_empty() {
            return Err(format!(
                "Tool auth request not found for server: {server_name}"
            ));
        }

        let mut failed = Vec::with_capacity(request_ids.len());
        for request_id in request_ids {
            failed.push(self.mark_tool_auth_failed(&request_id, error.clone())?);
        }
        Ok(failed)
    }

    /// Get the stored auth token for a server, if one was submitted.
    pub fn auth_token_for_server(&self, server_name: &str) -> Option<AuthToken> {
        self.mcp_auth_tokens
            .get(server_name)
            .map(|entry| entry.value().clone())
    }

    /// Store an MCP auth token without creating or mutating pending auth requests.
    pub fn store_auth_token_for_server(&self, server_name: impl Into<String>, token: AuthToken) {
        self.mcp_auth_tokens.insert(server_name.into(), token);
    }

    /// Clear all in-memory MCP auth tokens.
    pub fn clear_all_auth_tokens(&self) {
        self.mcp_auth_tokens.clear();
    }

    /// Clear stored tool auth state for a removed server.
    pub fn clear_tool_auth_state(&self, server_name: &str) {
        self.mcp_auth_tokens.remove(server_name);
        self.pending_tool_auth.retain(|_, auth| {
            if auth.server_name != server_name {
                return true;
            }
            if let Some(session_id) = auth.session_id.as_deref() {
                if let Some(mut session) = self.session_store.get_mut(session_id) {
                    let session = session.value_mut();
                    if session.pending_auth_id.as_deref() == Some(auth.request_id.as_str()) {
                        session.pending_auth_id = None;
                        session.info.status = "idle".to_string();
                    }
                }
            }
            false
        });
    }

    /// Remove a stored auth token for a server.
    pub fn clear_auth_token_for_server(&self, server_name: &str) {
        self.mcp_auth_tokens.remove(server_name);
        self.pending_tool_auth
            .retain(|_, auth| auth.server_name != server_name);
    }

    /// Remove a session and clean up any linked approval or auth state.
    pub fn remove_session(&self, session_id: &str) -> bool {
        let removed = self.session_store.remove(session_id);
        if removed.is_none() {
            return false;
        }

        let approval_ids: Vec<_> = self
            .pending_approvals
            .iter()
            .filter_map(|entry| {
                (entry.value().session_id == session_id).then(|| entry.key().clone())
            })
            .collect();
        for approval_id in approval_ids {
            self.pending_approvals.remove(&approval_id);
            self.approval_waiters.remove(&approval_id);
        }

        self.pending_tool_auth
            .retain(|_, auth| auth.session_id.as_deref() != Some(session_id));

        true
    }

    /// Count currently pending tool/MCP auth requests.
    pub fn pending_tool_auth_count(&self) -> usize {
        self.list_pending_tool_auth().len()
    }

    /// Check whether a tool has previously been granted an "always approve" decision.
    pub fn is_tool_always_approved(&self, tool_name: &str) -> bool {
        self.pending_approvals.iter().any(|entry| {
            let approval = entry.value();
            approval.tool_name == tool_name
                && approval.status == PendingApprovalStatus::AlwaysApproved
        })
    }

    /// Record a broadcast event for rate limiting
    pub fn record_broadcast(&self, sender_id: &str) {
        let mut tracking = self.broadcast_tracking.lock();
        tracking.insert(sender_id.to_string(), Instant::now());
    }

    // -----------------------------------------------------------------------
    // Token Management
    // -----------------------------------------------------------------------

    /// Issue a new short-lived access token with the given scopes.
    pub fn issue_token(
        &self,
        scopes: HashSet<String>,
        ttl_seconds: u64,
        device_name: Option<String>,
    ) -> IssuedToken {
        let token = IssuedToken::new(scopes, ttl_seconds, device_name);
        self.issued_tokens
            .insert(token.token.clone(), token.clone());
        token
    }

    /// List issued tokens with metadata (no raw values)
    pub fn list_tokens(&self) -> Vec<TokenInfo> {
        let mut tokens: Vec<_> = self
            .issued_tokens
            .iter()
            .map(|token| TokenInfo {
                token_id: token.token_id.clone(),
                token_type: token.token_type.clone(),
                scopes: token.scopes.clone(),
                expires_at: token.expires_at.clone(),
                device_name: token.device_name.clone(),
                issued_at: token.issued_at.clone(),
                is_expired: token.is_expired(),
            })
            .collect();
        tokens.sort_by(|a, b| a.issued_at.cmp(&b.issued_at));
        tokens
    }

    /// Revoke a token by its stable identifier
    pub fn revoke_token_by_id(&self, token_id: &str) -> bool {
        // Find the token by ID and remove it
        let mut found = false;
        let mut to_remove = None;

        for token in self.issued_tokens.iter() {
            if token.token_id == token_id {
                to_remove = Some(token.key().clone());
                found = true;
                break;
            }
        }

        if let Some(token_value) = to_remove {
            self.issued_tokens.remove(&token_value);
        }

        found
    }

    /// Get token information by stable identifier (no raw value)
    pub fn get_token_info(&self, token_id: &str) -> Option<TokenInfo> {
        for token in self.issued_tokens.iter() {
            if token.token_id == token_id {
                return Some(TokenInfo {
                    token_id: token.token_id.clone(),
                    token_type: token.token_type.clone(),
                    scopes: token.scopes.clone(),
                    expires_at: token.expires_at.clone(),
                    device_name: token.device_name.clone(),
                    issued_at: token.issued_at.clone(),
                    is_expired: token.is_expired(),
                });
            }
        }
        None
    }

    /// Validate a token and return the auth context if valid.
    ///
    /// Returns None if the token is invalid or expired.
    pub fn validate_token(&self, token_str: &str) -> Option<AuthContext> {
        // Check if it's the master key
        if let Some(ref master_key) = self.config.agent_api_key {
            if token_str == master_key {
                return Some(AuthContext::master());
            }
        }

        // Check if it's an issued token
        if let Some(issued) = self.issued_tokens.get(token_str) {
            let is_expired = issued.value().is_expired();
            let scopes = issued.value().scopes.clone();
            let device_name = issued.value().device_name.clone();
            drop(issued);

            if is_expired {
                // Clean up expired token after releasing the DashMap read guard.
                self.issued_tokens.remove(token_str);
                return None;
            }

            return Some(AuthContext::issued(scopes, device_name));
        }

        None
    }

    /// Revoke an issued token.
    pub fn revoke_token(&self, token_str: &str) -> bool {
        self.issued_tokens.remove(token_str).is_some()
    }

    /// Clean up expired issued tokens.
    pub fn gc_expired_tokens(&self) {
        self.issued_tokens.retain(|_, token| !token.is_expired());
    }

    // -----------------------------------------------------------------------
    // Pairing Management
    // -----------------------------------------------------------------------

    /// Create a new pending pairing challenge.
    pub fn create_pairing(
        &self,
        device_name: Option<String>,
        scopes: HashSet<String>,
    ) -> PendingPairing {
        self.gc_expired_pairings();
        let pairing = PendingPairing::new(device_name, scopes);
        self.pending_pairings
            .insert(pairing.code.clone(), pairing.clone());
        pairing
    }

    /// Verify a pairing code and issue a token if valid.
    ///
    /// Returns None if the code is not found or expired.
    pub fn verify_pairing(&self, code: &str, ttl_seconds: u64) -> Option<IssuedToken> {
        if let Some(pairing) = self.pending_pairings.get(code) {
            let is_expired = pairing.value().is_expired();
            let scopes = pairing.value().scopes.clone();
            let device_name = pairing.value().device_name.clone();
            drop(pairing);

            if is_expired {
                self.pending_pairings.remove(code);
                return None;
            }

            // Remove the used pairing after releasing the DashMap read guard.
            self.pending_pairings.remove(code);
            // Issue the token
            Some(self.issue_token(scopes, ttl_seconds, device_name))
        } else {
            None
        }
    }

    /// Clean up expired pairing challenges.
    pub fn gc_expired_pairings(&self) {
        self.pending_pairings
            .retain(|_, pairing| !pairing.is_expired());
    }

    /// Get list of pending pairings (for admin/debugging).
    pub fn list_pending_pairings(&self) -> Vec<PendingPairing> {
        let mut pending: Vec<_> = self
            .pending_pairings
            .iter()
            .filter(|entry| !entry.value().is_expired())
            .map(|entry| entry.value().clone())
            .collect();
        pending.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        pending
    }
}

fn workspace_security_policy(workspace_path: Option<&Path>) -> SecurityPolicy {
    let mut policy = SecurityPolicy::default();
    if let Some(workspace_path) = workspace_path {
        let workspace_path = workspace_path.to_path_buf();
        policy.allowed_read_paths = vec![workspace_path.clone()];
        policy.allowed_write_paths = vec![workspace_path];
    }
    policy
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

/// Public token information (no sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub token_id: String,
    pub token_type: TokenType,
    pub scopes: HashSet<String>,
    pub expires_at: String,
    pub device_name: Option<String>,
    pub issued_at: String,
    pub is_expired: bool,
}

/// Broadcast options for event emission.
#[derive(Debug, Clone)]
pub struct BroadcastOpts {
    /// Event scope (who should receive it)
    pub scope: Option<String>,
    /// Whether to include sequence number
    pub include_seq: bool,
}

impl Default for BroadcastOpts {
    fn default() -> Self {
        Self {
            scope: None,
            include_seq: true,
        }
    }
}

/// Event scopes for access control
pub mod scopes {
    /// Approval events
    pub const APPROVALS: &str = "approvals";
    /// Session events
    pub const SESSIONS: &str = "sessions";
    /// Tool execution events
    pub const TOOLS: &str = "tools";
    /// System events
    pub const SYSTEM: &str = "system";
    /// Cron job events
    pub const CRON: &str = "cron";
}

/// Scope guards for event routing
pub fn event_scope_guards() -> std::collections::HashMap<&'static str, &'static [&'static str]> {
    let mut m = std::collections::HashMap::new();
    m.insert(
        "exec.approval.requested",
        &[scopes::APPROVALS, scopes::SESSIONS] as &[&str],
    );
    m.insert(
        "exec.approval.resolved",
        &[scopes::APPROVALS, scopes::SESSIONS] as &[&str],
    );
    m.insert(
        "exec.auth.requested",
        &[scopes::TOOLS, scopes::SESSIONS] as &[&str],
    );
    m.insert(
        "exec.auth.resolved",
        &[scopes::TOOLS, scopes::SESSIONS] as &[&str],
    );
    m.insert(
        "exec.auth.failed",
        &[scopes::TOOLS, scopes::SESSIONS] as &[&str],
    );
    m.insert(
        "agent.execute.started",
        &[scopes::SESSIONS, scopes::SYSTEM] as &[&str],
    );
    m.insert(
        "agent.execute.completed",
        &[scopes::SESSIONS, scopes::SYSTEM] as &[&str],
    );
    m.insert("cron.job.started", &[scopes::CRON] as &[&str]);
    m.insert("cron.job.completed", &[scopes::CRON] as &[&str]);
    m.insert(
        "tool.call.started",
        &[scopes::TOOLS, scopes::SESSIONS] as &[&str],
    );
    m.insert(
        "tool.call.completed",
        &[scopes::TOOLS, scopes::SESSIONS] as &[&str],
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use maestro_core::{AuthTokenType, OAuthConfig};

    #[test]
    fn test_gateway_state_creation() {
        let state = GatewayState::new();
        assert_eq!(state.connection_count.load(Ordering::SeqCst), 0);
        assert!(state.method_registry.get("ping").is_some());
    }

    #[test]
    fn test_next_seq() {
        let state = GatewayState::new();
        assert_eq!(state.next_seq(), 0);
        assert_eq!(state.next_seq(), 1);
        assert_eq!(state.next_seq(), 2);
    }

    #[test]
    fn test_connection_tracking() {
        let state = GatewayState::new();
        state.add_connection();
        state.add_connection();
        assert_eq!(state.connection_count.load(Ordering::SeqCst), 2);
        assert!(state.can_accept_connection());
        state.remove_connection();
        assert_eq!(state.connection_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_enqueue_and_resolve_approval() {
        let state = GatewayState::new();
        let session_id = "sess-1".to_string();
        state.session_store.insert(
            session_id.clone(),
            StoredAgentSession {
                info: crate::agent::SessionInfo {
                    id: session_id.clone(),
                    thread_count: 1,
                    turn_count: 0,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    status: "idle".into(),
                },
                thread: maestro_claw::Thread::new(session_id.clone()),
                provider: "openai".into(),
                model: "gpt-4o".into(),
                pending_approval_id: None,
                pending_auth_id: None,
            },
        );

        let approval = state.enqueue_approval(
            &session_id,
            "thread-1",
            "shell",
            "shell_exec",
            serde_json::json!({"command": "pwd"}),
        );
        let mut rx = state
            .subscribe_approval_resolution(&approval.request_id)
            .expect("missing approval waiter");

        assert_eq!(state.pending_approval_count(), 1);
        assert_eq!(
            state.session_store.get(&session_id).unwrap().info.status,
            "awaiting_approval"
        );

        let resolved = state
            .resolve_approval(&approval.request_id, ApprovalDecision::Approve)
            .expect("approval should resolve");
        assert_eq!(resolved.status, PendingApprovalStatus::Approved);
        assert_eq!(state.pending_approval_count(), 0);

        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), Some(ApprovalDecision::Approve));
        assert_eq!(
            state.session_store.get(&session_id).unwrap().info.status,
            "active"
        );
    }

    #[test]
    fn test_enqueue_and_submit_tool_auth() {
        let state = GatewayState::new();
        let auth = state.enqueue_tool_auth(
            "github",
            None,
            GatewayAuthTokenType::Bearer,
            "Authenticate GitHub MCP",
            Some(OAuthConfig {
                auth_url: "https://example.com/auth".into(),
                token_url: "https://example.com/token".into(),
                client_id: "client-id".into(),
                client_secret: None,
                redirect_url: Some("http://localhost/callback".into()),
                scopes: vec!["repo".into()],
            }),
        );

        assert_eq!(state.pending_tool_auth_count(), 1);
        let updated = state
            .submit_tool_auth_token(
                &auth.request_id,
                maestro_core::AuthToken::new("secret-token", AuthTokenType::Bearer),
            )
            .expect("tool auth should resolve");
        assert_eq!(updated.status, PendingAuthStatus::Submitted);
        assert_eq!(
            state.auth_token_for_server("github").unwrap().value(),
            "secret-token"
        );

        let connected = state
            .mark_tool_auth_connected(&auth.request_id)
            .expect("tool auth should mark connected");
        assert_eq!(connected.status, PendingAuthStatus::Connected);
        assert_eq!(state.pending_tool_auth_count(), 0);
    }

    #[test]
    fn test_tool_auth_failure_retry_and_session_linkage() {
        let state = GatewayState::new();
        let session_id = "sess-auth".to_string();
        state.session_store.insert(
            session_id.clone(),
            StoredAgentSession {
                info: crate::agent::SessionInfo {
                    id: session_id.clone(),
                    thread_count: 1,
                    turn_count: 0,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    status: "idle".into(),
                },
                thread: maestro_claw::Thread::new(session_id.clone()),
                provider: "openai".into(),
                model: "gpt-4o".into(),
                pending_approval_id: None,
                pending_auth_id: None,
            },
        );

        let auth = state.enqueue_tool_auth(
            "github",
            Some(&session_id),
            GatewayAuthTokenType::Bearer,
            "Authenticate GitHub MCP",
            None,
        );

        let session = state.session_store.get(&session_id).expect("session");
        assert_eq!(session.info.status, "awaiting_auth");
        assert_eq!(
            session.pending_auth_id.as_deref(),
            Some(auth.request_id.as_str())
        );
        drop(session);

        let failed = state
            .mark_tool_auth_failed(&auth.request_id, "token rejected")
            .expect("tool auth should fail");
        assert_eq!(failed.status, PendingAuthStatus::Failed);
        assert_eq!(failed.last_error.as_deref(), Some("token rejected"));
        assert_eq!(state.pending_tool_auth_count(), 1);

        let retried = state
            .submit_tool_auth_token(
                &auth.request_id,
                maestro_core::AuthToken::new("retry-token", AuthTokenType::Bearer),
            )
            .expect("tool auth should allow retry");
        assert_eq!(retried.status, PendingAuthStatus::Submitted);
        assert!(retried.last_error.is_none());

        let connected = state
            .mark_tool_auth_connected(&auth.request_id)
            .expect("tool auth should connect");
        assert_eq!(connected.status, PendingAuthStatus::Connected);

        let session = state.session_store.get(&session_id).expect("session");
        assert_eq!(session.info.status, "active");
        assert!(session.pending_auth_id.is_none());
    }

    #[test]
    fn test_mark_tool_auth_connected_for_server_clears_sibling_requests() {
        let state = GatewayState::new();
        for session_id in ["sess-a", "sess-b"] {
            state.session_store.insert(
                session_id.to_string(),
                StoredAgentSession {
                    info: crate::agent::SessionInfo {
                        id: session_id.to_string(),
                        thread_count: 1,
                        turn_count: 0,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        status: "idle".into(),
                    },
                    thread: maestro_claw::Thread::new(session_id.to_string()),
                    provider: "openai".into(),
                    model: "gpt-4o".into(),
                    pending_approval_id: None,
                    pending_auth_id: None,
                },
            );
        }

        let auth_a = state.enqueue_tool_auth(
            "github",
            Some("sess-a"),
            GatewayAuthTokenType::Bearer,
            "Authenticate GitHub MCP",
            None,
        );
        let auth_b = state.enqueue_tool_auth(
            "github",
            Some("sess-b"),
            GatewayAuthTokenType::Bearer,
            "Authenticate GitHub MCP",
            None,
        );

        let resolved = state
            .mark_tool_auth_connected_for_server("github")
            .expect("server auth should resolve");
        assert_eq!(resolved.len(), 2);
        assert!(resolved
            .iter()
            .all(|auth| auth.status == PendingAuthStatus::Connected));
        assert_eq!(state.pending_tool_auth_count(), 0);
        assert!(state
            .session_store
            .get("sess-a")
            .unwrap()
            .pending_auth_id
            .is_none());
        assert!(state
            .session_store
            .get("sess-b")
            .unwrap()
            .pending_auth_id
            .is_none());
        assert_eq!(auth_a.server_name, "github");
        assert_eq!(auth_b.server_name, "github");
    }

    #[tokio::test]
    async fn test_remove_session_cleans_pending_state() {
        let state = GatewayState::new();
        let session_id = "sess-clean".to_string();
        state.session_store.insert(
            session_id.clone(),
            StoredAgentSession {
                info: crate::agent::SessionInfo {
                    id: session_id.clone(),
                    thread_count: 1,
                    turn_count: 0,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    status: "idle".into(),
                },
                thread: maestro_claw::Thread::new(session_id.clone()),
                provider: "openai".into(),
                model: "gpt-4o".into(),
                pending_approval_id: None,
                pending_auth_id: None,
            },
        );

        let approval = state.enqueue_approval(
            &session_id,
            "thread-1",
            "shell",
            "shell_exec",
            serde_json::json!({"command": "pwd"}),
        );
        let auth = state.enqueue_tool_auth(
            "github",
            Some(&session_id),
            GatewayAuthTokenType::Bearer,
            "Authenticate GitHub MCP",
            None,
        );

        assert!(state.remove_session(&session_id));
        assert!(state.session_store.get(&session_id).is_none());
        assert!(state.pending_approvals.get(&approval.request_id).is_none());
        assert!(state
            .subscribe_approval_resolution(&approval.request_id)
            .is_none());
        assert!(state.pending_tool_auth.get(&auth.request_id).is_none());
    }

    #[test]
    fn test_issue_and_validate_token() {
        let state = GatewayState::new();
        let mut scopes = HashSet::new();
        scopes.insert("sessions".to_string());
        scopes.insert("tools".to_string());

        let token = state.issue_token(scopes.clone(), 3600, Some("test-device".into()));

        assert_eq!(token.token_type, TokenType::Issued);
        assert!(token.has_scope("sessions"));
        assert!(token.has_scope("tools"));
        assert!(!token.has_scope("approvals"));

        // Validate the token
        let auth_ctx = state
            .validate_token(&token.token)
            .expect("token should be valid");
        assert_eq!(auth_ctx.token_type, TokenType::Issued);
        assert!(auth_ctx.has_scope("sessions"));
        assert!(!auth_ctx.has_scope("approvals"));
    }

    #[test]
    fn test_master_key_validation() {
        let mut config = GatewayConfig::default();
        config.agent_api_key = Some("master-key-123".to_string());
        let state = GatewayState::with_config(config);

        // Master key should validate with all scopes
        let auth_ctx = state
            .validate_token("master-key-123")
            .expect("master key should be valid");
        assert_eq!(auth_ctx.token_type, TokenType::Master);
        assert!(auth_ctx.has_scope("sessions"));
        assert!(auth_ctx.has_scope("approvals"));
        assert!(auth_ctx.has_scope("tools"));
        assert!(auth_ctx.has_scope("system"));
    }

    #[test]
    fn test_token_expiry() {
        let state = GatewayState::new();
        let mut scopes = HashSet::new();
        scopes.insert("sessions".to_string());

        // Issue a token with 1 second TTL
        let token = state.issue_token(scopes, 1, None);
        assert!(!token.is_expired());

        // Manually create an expired token
        let mut expired_token = token.clone();
        expired_token.expires_at = "2020-01-01T00:00:00Z".to_string();
        assert!(expired_token.is_expired());
    }

    #[test]
    fn test_intersect_scopes_master() {
        let auth_ctx = AuthContext::master();
        let mut requested = HashSet::new();
        requested.insert("sessions".to_string());
        requested.insert("tools".to_string());

        let result = auth_ctx.intersect_scopes(&requested);
        assert_eq!(result.len(), 2);
        assert!(result.contains("sessions"));
        assert!(result.contains("tools"));
    }

    #[test]
    fn test_intersect_scopes_issued() {
        let mut authorized = HashSet::new();
        authorized.insert("sessions".to_string());
        authorized.insert("tools".to_string());
        let auth_ctx = AuthContext::issued(authorized, None);

        let mut requested = HashSet::new();
        requested.insert("sessions".to_string());
        requested.insert("approvals".to_string());

        let result = auth_ctx.intersect_scopes(&requested);
        assert_eq!(result.len(), 1);
        assert!(result.contains("sessions"));
        assert!(!result.contains("approvals"));
    }

    #[test]
    fn test_create_and_verify_pairing() {
        let state = GatewayState::new();
        let mut scopes = HashSet::new();
        scopes.insert("sessions".to_string());

        let pairing = state.create_pairing(Some("test-device".into()), scopes.clone());
        assert_eq!(pairing.device_name, Some("test-device".into()));
        assert!(!pairing.is_expired());

        // Verify the pairing code
        let token = state
            .verify_pairing(&pairing.code, 3600)
            .expect("pairing should verify");
        assert_eq!(token.token_type, TokenType::Issued);
        assert!(token.has_scope("sessions"));
        assert_eq!(token.device_name, Some("test-device".into()));

        // Code should be consumed
        assert!(state.verify_pairing(&pairing.code, 3600).is_none());
    }

    #[test]
    fn test_pairing_expiry() {
        let state = GatewayState::new();
        let scopes = HashSet::new();

        let mut pairing = state.create_pairing(None, scopes);
        let pairing_code = pairing.code.clone();
        pairing.expires_at = "2020-01-01T00:00:00Z".to_string();
        assert!(pairing.is_expired());

        // Manually insert expired pairing
        state.pending_pairings.insert(pairing.code.clone(), pairing);

        // Expired pairing should not verify
        assert!(state.verify_pairing(&pairing_code, 3600).is_none());
    }

    #[test]
    fn test_token_revoke() {
        let state = GatewayState::new();
        let scopes = HashSet::new();

        let token = state.issue_token(scopes, 3600, None);
        assert!(state.validate_token(&token.token).is_some());

        // Revoke the token
        assert!(state.revoke_token(&token.token));
        assert!(state.validate_token(&token.token).is_none());

        // Revoking non-existent token returns false
        assert!(!state.revoke_token("non-existent-token"));
    }

    #[test]
    fn test_gc_expired_tokens() {
        let state = GatewayState::new();
        let scopes = HashSet::new();

        // Issue a valid token
        let valid_token = state.issue_token(scopes.clone(), 3600, None);

        // Create and insert an expired token directly
        let mut expired_token = IssuedToken::new(scopes, 3600, None);
        expired_token.expires_at = "2020-01-01T00:00:00Z".to_string();
        state
            .issued_tokens
            .insert(expired_token.token.clone(), expired_token);

        assert_eq!(state.issued_tokens.len(), 2);

        // GC should remove only the expired token
        state.gc_expired_tokens();
        assert_eq!(state.issued_tokens.len(), 1);
        assert!(state.issued_tokens.get(&valid_token.token).is_some());
    }

    #[test]
    fn test_list_pending_pairings() {
        let state = GatewayState::new();
        let scopes = HashSet::new();

        let _pairing1 = state.create_pairing(Some("device1".into()), scopes.clone());
        let _pairing2 = state.create_pairing(Some("device2".into()), scopes);

        let pending = state.list_pending_pairings();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_token_scope_from_str() {
        assert_eq!(TokenScope::from_str("sessions"), Some(TokenScope::Sessions));
        assert_eq!(
            TokenScope::from_str("approvals"),
            Some(TokenScope::Approvals)
        );
        assert_eq!(TokenScope::from_str("tools"), Some(TokenScope::Tools));
        assert_eq!(TokenScope::from_str("cron"), Some(TokenScope::Cron));
        assert_eq!(TokenScope::from_str("system"), Some(TokenScope::System));
        assert_eq!(
            TokenScope::from_str("full_access"),
            Some(TokenScope::FullAccess)
        );
        assert_eq!(TokenScope::from_str("invalid"), None);
    }

    #[test]
    fn test_token_scope_as_str() {
        assert_eq!(TokenScope::Sessions.as_str(), "sessions");
        assert_eq!(TokenScope::Approvals.as_str(), "approvals");
        assert_eq!(TokenScope::Tools.as_str(), "tools");
        assert_eq!(TokenScope::Cron.as_str(), "cron");
        assert_eq!(TokenScope::System.as_str(), "system");
        assert_eq!(TokenScope::FullAccess.as_str(), "full_access");
    }
}
