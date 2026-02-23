//! Bridge module for MaestroTabMultiplexer
//!
//! This module provides bridges between Maestro and external systems,
//! including the WebSocket to PTY bridge for tab-daemon integration.

pub mod websocket_pty_bridge;

pub use websocket_pty_bridge::{
    BridgeCommand,
    BridgeEvent,
    BridgeState,
    BridgeStats,
    WebSocketPtyBridge,
    WebSocketPtyBridgeConfig,
    WebSocketPtyBridgeConfigBuilder,
    ReconnectionPolicy,
    create_tab_metadata,
};

use std::fmt;
use thiserror::Error;

/// Errors that can occur in bridge operations
#[derive(Error, Debug, Clone)]
pub enum BridgeError {
    /// Connection to daemon failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Channel closed unexpectedly
    #[error("Channel closed: {0}")]
    ChannelClosed(String),

    /// Invalid bridge state for operation
    #[error("Invalid bridge state: {0}")]
    InvalidState(String),

    /// Task execution error
    #[error("Task error: {0}")]
    TaskError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Result type for bridge operations
pub type BridgeResult<T> = Result<T, BridgeError>;

/// Trait for bridge implementations
#[async_trait::async_trait]
pub trait Bridge: Send + Sync {
    /// Start the bridge
    async fn start(&self) -> BridgeResult<()>;

    /// Stop the bridge gracefully
    async fn stop(self) -> BridgeResult<()>;

    /// Check if the bridge is connected
    async fn is_connected(&self) -> bool;

    /// Get the current state of the bridge
    async fn state(&self) -> BridgeState;
}

/// Bridge factory for creating bridge instances
pub struct BridgeFactory;

impl BridgeFactory {
    /// Create a new WebSocket PTY bridge
    pub fn create_websocket_pty_bridge(
        config: WebSocketPtyBridgeConfig,
    ) -> BridgeResult<WebSocketPtyBridge> {
        WebSocketPtyBridge::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_error_display() {
        let err = BridgeError::ConnectionFailed("test error".to_string());
        assert_eq!(err.to_string(), "Connection failed: test error");

        let err = BridgeError::InvalidState("disconnected".to_string());
        assert_eq!(err.to_string(), "Invalid bridge state: disconnected");
    }

    #[test]
    fn test_bridge_result() {
        let ok_result: BridgeResult<i32> = Ok(42);
        assert!(ok_result.is_ok());

        let err_result: BridgeResult<i32> = Err(BridgeError::ChannelClosed("test".to_string()));
        assert!(err_result.is_err());
    }
}
