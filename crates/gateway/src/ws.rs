//! WebSocket handling for real-time communication
//!
//! Based on Moltis WebSocket lifecycle pattern with read/write split.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::protocol::{RequestFrame, ResponseFrame};
use crate::state::GatewayState;

/// Maximum allowed message size in bytes (1MB)
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

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

    // Generate a unique client ID for rate limiting
    let client_id = format!("ws:{}", remote_addr);

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
    let read_client_id = client_id.clone();
    let read_handle = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    handle_text_message(&text, &client_tx, &read_state, &read_client_id).await;
                }
                Ok(Message::Binary(data)) => {
                    let text = String::from_utf8_lossy(&data);
                    handle_text_message(&text, &client_tx, &read_state, &read_client_id).await;
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
    client_id: &str,
) {
    // SECURITY: Check message size limit before processing
    if text.len() > MAX_MESSAGE_SIZE {
        warn!(
            "Message size {} bytes exceeds limit {} for client: {}",
            text.len(),
            MAX_MESSAGE_SIZE,
            client_id
        );
        let response = ResponseFrame::error(
            "unknown",
            format!(
                "Message too large. Maximum size is {} bytes",
                MAX_MESSAGE_SIZE
            ),
            Some(413), // HTTP 413 Payload Too Large
        );
        if let Ok(json) = response.to_json() {
            let _ = client_tx.send(json).await;
        }
        return;
    }

    // SECURITY: Check rate limit for this client
    let (allowed, _remaining, retry_after) = state.ws_rate_limiter.check(client_id);
    if !allowed {
        warn!("WebSocket rate limit exceeded for client: {}", client_id);
        let response = ResponseFrame::error(
            "unknown",
            format!(
                "Rate limit exceeded. Try again in {} ms",
                retry_after.unwrap_or(1000)
            ),
            Some(429), // HTTP 429 Too Many Requests
        );
        if let Ok(json) = response.to_json() {
            let _ = client_tx.send(json).await;
        }
        return;
    }

    // Parse as request frame
    let request: Result<RequestFrame, _> = serde_json::from_str(text);

    match request {
        Ok(req) => {
            debug!("Received request: {} (id={})", req.method, req.id);

            // Look up method handler
            let response = if let Some(handler) = state.method_registry.get(&req.method) {
                handler(req.clone(), &state.clone()).await
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
pub type MethodHandler = Arc<
    dyn Fn(
            RequestFrame,
            &Arc<GatewayState>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = ResponseFrame> + Send + '_>>
        + Send
        + Sync,
>;

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
pub fn builtin_handlers() -> Vec<(
    &'static str,
    fn(
        RequestFrame,
        Arc<GatewayState>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ResponseFrame> + Send + 'static>>,
)> {
    vec![
        ("ping", |req, _| {
            Box::pin(async move {
                ResponseFrame::success(&req.id, Some(serde_json::json!({"pong": true})))
            })
        }),
        ("echo", |req, _| {
            Box::pin(async move { ResponseFrame::success(&req.id, req.params) })
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
                ResponseFrame::success(
                    &req.id,
                    Some(serde_json::json!({
                        "mcp_servers": {
                            "registered": registered,
                            "connected": connected,
                        },
                        "cron_jobs": state.cron_jobs.len(),
                    })),
                )
            })
        }),
        // Agent WebSocket methods
        ("agent/execute", |req, state| {
            Box::pin(async move {
                // Parse the request
                let exec_req: Result<crate::agent::AgentExecuteRequest, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match exec_req {
                    Ok(exec_req) => {
                        // Broadcast event
                        let event = crate::protocol::EventFrame::new(
                            "agent.execute.started",
                            Some(serde_json::json!({
                                "prompt_preview": if exec_req.prompt.len() > 100 {
                                    format!("{}...", &exec_req.prompt[..97])
                                } else {
                                    exec_req.prompt.clone()
                                },
                                "provider": exec_req.provider,
                            })),
                            Some(state.next_seq()),
                        );
                        let _ = state.event_bus.send(event);

                        // TODO: Wire to maestro-claw agent_loop
                        ResponseFrame::success(
                            &req.id,
                            Some(serde_json::json!({
                                "status": "accepted",
                                "message": "Agent execution queued (not yet wired to maestro-claw)",
                            })),
                        )
                    }
                    Err(e) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", e), None)
                    }
                }
            })
        }),
        ("agent/session/list", |req, _state| {
            Box::pin(async move {
                // TODO: Wire to maestro-claw session store
                ResponseFrame::success(
                    &req.id,
                    Some(serde_json::to_value(crate::agent::SessionListResponse::empty()).unwrap()),
                )
            })
        }),
        ("agent/session/create", |req, _state| {
            Box::pin(async move {
                // Parse the request
                let create_req: Result<crate::agent::SessionCreateRequest, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match create_req {
                    Ok(create_req) => {
                        // TODO: Wire to maestro-claw session creation
                        ResponseFrame::success(
                            &req.id,
                            Some(serde_json::json!({
                                "session_id": uuid::Uuid::new_v4().to_string(),
                                "provider": create_req.provider,
                                "model": create_req.model,
                                "status": "created",
                            })),
                        )
                    }
                    Err(e) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", e), None)
                    }
                }
            })
        }),
        ("agent/status", |req, _state| {
            Box::pin(async move {
                // TODO: Wire to actual agent status
                ResponseFrame::success(
                    &req.id,
                    Some(serde_json::json!({
                        "status": "idle",
                        "sessions": 0,
                        "active_runs": 0,
                    })),
                )
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

    #[test]
    fn test_max_message_size_constant() {
        // Verify the max message size is set to 1MB
        assert_eq!(MAX_MESSAGE_SIZE, 1024 * 1024);
    }

    #[test]
    fn test_message_size_check() {
        // Test that size limit is reasonable
        assert!(MAX_MESSAGE_SIZE > 0);
        assert!(MAX_MESSAGE_SIZE < 100 * 1024 * 1024); // Less than 100MB
    }
}
