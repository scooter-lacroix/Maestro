//! WebSocket handling for real-time communication
//!
//! Based on Moltis WebSocket lifecycle pattern with read/write split.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, ConnectInfo,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::protocol::{RequestFrame, ResponseFrame};
use crate::state::GatewayState;

/// WebSocket connection handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GatewayState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    ws.on_upgrade(move |socket| handle_connection(socket, state, addr))
}

/// Handle a WebSocket connection
async fn handle_connection(socket: WebSocket, state: Arc<GatewayState>, remote_addr: SocketAddr) {
    info!("WebSocket connection established from {}", remote_addr);

    // Split into read and write halves
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Channel for outgoing messages to this client
    let (client_tx, mut client_rx) = mpsc::channel::<String>(32);

    // Subscribe to broadcast events
    let mut event_rx = state.event_bus.subscribe();

    // Spawn write loop: forwards frames from channels to WebSocket
    let write_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Messages directed to this client
                Some(msg) = client_rx.recv() => {
                    if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                        debug!("WebSocket write error, client disconnected");
                        break;
                    }
                }
                // Broadcast events
                Ok(event) = event_rx.recv() => {
                    let json = event.to_json().unwrap_or_default();
                    if ws_tx.send(Message::Text(json.into())).await.is_err() {
                        debug!("WebSocket write error, client disconnected");
                        break;
                    }
                }
                else => break,
            }
        }
    });

    // Read loop: handle incoming messages from client
    let read_state = state.clone();
    let read_handle = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    handle_text_message(&text, &client_tx, &read_state).await;
                }
                Ok(Message::Binary(data)) => {
                    let text = String::from_utf8_lossy(&data);
                    handle_text_message(&text, &client_tx, &read_state).await;
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping: {} bytes", data.len());
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} requested close", remote_addr);
                    break;
                }
                Err(e) => {
                    warn!("WebSocket error from {}: {}", remote_addr, e);
                    break;
                }
            }
        }
    });

    // Wait for either loop to finish
    tokio::select! {
        _ = write_handle => debug!("Write loop finished for {}", remote_addr),
        _ = read_handle => debug!("Read loop finished for {}", remote_addr),
    }

    info!("WebSocket connection closed for {}", remote_addr);
}

/// Handle a text message from the client
async fn handle_text_message(
    text: &str,
    client_tx: &mpsc::Sender<String>,
    state: &Arc<GatewayState>,
) {
    // Parse as request frame
    let request: Result<RequestFrame, _> = serde_json::from_str(text);

    match request {
        Ok(req) => {
            debug!("Received request: {} (id={})", req.method, req.id);

            // Look up method handler
            let response = if let Some(handler) = state.method_registry.get(&req.method) {
                handler(req.clone(), state).await
            } else {
                ResponseFrame::error(
                    &req.id,
                    format!("Method not found: {}", req.method),
                    Some(crate::protocol::error_codes::METHOD_NOT_FOUND),
                )
            };

            // Send response
            if let Ok(json) = response.to_json() {
                if client_tx.send(json).await.is_err() {
                    warn!("Failed to send response, client disconnected");
                }
            }
        }
        Err(e) => {
            warn!("Failed to parse request: {}", e);
            let response = ResponseFrame::error(
                "unknown",
                format!("Invalid request: {}", e),
                Some(crate::protocol::error_codes::INVALID_REQUEST),
            );
            if let Ok(json) = response.to_json() {
                let _ = client_tx.send(json).await;
            }
        }
    }
}

/// Method handler type - uses Arc for cloning
pub type MethodHandler = Arc<dyn Fn(RequestFrame, &Arc<GatewayState>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ResponseFrame> + Send + '_>> + Send + Sync>;

/// Registry of method handlers
#[derive(Default, Clone)]
pub struct MethodRegistry {
    handlers: std::collections::HashMap<String, MethodHandler>,
}

impl MethodRegistry {
    /// Create a new method registry
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    /// Register a method handler
    pub fn register<F, Fut>(&mut self, method: &str, handler: F)
    where
        F: Fn(RequestFrame, Arc<GatewayState>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ResponseFrame> + Send + 'static,
    {
        self.handlers.insert(
            method.to_string(),
            Arc::new(move |req, state| {
                let state = state.clone();
                Box::pin(handler(req, state))
            }),
        );
    }

    /// Get a method handler
    pub fn get(&self, method: &str) -> Option<MethodHandler> {
        self.handlers.get(method).cloned()
    }

    /// List registered methods
    pub fn list_methods(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

/// Built-in method handlers
pub fn builtin_handlers() -> Vec<(&'static str, fn(RequestFrame, Arc<GatewayState>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ResponseFrame> + Send + 'static>>)> {
    vec![
        ("ping", |req, _| {
            Box::pin(async move {
                ResponseFrame::success(&req.id, Some(serde_json::json!({"pong": true})))
            })
        }),
        ("echo", |req, _| {
            Box::pin(async move {
                ResponseFrame::success(&req.id, req.params)
            })
        }),
        ("methods/list", |req, state| {
            Box::pin(async move {
                let methods = state.method_registry.list_methods();
                ResponseFrame::success(&req.id, Some(serde_json::json!({ "methods": methods })))
            })
        }),
        ("session/status", |req, state| {
            Box::pin(async move {
                let (registered, connected) = state.mcp_manager.try_get_status();
                ResponseFrame::success(&req.id, Some(serde_json::json!({
                    "mcp_servers": {
                        "registered": registered,
                        "connected": connected,
                    },
                    "cron_jobs": state.cron_jobs.len(),
                })))
            })
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_registry() {
        let mut registry = MethodRegistry::new();

        registry.register("test", |req, _| {
            Box::pin(async move { ResponseFrame::success(&req.id, None) })
        });

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_builtin_handlers() {
        let handlers = builtin_handlers();
        assert!(handlers.iter().any(|(m, _)| *m == "ping"));
        assert!(handlers.iter().any(|(m, _)| *m == "echo"));
        assert!(handlers.iter().any(|(m, _)| *m == "methods/list"));
    }
}
