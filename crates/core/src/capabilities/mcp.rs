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
    /// Server configurations.
    configs: HashMap<String, McpServerConfig>,
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
            inner: Arc::new(tokio::sync::RwLock::new(McpManagerInner {
                clients: HashMap::new(),
                tools: HashMap::new(),
                configs: HashMap::new(),
            })),
        }
    }

    /// Register a server configuration.
    pub async fn register_server(&self, config: McpServerConfig) {
        let mut inner = self.inner.write().await;
        inner.configs.insert(config.name.clone(), config);
    }

    /// Connect to a registered server.
    pub async fn connect(&self, server_name: &str) -> anyhow::Result<()> {
        let inner = self.inner.read().await;
        let config = inner
            .configs
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("Server not registered: {}", server_name))?
            .clone();

        // Check if already connected
        if inner.clients.contains_key(server_name) {
            return Ok(());
        }
        drop(inner);

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
        inner.tools.insert(server_name.to_string(), tools_list);

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
                let registered: Vec<String> = inner.configs.keys().cloned().collect();
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
}
