//! Shared gateway state
//!
//! Contains all shared state for the gateway including connections, managers, and event bus.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::broadcast;
use tracing::debug;

use maestro_core::{CronJob, McpManager, SandboxManager};

use crate::protocol::EventFrame;
use crate::rate_limit::{RateLimitConfig, SlidingWindowRateLimiter};
use crate::ws::MethodRegistry;

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
                limit: 60,          // 60 messages per minute
                window: Duration::from_secs(60),
                include_retry_after: true,
            },
            broadcast_rate_limit: RateLimitConfig {
                limit: 10,          // 10 broadcasts per minute
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
            cors_allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
            cors_allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
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

        Self {
            config,
            start_time: Instant::now(),
            connection_count: AtomicUsize::new(0),
            event_seq: AtomicU64::new(0),
            event_bus: event_tx,
            method_registry,
            mcp_manager: McpManager::new(),
            sandbox_manager: SandboxManager::new(maestro_core::SecurityPolicy::default()),
            cron_jobs: Vec::new(),
            ws_rate_limiter: Arc::new(SlidingWindowRateLimiter::new(ws_rate_limit)),
            broadcast_rate_limiter: Arc::new(SlidingWindowRateLimiter::new(broadcast_rate_limit)),
            broadcast_tracking: Mutex::new(std::collections::HashMap::new()),
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

    /// Record a broadcast event for rate limiting
    pub fn record_broadcast(&self, sender_id: &str) {
        let mut tracking = self.broadcast_tracking.lock();
        tracking.insert(sender_id.to_string(), Instant::now());
    }
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

/// Broadcast options for event scoping
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
    m.insert("exec.approval.requested", &[scopes::APPROVALS, scopes::SESSIONS] as &[&str]);
    m.insert("exec.approval.resolved", &[scopes::APPROVALS, scopes::SESSIONS] as &[&str]);
    m.insert("cron.job.started", &[scopes::CRON] as &[&str]);
    m.insert("cron.job.completed", &[scopes::CRON] as &[&str]);
    m.insert("tool.call.started", &[scopes::TOOLS, scopes::SESSIONS] as &[&str]);
    m.insert("tool.call.completed", &[scopes::TOOLS, scopes::SESSIONS] as &[&str]);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
