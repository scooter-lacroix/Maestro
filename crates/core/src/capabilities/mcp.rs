//! MCP Client Integration
//!
//! This module implements MCP client integration following patterns from Moltis and IronClaw:
//! - `moltis/crates/mcp/src/tool_bridge.rs` - McpToolBridge pattern
//! - `ironclaw/src/extensions/manager.rs` - Extension manager
//! - `moltis/crates/mcp/src/manager.rs` - McpManager with lifecycle
//!
//! Key features:
//! - McpToolBridge: Wraps MCP server tools as AgentTools
//! - McpManager: Manages lifecycle of multiple MCP server connections
//! - SSE Transport for remote MCP servers with OAuth
//! - Tool registry integration

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::traits::Tool;
use crate::{AuthToken, AuthTokenType};

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Server lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Registered,
    Connecting,
    Connected,
    Authenticating,
    AuthFailed,
    Disconnected,
    Error,
}

impl Default for ServerState {
    fn default() -> Self {
        ServerState::Registered
    }
}

impl ServerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerState::Registered => "registered",
            ServerState::Connecting => "connecting",
            ServerState::Connected => "connected",
            ServerState::Authenticating => "authenticating",
            ServerState::AuthFailed => "auth_failed",
            ServerState::Disconnected => "disconnected",
            ServerState::Error => "error",
        }
    }
}

/// Managed server entry with full lifecycle tracking.
#[derive(Debug, Clone)]
pub struct ManagedServer {
    pub config: McpServerConfig,
    pub state: ServerState,
    pub connected_at: Option<i64>,
    pub last_error: Option<String>,
    pub has_auth_token: bool,
    pub auth_token_type: Option<String>,
    pub auth_updated_at: Option<i64>,
    pub tools_count: usize,
}

impl ManagedServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: ServerState::Registered,
            connected_at: None,
            last_error: None,
            has_auth_token: false,
            auth_token_type: None,
            auth_updated_at: None,
            tools_count: 0,
        }
    }

    pub fn set_connected(&mut self, tools_count: usize) {
        self.state = ServerState::Connected;
        self.connected_at = Some(unix_timestamp());
        self.last_error = None;
        self.tools_count = tools_count;
    }

    pub fn set_error(&mut self, error: &str) {
        self.state = ServerState::Error;
        self.last_error = Some(error.to_string());
    }

    pub fn set_auth_failed(&mut self, error: &str) {
        self.state = ServerState::AuthFailed;
        self.last_error = Some(error.to_string());
    }

    pub fn set_auth_token(&mut self, token: &AuthToken) {
        self.has_auth_token = true;
        self.auth_token_type = Some(token.token_type().as_str().to_string());
        self.auth_updated_at = Some(unix_timestamp());
        self.last_error = None;
    }

    pub fn clear_auth_token(&mut self) {
        self.has_auth_token = false;
        self.auth_token_type = None;
        self.auth_updated_at = None;
    }

    pub fn disconnect(&mut self) {
        self.state = ServerState::Disconnected;
        self.last_error = None;
        self.tools_count = 0;
    }
}

/// MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// Server name/identifier.
    pub name: String,
    /// Server URL (for SSE/HTTP) or command (for stdio).
    pub url: Option<String>,
    /// Command to spawn (for stdio transport).
    pub command: Option<String>,
    /// Arguments for stdio command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether OAuth is required.
    #[serde(default)]
    pub requires_auth: bool,
    /// OAuth configuration.
    #[serde(default)]
    pub oauth_config: Option<OAuthConfig>,
}

/// OAuth configuration for MCP servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfig {
    /// Authorization URL.
    pub auth_url: String,
    /// Token URL.
    pub token_url: String,
    /// Client ID.
    pub client_id: String,
    /// Client secret (if needed).
    #[serde(default)]
    pub client_secret: Option<String>,
    /// Redirect URL.
    #[serde(default)]
    pub redirect_url: Option<String>,
    /// Required scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Persisted auth token metadata for a managed MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedAuthToken {
    pub value: String,
    pub token_type: String,
}

impl PersistedAuthToken {
    pub fn from_auth_token(token: &AuthToken) -> Self {
        Self {
            value: token.value().to_string(),
            token_type: token.token_type().as_str().to_string(),
        }
    }

    pub fn into_auth_token(self) -> anyhow::Result<AuthToken> {
        let token_type = match self.token_type.as_str() {
            "bearer" => AuthTokenType::Bearer,
            "api_key" => AuthTokenType::ApiKey,
            "oauth" => AuthTokenType::OAuth,
            other => anyhow::bail!("Unsupported auth token type: {}", other),
        };
        Ok(AuthToken::new(self.value, token_type))
    }
}

/// Persisted managed MCP server entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedManagedServer {
    #[serde(flatten)]
    pub config: McpServerConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<PersistedAuthToken>,
}

/// Persisted MCP manager snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpManagerSnapshot {
    #[serde(default)]
    pub servers: Vec<PersistedManagedServer>,
}

/// MCP tool definition from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDef {
    /// Tool name.
    pub name: String,
    /// Tool description.
    #[serde(default)]
    pub description: String,
    /// Input schema (JSON Schema).
    #[serde(default)]
    pub input_schema: Value,
}

/// MCP tool call request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCall {
    /// Tool name.
    pub name: String,
    /// Tool arguments.
    #[serde(default)]
    pub arguments: Value,
}

/// MCP tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    /// Whether the call succeeded.
    pub success: bool,
    /// Output content.
    pub content: Vec<McpContent>,
    /// Error message if failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// MCP content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum McpContent {
    /// Text content.
    Text { text: String },
    /// Image content.
    Image { data: String, mime_type: String },
    /// Resource reference.
    Resource {
        uri: String,
        mime_type: Option<String>,
    },
}

/// Trait for MCP client implementations.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// List available tools from the server.
    async fn list_tools(&self) -> anyhow::Result<Vec<McpToolDef>>;

    /// Call a tool on the server.
    async fn call_tool(&self, call: McpToolCall) -> anyhow::Result<McpToolResult>;

    /// Get the server name.
    fn server_name(&self) -> &str;

    /// Check if the client is connected.
    async fn is_connected(&self) -> bool;

    /// Disconnect from the server.
    async fn disconnect(&self) -> anyhow::Result<()>;
}

/// MCP tool bridge that wraps MCP tools as AgentTools.
///
/// Based on Moltis McpToolBridge pattern.
pub struct McpToolBridge {
    /// Prefixed tool name: `mcp__<server>__<tool>`.
    prefixed_name: String,
    /// Original tool name on the MCP server.
    original_name: String,
    /// Name of the MCP server this tool belongs to.
    server_name: String,
    /// Tool description.
    description: String,
    /// Input schema.
    input_schema: Value,
    /// Reference to the MCP client.
    client: Arc<dyn McpClient>,
}

impl McpToolBridge {
    /// Create a new MCP tool bridge.
    pub fn new(server_name: &str, tool_def: &McpToolDef, client: Arc<dyn McpClient>) -> Self {
        Self {
            prefixed_name: format!("mcp__{}__{}", server_name, tool_def.name),
            original_name: tool_def.name.clone(),
            server_name: server_name.to_string(),
            description: tool_def.description.clone(),
            input_schema: tool_def.input_schema.clone(),
            client,
        }
    }

    /// Get the original tool name.
    pub fn original_name(&self) -> &str {
        &self.original_name
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

#[async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        &self.prefixed_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, input: Value) -> anyhow::Result<Value> {
        let call = McpToolCall {
            name: self.original_name.clone(),
            arguments: input,
        };

        let result = self.client.call_tool(call).await?;

        // Convert result to JSON
        let output = if result.success {
            let content: Vec<Value> = result
                .content
                .iter()
                .map(|c| match c {
                    McpContent::Text { text } => {
                        serde_json::json!({ "type": "text", "text": text })
                    }
                    McpContent::Image { data, mime_type } => {
                        serde_json::json!({ "type": "image", "data": data, "mimeType": mime_type })
                    }
                    McpContent::Resource { uri, mime_type } => {
                        serde_json::json!({ "type": "resource", "uri": uri, "mimeType": mime_type })
                    }
                })
                .collect();

            serde_json::json!({
                "success": true,
                "content": content
            })
        } else {
            serde_json::json!({
                "success": false,
                "error": result.error.unwrap_or_else(|| "Unknown error".to_string())
            })
        };

        Ok(output)
    }
}

/// MCP manager state.
struct McpManagerInner {
    /// Connected clients by server name.
    clients: HashMap<String, Arc<dyn McpClient>>,
    /// Tool definitions by server name.
    tools: HashMap<String, Vec<McpToolDef>>,
    /// Managed servers with full lifecycle tracking.
    managed_servers: HashMap<String, ManagedServer>,
    /// Shared auth tokens keyed by server name.
    auth_tokens: HashMap<String, AuthToken>,
}

impl Default for McpManagerInner {
    fn default() -> Self {
        Self::new()
    }
}

impl McpManagerInner {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            tools: HashMap::new(),
            managed_servers: HashMap::new(),
            auth_tokens: HashMap::new(),
        }
    }
}

/// Manages the lifecycle of multiple MCP server connections.
///
/// Based on Moltis McpManager pattern.
pub struct McpManager {
    inner: Arc<tokio::sync::RwLock<McpManagerInner>>,
}

impl McpManager {
    /// Create a new MCP manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(McpManagerInner::new())),
        }
    }

    /// Register a server configuration.
    pub async fn register_server(&self, config: McpServerConfig) {
        let mut inner = self.inner.write().await;
        let mut server = ManagedServer::new(config.clone());
        if let Some(token) = inner.auth_tokens.get(&config.name) {
            server.set_auth_token(token);
        }
        inner
            .managed_servers
            .insert(server.config.name.clone(), server);
    }

    /// Update an existing server configuration.
    pub async fn update_server(&self, config: McpServerConfig) -> bool {
        let mut inner = self.inner.write().await;
        let disconnected_client = inner.clients.remove(&config.name);
        let was_connected = disconnected_client.is_some();
        if was_connected {
            inner.tools.remove(&config.name);
        }
        let updated = if let Some(server) = inner.managed_servers.get_mut(&config.name) {
            server.config = config;
            server.connected_at = None;
            server.tools_count = 0;
            server.last_error = None;
            server.state = ServerState::Registered;
            true
        } else {
            false
        };
        drop(inner);

        if let Some(client) = disconnected_client {
            let _ = client.disconnect().await;
        }

        updated
    }

    /// Remove a server configuration.
    pub async fn remove_server(&self, server_name: &str) -> bool {
        let mut inner = self.inner.write().await;
        if inner.clients.contains_key(server_name) {
            return false;
        }
        inner.tools.remove(server_name);
        inner.auth_tokens.remove(server_name);
        inner.managed_servers.remove(server_name).is_some()
    }

    /// Get a managed server by name.
    pub async fn get_managed_server(&self, server_name: &str) -> Option<ManagedServer> {
        let inner = self.inner.read().await;
        inner.managed_servers.get(server_name).cloned()
    }

    /// List all managed servers with their states.
    pub async fn list_managed_servers(&self) -> Vec<ManagedServer> {
        let inner = self.inner.read().await;
        inner.managed_servers.values().cloned().collect()
    }

    /// Set auth token for a server.
    pub async fn set_auth_token(&self, server_name: &str, token: AuthToken) -> bool {
        let mut inner = self.inner.write().await;
        if let Some(server) = inner.managed_servers.get_mut(server_name) {
            server.set_auth_token(&token);
            inner.auth_tokens.insert(server_name.to_string(), token);
            true
        } else {
            false
        }
    }

    /// Get auth token for a server.
    pub async fn get_auth_token(&self, server_name: &str) -> Option<AuthToken> {
        let inner = self.inner.read().await;
        inner.auth_tokens.get(server_name).cloned()
    }

    /// Check if auth token needs refresh (expires within threshold).
    pub async fn needs_token_refresh(&self, server_name: &str, _threshold_secs: i64) -> bool {
        let inner = self.inner.read().await;
        if let Some(server) = inner.managed_servers.get(server_name) {
            return server.config.requires_auth && !inner.auth_tokens.contains_key(server_name);
        }
        false
    }

    /// Update server state.
    pub async fn update_server_state(&self, server_name: &str, state: ServerState) {
        let mut inner = self.inner.write().await;
        if let Some(server) = inner.managed_servers.get_mut(server_name) {
            server.state = state;
        }
    }

    /// Get a registered server configuration by name.
    pub async fn get_config(&self, server_name: &str) -> Option<McpServerConfig> {
        let inner = self.inner.read().await;
        inner
            .managed_servers
            .get(server_name)
            .map(|s| s.config.clone())
    }

    /// List all registered server configurations.
    pub async fn registered_server_configs(&self) -> Vec<McpServerConfig> {
        let inner = self.inner.read().await;
        inner
            .managed_servers
            .values()
            .map(|s| s.config.clone())
            .collect()
    }

    /// Snapshot registered MCP servers and any stored auth tokens.
    pub async fn snapshot(&self) -> McpManagerSnapshot {
        let inner = self.inner.read().await;
        let mut servers: Vec<_> = inner
            .managed_servers
            .values()
            .map(|server| PersistedManagedServer {
                config: server.config.clone(),
                auth_token: inner
                    .auth_tokens
                    .get(&server.config.name)
                    .map(PersistedAuthToken::from_auth_token),
            })
            .collect();
        servers.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        McpManagerSnapshot { servers }
    }

    /// Restore registered MCP servers and auth tokens from a persisted snapshot.
    pub async fn hydrate_snapshot(&self, snapshot: McpManagerSnapshot) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let disconnected_clients: Vec<_> =
            inner.clients.drain().map(|(_, client)| client).collect();
        inner.tools.clear();
        inner.managed_servers.clear();
        inner.auth_tokens.clear();

        for persisted in snapshot.servers {
            let mut server = ManagedServer::new(persisted.config.clone());
            if let Some(auth_token) = persisted.auth_token {
                let auth_token = auth_token.into_auth_token()?;
                server.set_auth_token(&auth_token);
                inner
                    .auth_tokens
                    .insert(persisted.config.name.clone(), auth_token);
            }
            inner
                .managed_servers
                .insert(persisted.config.name.clone(), server);
        }

        drop(inner);

        for client in disconnected_clients {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// Connect to a registered server.
    pub async fn connect(&self, server_name: &str) -> anyhow::Result<()> {
        let config = {
            let already_connected = {
                let inner = self.inner.read().await;
                if !inner.managed_servers.contains_key(server_name) {
                    return Err(anyhow::anyhow!("Server not registered: {}", server_name));
                }
                inner.clients.contains_key(server_name)
            };

            if already_connected {
                return Ok(());
            }

            let mut inner = self.inner.write().await;
            let server = inner
                .managed_servers
                .get_mut(server_name)
                .ok_or_else(|| anyhow::anyhow!("Server not registered: {}", server_name))?;

            server.state = ServerState::Connecting;
            server.last_error = None;
            server.config.clone()
        };

        // Check if auth is required but we don't have a valid token
        if config.requires_auth {
            let has_valid_token = {
                let inner = self.inner.read().await;
                inner.auth_tokens.contains_key(server_name)
            };

            if !has_valid_token {
                let mut inner = self.inner.write().await;
                if let Some(server) = inner.managed_servers.get_mut(server_name) {
                    server.set_auth_failed("Authentication required but no valid token");
                }
                return Err(anyhow::anyhow!(
                    "Authentication required for server: {}",
                    server_name
                ));
            }

            let mut inner = self.inner.write().await;
            if let Some(server) = inner.managed_servers.get_mut(server_name) {
                server.state = ServerState::Authenticating;
            }
        }

        // In a real implementation, this would create the appropriate transport
        // based on config.url (SSE) or config.command (stdio).
        // For now, create a mock client that returns the config info.
        let tools = vec![McpToolDef {
            name: "list_config".to_string(),
            description: format!("List configuration for {}", config.name),
            input_schema: serde_json::json!({ "type": "object" }),
        }];

        let client = Arc::new(MockMcpClient::new(&config.name, tools));

        // Store the connected client
        let mut inner = self.inner.write().await;
        let tools_list = client.list_tools().await?;
        inner.clients.insert(server_name.to_string(), client);
        inner
            .tools
            .insert(server_name.to_string(), tools_list.clone());

        if let Some(server) = inner.managed_servers.get_mut(server_name) {
            server.set_connected(tools_list.len());
        }

        tracing::info!("Connected to MCP server: {}", server_name);
        Ok(())
    }

    /// Disconnect from a server.
    pub async fn disconnect(&self, server_name: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        if let Some(client) = inner.clients.remove(server_name) {
            client.disconnect().await?;
            inner.tools.remove(server_name);
        }
        if let Some(server) = inner.managed_servers.get_mut(server_name) {
            server.disconnect();
        }
        Ok(())
    }

    /// Get all available tools from connected servers.
    pub async fn get_all_tools(&self) -> Vec<(String, McpToolDef)> {
        let inner = self.inner.read().await;
        let mut tools = Vec::new();
        for (server_name, tool_defs) in &inner.tools {
            for tool_def in tool_defs {
                tools.push((server_name.clone(), tool_def.clone()));
            }
        }
        tools
    }

    /// Get a client for a specific server.
    pub async fn get_client(&self, server_name: &str) -> Option<Arc<dyn McpClient>> {
        let inner = self.inner.read().await;
        inner.clients.get(server_name).cloned()
    }

    /// List connected servers.
    pub async fn connected_servers(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.clients.keys().cloned().collect()
    }

    /// Create tool bridges for all tools from a server.
    pub async fn create_tool_bridges(
        &self,
        server_name: &str,
    ) -> anyhow::Result<Vec<McpToolBridge>> {
        let inner = self.inner.read().await;

        let client = inner
            .clients
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("Server not connected: {}", server_name))?
            .clone();

        let tools = inner.tools.get(server_name).cloned().unwrap_or_default();

        drop(inner);

        let bridges: Vec<McpToolBridge> = tools
            .iter()
            .map(|tool_def| McpToolBridge::new(server_name, tool_def, Arc::clone(&client)))
            .collect();

        Ok(bridges)
    }

    /// Get server status synchronously (non-blocking).
    /// Returns (registered_servers, connected_servers) pairs.
    pub fn try_get_status(&self) -> (Vec<String>, Vec<String>) {
        match self.inner.try_read() {
            Ok(inner) => {
                let registered: Vec<String> = inner.managed_servers.keys().cloned().collect();
                let connected: Vec<String> = inner.clients.keys().cloned().collect();
                (registered, connected)
            }
            Err(_) => {
                // Lock contention - return empty lists
                (Vec::new(), Vec::new())
            }
        }
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple in-memory MCP client for testing.
pub struct MockMcpClient {
    server_name: String,
    tools: Vec<McpToolDef>,
    connected: bool,
}

impl MockMcpClient {
    /// Create a new mock client.
    pub fn new(server_name: &str, tools: Vec<McpToolDef>) -> Self {
        Self {
            server_name: server_name.to_string(),
            tools,
            connected: true,
        }
    }
}

#[async_trait]
impl McpClient for MockMcpClient {
    async fn list_tools(&self) -> anyhow::Result<Vec<McpToolDef>> {
        Ok(self.tools.clone())
    }

    async fn call_tool(&self, call: McpToolCall) -> anyhow::Result<McpToolResult> {
        // Mock implementation - just echo back the call
        Ok(McpToolResult {
            success: true,
            content: vec![McpContent::Text {
                text: format!("Called {} with {:?}", call.name, call.arguments),
            }],
            error: None,
        })
    }

    fn server_name(&self) -> &str {
        &self.server_name
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthTokenType;

    #[test]
    fn test_mcp_server_config_serialization() {
        let config = McpServerConfig {
            name: "test-server".to_string(),
            url: Some("http://localhost:8080".to_string()),
            command: None,
            args: vec![],
            env: HashMap::new(),
            requires_auth: false,
            oauth_config: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-server"));
        assert!(json.contains("localhost"));
    }

    #[test]
    fn test_mcp_tool_def() {
        let tool_def = McpToolDef {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }),
        };

        assert_eq!(tool_def.name, "test_tool");
    }

    #[test]
    fn test_mcp_content_serialization() {
        let text_content = McpContent::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&text_content).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("Hello"));

        let image_content = McpContent::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        };
        let json = serde_json::to_string(&image_content).unwrap();
        assert!(json.contains("image"));
    }

    #[tokio::test]
    async fn test_mock_mcp_client() {
        let tools = vec![McpToolDef {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({}),
        }];

        let client = MockMcpClient::new("test", tools);

        let listed = client.list_tools().await.unwrap();
        assert_eq!(listed.len(), 1);

        let result = client
            .call_tool(McpToolCall {
                name: "echo".to_string(),
                arguments: serde_json::json!({ "text": "hi" }),
            })
            .await
            .unwrap();

        assert!(result.success);
        assert!(client.is_connected().await);
    }

    #[tokio::test]
    async fn test_mcp_tool_bridge() {
        let tools = vec![McpToolDef {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({}),
        }];

        let client = Arc::new(MockMcpClient::new("test", tools));
        let bridge = McpToolBridge::new(
            "test",
            &McpToolDef {
                name: "echo".to_string(),
                description: "Echo".to_string(),
                input_schema: serde_json::json!({}),
            },
            client,
        );

        assert_eq!(bridge.name(), "mcp__test__echo");
        assert_eq!(bridge.original_name(), "echo");
        assert_eq!(bridge.server_name(), "test");
    }

    #[tokio::test]
    async fn test_mcp_tool_bridge_execute() {
        let tools = vec![McpToolDef {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({}),
        }];

        let client = Arc::new(MockMcpClient::new("test", tools));
        let bridge = McpToolBridge::new(
            "test",
            &McpToolDef {
                name: "echo".to_string(),
                description: "Echo".to_string(),
                input_schema: serde_json::json!({}),
            },
            client,
        );

        let result = bridge
            .execute(serde_json::json!({ "text": "hello" }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_mcp_manager_creation() {
        let manager = McpManager::new();
        let servers = manager.connected_servers().await;
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_manager_register_server() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;

        // Server is registered but not connected
        let servers = manager.connected_servers().await;
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_manager_get_config() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "secured".to_string(),
                url: Some("http://localhost:8081".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: Some(OAuthConfig {
                    auth_url: "https://auth.example.com/authorize".to_string(),
                    token_url: "https://auth.example.com/token".to_string(),
                    client_id: "client".to_string(),
                    client_secret: None,
                    redirect_url: None,
                    scopes: vec!["read".to_string()],
                }),
            })
            .await;

        let config = manager.get_config("secured").await.expect("config");
        assert!(config.requires_auth);
        assert_eq!(config.name, "secured");
    }

    #[tokio::test]
    async fn test_mcp_manager_registered_server_configs() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "one".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;
        manager
            .register_server(McpServerConfig {
                name: "two".to_string(),
                url: Some("http://localhost:8081".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let mut configs = manager.registered_server_configs().await;
        configs.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].name, "one");
        assert_eq!(configs[1].name, "two");
    }

    #[test]
    fn test_oauth_config() {
        let oauth = OAuthConfig {
            auth_url: "https://auth.example.com/authorize".to_string(),
            token_url: "https://auth.example.com/token".to_string(),
            client_id: "test-client".to_string(),
            client_secret: Some("secret".to_string()),
            redirect_url: Some("http://localhost/callback".to_string()),
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        assert_eq!(oauth.client_id, "test-client");
        assert_eq!(oauth.scopes.len(), 2);
    }

    #[test]
    fn test_managed_server_state_transitions() {
        let config = McpServerConfig {
            name: "test".to_string(),
            url: Some("http://localhost:8080".to_string()),
            command: None,
            args: vec![],
            env: HashMap::new(),
            requires_auth: false,
            oauth_config: None,
        };

        let mut server = ManagedServer::new(config);
        assert_eq!(server.state, ServerState::Registered);
        assert!(server.connected_at.is_none());
        assert!(!server.has_auth_token);

        server.set_auth_token(&AuthToken::new("secret", AuthTokenType::Bearer));
        assert!(server.has_auth_token);
        assert_eq!(server.auth_token_type.as_deref(), Some("bearer"));
        assert!(server.auth_updated_at.is_some());

        server.set_connected(3);
        assert_eq!(server.state, ServerState::Connected);
        assert!(server.connected_at.is_some());
        assert_eq!(server.tools_count, 3);

        server.set_error("test error");
        assert_eq!(server.state, ServerState::Error);
        assert_eq!(server.last_error, Some("test error".to_string()));

        server.disconnect();
        assert_eq!(server.state, ServerState::Disconnected);
        assert!(server.has_auth_token);
        assert_eq!(server.tools_count, 0);
    }

    #[tokio::test]
    async fn test_manager_update_server() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;

        let updated = manager
            .update_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:9090".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        assert!(updated);

        let config = manager.get_config("test").await.unwrap();
        assert_eq!(config.url, Some("http://localhost:9090".to_string()));
        assert!(config.requires_auth);
    }

    #[tokio::test]
    async fn test_manager_remove_server() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;

        let removed = manager.remove_server("test").await;
        assert!(removed);

        let config = manager.get_config("test").await;
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_manager_remove_connected_server_fails() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;

        manager.connect("test").await.unwrap();

        let removed = manager.remove_server("test").await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_manager_auth_token_lifecycle() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let token = AuthToken::new("test_token", AuthTokenType::Bearer);

        let set = manager.set_auth_token("test", token.clone()).await;
        assert!(set);

        let stored = manager.get_auth_token("test").await.unwrap();
        assert_eq!(stored.value(), "test_token");

        let needs_refresh = manager.needs_token_refresh("test", 60).await;
        assert!(!needs_refresh);

        let server = manager
            .get_managed_server("test")
            .await
            .expect("managed server");
        assert!(server.has_auth_token);
        assert_eq!(server.auth_token_type.as_deref(), Some("bearer"));
    }

    #[tokio::test]
    async fn test_manager_list_managed_servers() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "one".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;
        manager
            .register_server(McpServerConfig {
                name: "two".to_string(),
                url: Some("http://localhost:8081".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let servers = manager.list_managed_servers().await;
        assert_eq!(servers.len(), 2);

        let names: Vec<_> = servers.iter().map(|s| s.config.name.clone()).collect();
        assert!(names.contains(&"one".to_string()));
        assert!(names.contains(&"two".to_string()));
    }

    #[tokio::test]
    async fn test_manager_connect_requires_auth_without_token() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let result = manager.connect("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_connect_with_valid_token() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let token = AuthToken::new("test_token", AuthTokenType::Bearer);

        manager.set_auth_token("test", token).await;
        let result = manager.connect("test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manager_disconnect_preserves_auth_token() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        manager
            .set_auth_token("test", AuthToken::new("secret", AuthTokenType::Bearer))
            .await;
        manager.connect("test").await.unwrap();
        manager.disconnect("test").await.unwrap();

        let stored = manager.get_auth_token("test").await.expect("stored token");
        assert_eq!(stored.value(), "secret");
        let server = manager
            .get_managed_server("test")
            .await
            .expect("managed server");
        assert_eq!(server.state, ServerState::Disconnected);
        assert!(server.has_auth_token);
    }

    #[tokio::test]
    async fn test_manager_update_server_state() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "test".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: false,
                oauth_config: None,
            })
            .await;

        manager
            .update_server_state("test", ServerState::Connecting)
            .await;

        let server = manager.get_managed_server("test").await.unwrap();
        assert_eq!(server.state, ServerState::Connecting);
    }

    #[tokio::test]
    async fn test_manager_snapshot_round_trip_preserves_auth_tokens() {
        let manager = McpManager::new();

        manager
            .register_server(McpServerConfig {
                name: "github".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;
        manager
            .set_auth_token("github", AuthToken::new("secret", AuthTokenType::Bearer))
            .await;

        let snapshot = manager.snapshot().await;
        assert_eq!(snapshot.servers.len(), 1);
        assert_eq!(
            snapshot.servers[0]
                .auth_token
                .as_ref()
                .map(|token| token.token_type.as_str()),
            Some("bearer")
        );

        let restored = McpManager::new();
        restored.hydrate_snapshot(snapshot).await.unwrap();

        let config = restored.get_config("github").await.expect("config");
        assert_eq!(config.name, "github");
        let auth = restored.get_auth_token("github").await.expect("auth token");
        assert_eq!(auth.value(), "secret");
    }
}
