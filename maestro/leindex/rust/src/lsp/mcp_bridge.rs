//! LSP to MCP Bridge
//!
//! Translates Language Server Protocol (LSP) messages to Model Context Protocol (MCP) format.
//!
//! ## Architecture
//!
//! The bridge acts as a bidirectional translator:
//!
//! ```text
//! ┌─────────────┐     JSON-RPC      ┌─────────────┐     JSON-RPC      ┌─────────────┐
//! │   MCP       │  ─────────────▶   │   Bridge    │  ─────────────▶   │   LSP       │
//! │   Client    │                    │             │                    │   Server    │
//! │             │  ◀─────────────    │             │  ◀─────────────    │             │
//! └─────────────┘                    └─────────────┘                    └─────────────┘
//! ```
//!
//! ## Protocol Translation
//!
//! ### MCP Tools (LSP Requests)
//!
//! - `lsp/document_symbols` → `textDocument/documentSymbol`
//! - `lsp/workspace_symbols` → `workspace/symbol`
//! - `lsp/definition` → `textDocument/definition`
//! - `lsp/references` → `textDocument/references`
//! - `lsp/diagnostics` → Pull current diagnostics
//!
//! ### MCP Events (LSP Notifications)
//!
//! - `diagnostics/published` ← `textDocument/publishDiagnostics`
//! - `lsp/initialized` ← `initialized` notification
//! - `lsp/log_message` ← `window/logMessage`

use anyhow::{anyhow, Context, Result};
use lsp_types::{Diagnostic, PublishDiagnosticsParams};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::memory::lsp_manager::LspType;

/// LSP to MCP bridge
///
/// Manages communication between MCP clients and LSP servers, translating
/// between the two protocols.
pub struct McpBridge {
    /// LSP server name (e.g., "rust-analyzer")
    lsp_name: String,
    /// LSP type (Rust, Python, TypeScript)
    lsp_type: LspType,
    /// Project root path
    project_path: String,
    /// Running LSP process
    lsp_process: Option<LspChildProcess>,
    /// Request ID counter for MCP
    mcp_request_id: u64,
    /// Request ID counter for LSP
    lsp_request_id: u64,
    /// Pending LSP requests (internal_id -> (mcp_id, responder))
    pending_requests: RwLock<HashMap<String, (u64, mpsc::Sender<Value>)>>,
    /// Diagnostic cache for pull requests
    diagnostics_cache: RwLock<HashMap<String, Vec<Diagnostic>>>,
}

/// Wrapper for LSP child process with stdio handles
struct LspChildProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl LspChildProcess {
    /// Spawn a new LSP process with stdio communication
    fn spawn(lsp_type: LspType, project_path: &str) -> Result<Self> {
        let binary = lsp_type.binary_name();
        info!("Spawning LSP process: {} for project: {}", binary, project_path);

        let mut cmd = Command::new(binary);
        cmd.current_dir(project_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add default arguments for specific LSPs
        for arg in lsp_type.default_additional_args() {
            cmd.arg(arg);
        }

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn LSP '{}'. Is it installed and in PATH?",
                binary
            )
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open LSP stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to open LSP stdout"))?;

        Ok(Self { child, stdin, stdout })
    }

    /// Send a JSON-RPC message to the LSP
    fn send(&mut self, message: &Value) -> Result<()> {
        let content = serde_json::to_string(message)
            .with_context(|| "Failed to serialize JSON-RPC message")?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        self.stdin
            .write_all(header.as_bytes())
            .context("Failed to write LSP headers")?;
        self.stdin
            .write_all(content.as_bytes())
            .context("Failed to write LSP content")?;
        self.stdin.flush().context("Failed to flush LSP stdin")?;
        debug!("Sent to LSP: {}", content);
        Ok(())
    }

    /// Read a JSON-RPC message from the LSP
    fn read(&mut self) -> Result<Value> {
        let mut reader = io::BufReader::new(&mut self.stdout);
        let mut content_length = None;

        // Read headers
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .context("Failed to read LSP header line")?;
            let line = line.trim();

            if line.is_empty() {
                break; // Empty line indicates end of headers
            }

            if line.to_lowercase().starts_with("content-length:") {
                let len_str = line.split(':').nth(1).unwrap_or("").trim();
                content_length = Some(
                    len_str
                        .parse::<usize>()
                        .context("Failed to parse Content-Length")?,
                );
            }
        }

        let length = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        // Read content
        let mut buffer = vec![0u8; length];
        reader
            .read_exact(&mut buffer)
            .with_context(|| format!("Failed to read LSP content body ({} bytes)", length))?;

        let content = String::from_utf8(buffer)
            .context("LSP response was not valid UTF-8")?;
        let value: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse LSP JSON response: {}", content))?;
        debug!("Received from LSP: {}", content);
        Ok(value)
    }
}

/// MCP tool definition for LSP capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (e.g., "lsp/document_symbols")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    pub input_schema: Value,
}

/// MCP event notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEvent {
    /// Event name (e.g., "diagnostics/published")
    pub name: String,
    /// Event data
    pub data: Value,
}

/// LSP message types
#[derive(Debug, Clone, PartialEq)]
enum LspMessage {
    Request { id: String, method: String, params: Value },
    Response { id: String, result: Option<Value>, error: Option<Value> },
    Notification { method: String, params: Value },
}

impl McpBridge {
    /// Create a new LSP to MCP bridge
    ///
    /// ## Arguments
    ///
    /// - `lsp_type`: Type of LSP server to bridge
    /// - `project_path`: Path to the project root
    pub fn new(lsp_type: LspType, project_path: &str) -> Self {
        Self {
            lsp_name: lsp_type.display_name().to_string(),
            lsp_type,
            project_path: project_path.to_string(),
            lsp_process: None,
            mcp_request_id: 1,
            lsp_request_id: 1,
            pending_requests: RwLock::new(HashMap::new()),
            diagnostics_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Start the LSP process and initialize it
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` when the LSP is initialized and ready
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting LSP bridge for '{}'", self.lsp_name);

        // Spawn the LSP process
        let mut process = LspChildProcess::spawn(self.lsp_type, &self.project_path)?;

        // Create minimal initialize params
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": format!("file://{}", self.project_path),
            "capabilities": {
                "workspace": {
                    "workspaceEdit": false,
                    "didChangeConfiguration": false,
                    "symbol": true,
                    "executeCommand": false,
                    "workspaceFolders": false,
                    "configuration": false
                },
                "textDocument": {
                    "definition": true,
                    "references": true,
                    "documentSymbol": true,
                    "publishDiagnostics": true
                },
                "window": {
                    "workDoneProgress": false
                },
                "general": {
                    "markdown": {
                        "parser": "markdown-it"
                    }
                }
            },
            "clientInfo": {
                "name": "maestro-lsp-mcp-bridge",
                "version": "2.0.0"
            }
        });

        let init_request = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "initialize",
            "params": init_params
        });

        process.send(&init_request).context("Failed to send initialize request")?;

        // Read response (blocking, but OK for initialization)
        let response = process.read().context("Failed to read initialize response")?;

        // Check for error
        if let Some(error) = response.get("error") {
            return Err(anyhow!("LSP initialization failed: {}", error));
        }

        // Send initialized notification
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        process
            .send(&initialized)
            .context("Failed to send initialized notification")?;

        self.lsp_process = Some(process);
        info!("LSP bridge for '{}' is ready", self.lsp_name);

        Ok(())
    }

    /// Get available MCP tools exposed by this bridge
    ///
    /// ## Returns
    ///
    /// Returns a list of MCP tool definitions
    pub fn get_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "lsp/document_symbols".to_string(),
                description: "Get symbols (functions, classes, etc.) in a document".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "Document URI (e.g., file:///path/to/file.rs)"
                        }
                    },
                    "required": ["uri"]
                }),
            },
            McpTool {
                name: "lsp/workspace_symbols".to_string(),
                description: "Search for symbols across the entire workspace".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query string"
                        }
                    },
                    "required": ["query"]
                }),
            },
            McpTool {
                name: "lsp/definition".to_string(),
                description: "Go to definition of a symbol".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "Document URI"
                        },
                        "line": {
                            "type": "integer",
                            "description": "Line number (0-based)"
                        },
                        "character": {
                            "type": "integer",
                            "description": "Character position (0-based)"
                        }
                    },
                    "required": ["uri", "line", "character"]
                }),
            },
            McpTool {
                name: "lsp/references".to_string(),
                description: "Find all references to a symbol".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "Document URI"
                        },
                        "line": {
                            "type": "integer",
                            "description": "Line number (0-based)"
                        },
                        "character": {
                            "type": "integer",
                            "description": "Character position (0-based)"
                        }
                    },
                    "required": ["uri", "line", "character"]
                }),
            },
            McpTool {
                name: "lsp/diagnostics".to_string(),
                description: "Get current diagnostics for a document".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "Document URI"
                        }
                    },
                    "required": ["uri"]
                }),
            },
        ]
    }

    /// Handle an MCP tool call request
    ///
    /// ## Arguments
    ///
    /// - `tool_name`: Name of the tool being called
    /// - `arguments`: Tool arguments
    ///
    /// ## Returns
    ///
    /// Returns the tool result as a JSON value
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        debug!("Calling MCP tool: {} with args: {}", tool_name, arguments);

        match tool_name {
            "lsp/document_symbols" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;

                self.document_symbols(uri).await
            }
            "lsp/workspace_symbols" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'query' argument"))?;

                self.workspace_symbols(query).await
            }
            "lsp/definition" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;
                let line = arguments
                    .get("line")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'line' argument"))? as u32;
                let character = arguments
                    .get("character")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'character' argument"))? as u32;

                self.definition(uri, line, character).await
            }
            "lsp/references" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;
                let line = arguments
                    .get("line")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'line' argument"))? as u32;
                let character = arguments
                    .get("character")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'character' argument"))? as u32;

                self.references(uri, line, character).await
            }
            "lsp/diagnostics" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;

                self.get_diagnostics(uri).await
            }
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    /// Request document symbols from the LSP
    async fn document_symbols(&mut self, uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": {"uri": uri}
        });

        let result = self
            .lsp_request("textDocument/documentSymbol", &params)
            .await?;

        Ok(result)
    }

    /// Request workspace symbols from the LSP
    async fn workspace_symbols(&mut self, query: &str) -> Result<Value> {
        let params = serde_json::json!({
            "query": query
        });

        let result = self.lsp_request("workspace/symbol", &params).await?;

        Ok(result)
    }

    /// Request go-to-definition from the LSP
    async fn definition(&mut self, uri: &str, line: u32, character: u32) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        });

        let result = self.lsp_request("textDocument/definition", &params).await?;

        Ok(result)
    }

    /// Request find-references from the LSP
    async fn references(&mut self, uri: &str, line: u32, character: u32) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": true}
        });

        let result = self
            .lsp_request("textDocument/references", &params)
            .await?;

        Ok(result)
    }

    /// Get cached diagnostics for a document
    async fn get_diagnostics(&self, uri: &str) -> Result<Value> {
        let cache = self.diagnostics_cache.read().await;
        let diagnostics = cache.get(uri).cloned().unwrap_or_default();
        Ok(json!({ "uri": uri, "diagnostics": diagnostics }))
    }

    /// Send a request to the LSP and wait for the response
    async fn lsp_request<T: Serialize>(&mut self, method: &str, params: &T) -> Result<Value> {
        let process = self
            .lsp_process
            .as_mut()
            .ok_or_else(|| anyhow!("LSP process not started"))?;

        let request_id = format!("req-{}", self.lsp_request_id);
        self.lsp_request_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        });

        process.send(&request).context("Failed to send LSP request")?;

        // Read response (blocking for now - could be made async)
        let response = process.read().context("Failed to read LSP response")?;

        // Check for error
        if let Some(error) = response.get("error") {
            return Err(anyhow!("LSP request error: {}", error));
        }

        Ok(response)
    }

    /// Handle an LSP notification (e.g., publishDiagnostics)
    ///
    /// Note: This method returns the events but doesn't update the cache asynchronously
    /// due to lifetime constraints. The cache update should be handled separately.
    pub fn handle_notification(&self, notification: &LspMessage) -> Result<Vec<McpEvent>> {
        if let LspMessage::Notification { method, params } = notification {
            match method.as_str() {
                "textDocument/publishDiagnostics" => {
                    let diag: PublishDiagnosticsParams =
                        serde_json::from_value(params.clone())
                            .context("Failed to parse diagnostics")?;

                    // Note: Cache update needs to be done by the caller with mutable access
                    debug!(
                        "Received diagnostics for URI: {}, count: {}",
                        diag.uri.as_str(),
                        diag.diagnostics.len()
                    );

                    // Convert to MCP event
                    Ok(vec![McpEvent {
                        name: "diagnostics/published".to_string(),
                        data: json!({
                            "uri": diag.uri,
                            "diagnostics": diag.diagnostics,
                        }),
                    }])
                }
                "window/logMessage" => {
                    Ok(vec![McpEvent {
                        name: "lsp/log_message".to_string(),
                        data: params.clone(),
                    }])
                }
                _ => {
                    debug!("Unhandled LSP notification: {}", method);
                    Ok(vec![])
                }
            }
        } else {
            Ok(vec![])
        }
    }

    /// Shutdown the LSP process gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down LSP bridge for '{}'", self.lsp_name);

        if let Some(mut process) = self.lsp_process.take() {
            // Send shutdown request
            let shutdown = json!({
                "jsonrpc": "2.0",
                "id": "shutdown",
                "method": "shutdown"
            });

            let _ = process.send(&shutdown);

            // Send exit notification
            let exit = json!({
                "jsonrpc": "2.0",
                "method": "exit"
            });

            let _ = process.send(&exit);

            // Kill the process
            let _ = process.child.kill();
        }

        Ok(())
    }
}

impl Drop for McpBridge {
    fn drop(&mut self) {
        if self.lsp_process.is_some() {
            warn!("McpBridge dropped without explicit shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool {
            name: "lsp/document_symbols".to_string(),
            description: "Get symbols in a document".to_string(),
            input_schema: json!({"type": "object"}),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("lsp/document_symbols"));
    }

    #[test]
    fn test_mcp_event_serialization() {
        let event = McpEvent {
            name: "diagnostics/published".to_string(),
            data: json!({"uri": "file:///test.rs", "diagnostics": []}),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("diagnostics/published"));
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = McpBridge::new(LspType::Rust, "/tmp/test");
        assert_eq!(bridge.lsp_name, "rust-analyzer");
        assert_eq!(bridge.project_path, "/tmp/test");
    }

    #[test]
    fn test_get_tools() {
        let bridge = McpBridge::new(LspType::Python, "/tmp/test");
        let tools = bridge.get_tools();

        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "lsp/document_symbols"));
        assert!(tools.iter().any(|t| t.name == "lsp/definition"));
    }
}
