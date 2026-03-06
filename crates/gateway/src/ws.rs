//! WebSocket handling for real-time communication
//!
//! Based on Moltis WebSocket lifecycle pattern with read/write split.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Query, State,
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::protocol::{RequestFrame, ResponseFrame};
use crate::state::{scopes, AuthContext, GatewayState};

/// Maximum allowed message size in bytes (1MB)
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

#[derive(Debug, Default, serde::Deserialize)]
pub struct WsQuery {
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub scopes: Option<String>,
}

/// WebSocket connection handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    let query_token = query.api_key.as_deref().or(query.access_token.as_deref());
    let auth = match crate::agent_runtime::verify_agent_auth(&state, &headers, query_token) {
        Ok(auth) => auth,
        Err(error) => return error.into_response(),
    };

    let requested_scopes = crate::agent_runtime::parse_event_scopes(query.scopes.as_deref());
    let scopes = auth.intersect_scopes(&requested_scopes);
    ws.on_upgrade(move |socket| handle_connection(socket, state, addr, scopes, auth))
}

/// Handle a WebSocket connection
async fn handle_connection(
    socket: WebSocket,
    state: Arc<GatewayState>,
    remote_addr: SocketAddr,
    scopes: std::collections::HashSet<String>,
    auth: AuthContext,
) {
    info!("WebSocket connection established from {}", remote_addr);
    state.add_connection();

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
                    if !crate::agent_runtime::event_visible(&event, &scopes) {
                        continue;
                    }
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
                    handle_text_message(&text, &client_tx, &read_state, &read_client_id, &auth)
                        .await;
                }
                Ok(Message::Binary(data)) => {
                    let text = String::from_utf8_lossy(&data);
                    handle_text_message(&text, &client_tx, &read_state, &read_client_id, &auth)
                        .await;
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

    state.remove_connection();
    info!("WebSocket connection closed for {}", remote_addr);
}

/// Handle a text message from the client
async fn handle_text_message(
    text: &str,
    client_tx: &mpsc::Sender<String>,
    state: &Arc<GatewayState>,
    client_id: &str,
    auth: &AuthContext,
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

            if let Some(required_scope) = required_scope_for_method(&req.method) {
                if !auth.has_scope(required_scope) {
                    let response = ResponseFrame::error(
                        &req.id,
                        format!(
                            "Method '{}' requires '{}' scope",
                            req.method, required_scope
                        ),
                        Some(403),
                    );
                    if let Ok(json) = response.to_json() {
                        let _ = client_tx.send(json).await;
                    }
                    return;
                }
            }

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

fn required_scope_for_method(method: &str) -> Option<&'static str> {
    match method {
        "agent/execute" | "agent/session/list" | "agent/session/create" | "agent/status" => {
            Some(scopes::SESSIONS)
        }
        "approval/list" | "approval/resolve" => Some(scopes::APPROVALS),
        "mcp/auth/list"
        | "mcp/auth/submit"
        | "mcp/server/list"
        | "mcp/server/register"
        | "mcp/server/remove"
        | "mcp/server/connect"
        | "mcp/server/disconnect" => Some(scopes::TOOLS),
        "methods/list" | "session/status" => Some(scopes::SYSTEM),
        _ => None,
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
                let exec_req: Result<crate::agent::AgentExecuteRequest, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match exec_req {
                    Ok(exec_req) => {
                        match crate::agent_runtime::execute_agent_request(state.clone(), exec_req)
                            .await
                        {
                            Ok(response) => ResponseFrame::success(
                                &req.id,
                                Some(serde_json::to_value(response).unwrap_or_default()),
                            ),
                            Err(error) => error.to_ws_response(&req.id),
                        }
                    }
                    Err(e) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", e), None)
                    }
                }
            })
        }),
        ("agent/session/list", |req, state| {
            Box::pin(async move {
                ResponseFrame::success(
                    &req.id,
                    Some(
                        serde_json::to_value(crate::agent_runtime::list_sessions(&state))
                            .unwrap_or_default(),
                    ),
                )
            })
        }),
        ("agent/session/create", |req, state| {
            Box::pin(async move {
                let create_req: Result<crate::agent::SessionCreateRequest, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match create_req {
                    Ok(create_req) => {
                        let created = crate::agent_runtime::create_session(&state, &create_req);
                        ResponseFrame::success(
                            &req.id,
                            Some(serde_json::to_value(created).unwrap_or_default()),
                        )
                    }
                    Err(e) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", e), None)
                    }
                }
            })
        }),
        ("agent/status", |req, state| {
            Box::pin(async move {
                ResponseFrame::success(
                    &req.id,
                    Some(
                        serde_json::to_value(crate::agent_runtime::agent_status(&state))
                            .unwrap_or_default(),
                    ),
                )
            })
        }),
        ("approval/list", |req, state| {
            Box::pin(async move {
                ResponseFrame::success(
                    &req.id,
                    Some(
                        serde_json::to_value(crate::agent_runtime::approval_queue(&state))
                            .unwrap_or_default(),
                    ),
                )
            })
        }),
        ("approval/resolve", |req, state| {
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                struct ApprovalResolveParams {
                    request_id: String,
                    decision: crate::agent::ApprovalDecisionValue,
                }

                let params: Result<ApprovalResolveParams, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match params {
                    Ok(params) => match crate::agent_runtime::resolve_approval_request(
                        &state,
                        &params.request_id,
                        &crate::agent::ApprovalDecisionRequest {
                            decision: params.decision,
                        },
                    ) {
                        Ok(response) => ResponseFrame::success(
                            &req.id,
                            Some(serde_json::to_value(response).unwrap_or_default()),
                        ),
                        Err(error) => error.to_ws_response(&req.id),
                    },
                    Err(error) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", error), None)
                    }
                }
            })
        }),
        ("mcp/auth/list", |req, state| {
            Box::pin(async move {
                ResponseFrame::success(
                    &req.id,
                    Some(
                        serde_json::to_value(crate::agent_runtime::pending_tool_auth(&state))
                            .unwrap_or_default(),
                    ),
                )
            })
        }),
        ("mcp/auth/submit", |req, state| {
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                struct AuthSubmitParams {
                    request_id: String,
                    token: String,
                    #[serde(default)]
                    token_type: Option<crate::agent::GatewayAuthTokenType>,
                }

                let params: Result<AuthSubmitParams, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match params {
                    Ok(params) => match crate::agent_runtime::submit_mcp_auth(
                        &state,
                        &params.request_id,
                        crate::agent_runtime::gateway_auth_token(params.token, params.token_type),
                    )
                    .await
                    {
                        Ok(response) => ResponseFrame::success(
                            &req.id,
                            Some(serde_json::to_value(response).unwrap_or_default()),
                        ),
                        Err(error) => error.to_ws_response(&req.id),
                    },
                    Err(error) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", error), None)
                    }
                }
            })
        }),
        ("mcp/server/list", |req, state| {
            Box::pin(async move {
                ResponseFrame::success(
                    &req.id,
                    Some(
                        serde_json::to_value(crate::agent_runtime::list_mcp_servers(&state).await)
                            .unwrap_or_default(),
                    ),
                )
            })
        }),
        ("mcp/server/register", |req, state| {
            Box::pin(async move {
                let params: Result<crate::agent::McpServerRegisterRequest, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match params {
                    Ok(params) => {
                        match crate::agent_runtime::register_or_update_mcp_server(&state, &params)
                            .await
                        {
                            Ok(response) => ResponseFrame::success(
                                &req.id,
                                Some(serde_json::to_value(response).unwrap_or_default()),
                            ),
                            Err(error) => error.to_ws_response(&req.id),
                        }
                    }
                    Err(error) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", error), None)
                    }
                }
            })
        }),
        ("mcp/server/remove", |req, state| {
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                struct RemoveParams {
                    name: String,
                }

                let params: Result<RemoveParams, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match params {
                    Ok(params) => {
                        match crate::agent_runtime::remove_mcp_server(&state, &params.name).await {
                            Ok(response) => ResponseFrame::success(
                                &req.id,
                                Some(serde_json::to_value(response).unwrap_or_default()),
                            ),
                            Err(error) => error.to_ws_response(&req.id),
                        }
                    }
                    Err(error) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", error), None)
                    }
                }
            })
        }),
        ("mcp/server/connect", |req, state| {
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                struct ConnectParams {
                    name: String,
                }

                let params: Result<ConnectParams, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match params {
                    Ok(params) => {
                        match crate::agent_runtime::connect_mcp_server(state.clone(), &params.name)
                            .await
                        {
                            Ok(crate::agent_runtime::McpConnectOutcome::Connected) => {
                                ResponseFrame::success(
                                    &req.id,
                                    Some(serde_json::json!({ "connected": true })),
                                )
                            }
                            Ok(crate::agent_runtime::McpConnectOutcome::AuthRequired(auth)) => {
                                ResponseFrame::success(
                                    &req.id,
                                    Some(serde_json::json!({
                                        "connected": false,
                                        "auth_required": true,
                                        "auth": auth,
                                    })),
                                )
                            }
                            Err(error) => error.to_ws_response(&req.id),
                        }
                    }
                    Err(error) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", error), None)
                    }
                }
            })
        }),
        ("mcp/server/disconnect", |req, state| {
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                struct DisconnectParams {
                    name: String,
                }

                let params: Result<DisconnectParams, _> =
                    serde_json::from_value(req.params.clone().unwrap_or(serde_json::json!({})));

                match params {
                    Ok(params) => {
                        match crate::agent_runtime::disconnect_mcp_server(&state, &params.name)
                            .await
                        {
                            Ok(()) => ResponseFrame::success(
                                &req.id,
                                Some(serde_json::json!({ "disconnected": true })),
                            ),
                            Err(error) => error.to_ws_response(&req.id),
                        }
                    }
                    Err(error) => {
                        ResponseFrame::error(&req.id, format!("Invalid request: {}", error), None)
                    }
                }
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
        assert!(handlers.iter().any(|(m, _)| *m == "mcp/server/list"));
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

    #[test]
    fn test_required_scope_for_mcp_management_methods() {
        assert_eq!(
            required_scope_for_method("mcp/server/list"),
            Some(scopes::TOOLS)
        );
        assert_eq!(
            required_scope_for_method("mcp/server/register"),
            Some(scopes::TOOLS)
        );
        assert_eq!(
            required_scope_for_method("mcp/server/disconnect"),
            Some(scopes::TOOLS)
        );
    }
}
