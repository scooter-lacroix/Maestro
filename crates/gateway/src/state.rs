//! Shared gateway state
//!
//! Contains all shared state for the gateway including connections, managers, and event bus.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use tokio::sync::broadcast;
use tracing::debug;

use maestro_core::{CronJob, McpManager, SandboxManager};

use crate::protocol::EventFrame;
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
