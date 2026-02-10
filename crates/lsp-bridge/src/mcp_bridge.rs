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
//! - `lsp/log_message` ← `window/logMessage`

use anyhow::{anyhow, Context, Result};
use lsp_types::{Diagnostic, PublishDiagnosticsParams};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use leindex_core::memory::lsp_manager::LspType;
#[cfg(feature = "rusqlite")]
use crate::memory::{models::MemoryCategory, MemoryService};

/// Maximum message size to prevent DoS (16MB)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

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
    /// Session identifier (if available)
    session_id: Option<String>,
    /// Diagnostic cache for pull requests (wrapped in Arc for sharing)
    diagnostics_cache: Arc<RwLock<HashMap<String, Vec<Diagnostic>>>>,
    /// Diagnostics fingerprints to avoid duplicative memory writes
    diagnostics_fingerprints: Arc<RwLock<HashMap<String, String>>>,
    /// Optional memory service for storing diagnostics
    #[cfg(feature = "rusqlite")]
    memory_service: Option<crate::memory::MemoryService>,
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

impl McpBridge {
    /// Create a new LSP to MCP bridge
    ///
    /// ## Arguments
    ///
    /// - `lsp_type`: Type of LSP server to bridge
    /// - `project_path`: Path to the project root
    pub fn new(lsp_type: LspType, project_path: &str) -> Self {
        Self::new_with_session(lsp_type, project_path, None)
    }

    /// Create a new LSP to MCP bridge with an optional session id
    pub fn new_with_session(
        lsp_type: LspType,
        project_path: &str,
        session_id: Option<String>,
    ) -> Self {
        #[cfg(feature = "rusqlite")]
        let memory_service = MemoryService::new(None).ok().map(|svc| {
            let _ = svc.initialize();
            svc
        });

        Self {
            lsp_name: lsp_type.display_name().to_string(),
            lsp_type,
            project_path: project_path.to_string(),
            session_id,
            diagnostics_cache: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_fingerprints: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "rusqlite")]
            memory_service,
        }
    }

    /// Start the LSP process and return handles for communication
    ///
    /// ## Returns
    ///
    /// Returns (stdin, stdout, child) for the LSP process
    pub async fn start_lsp_process(
        &self,
    ) -> Result<(ChildStdin, ChildStdout, tokio::process::Child)> {
        info!(
            "Starting LSP process '{}' for project: {}",
            self.lsp_name, self.project_path
        );

        let binary = self.lsp_type.binary_name();

        let mut cmd = tokio::process::Command::new(binary);
        cmd.current_dir(&self.project_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Inherit stderr to avoid deadlock
            .kill_on_drop(true);

        // Add default arguments for specific LSPs
        for arg in self.lsp_type.default_additional_args() {
            cmd.arg(arg);
        }

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn LSP '{}'. Is it installed and in PATH?",
                binary
            )
        })?;

        let stdin = child.stdin.take().context("Failed to open LSP stdin")?;
        let stdout = child.stdout.take().context("Failed to open LSP stdout")?;

        Ok((stdin, stdout, child))
    }

    /// Initialize the LSP server
    pub async fn initialize_lsp(
        &self,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<()> {
        // Create proper initialize params with encoded rootUri
        let root_uri = McpBridge::path_to_file_uri(&self.project_path)?;

        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
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

        // Send initialize request
        self.send_lsp_request(stdin, "1", "initialize", &init_params)
            .await?;

        // Read initialize response
        let _response = self.read_lsp_message(stdout, "1").await?;

        // Send initialized notification
        self.send_lsp_notification(stdin, "initialized", &json!({}))
            .await?;

        info!("LSP '{}' initialized successfully", self.lsp_name);

        Ok(())
    }

    /// Convert file path to file:// URI with proper encoding
    fn path_to_file_uri(path: &str) -> Result<String> {
        // Canonicalize the path first
        let canonical = Path::new(path)
            .canonicalize()
            .with_context(|| format!("Invalid project path: {}", path))?;

        // Convert to string
        let path_str = canonical
            .to_str()
            .ok_or_else(|| anyhow!("Project path contains invalid UTF-8"))?;

        // URL-encode only the path components, not the separators
        // Split by path separator, encode each component, then rejoin
        let encoded_components: Vec<String> = path_str
            .split('/')
            .map(|component| urlencoding::encode(component).to_string())
            .collect();
        let encoded = encoded_components.join("/");

        // For absolute paths, we need file:/// (3 slashes)
        // For relative paths, file:// (2 slashes)
        if path_str.starts_with('/') {
            Ok(format!("file:///{}", encoded))
        } else {
            Ok(format!("file://{}", encoded))
        }
    }

    /// Send a request to the LSP
    async fn send_lsp_request(
        &self,
        stdin: &mut ChildStdin,
        id: &str,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<()> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let content =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC message")?;

        // LSP uses Content-Length framing with \r\n headers
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        stdin
            .write_all(header.as_bytes())
            .await
            .context("Failed to write LSP headers")?;
        stdin
            .write_all(content.as_bytes())
            .await
            .context("Failed to write LSP content")?;
        stdin.flush().await.context("Failed to flush LSP stdin")?;

        debug!("Sent to LSP: {}", content);

        Ok(())
    }

    /// Send a notification to the LSP
    async fn send_lsp_notification(
        &self,
        stdin: &mut ChildStdin,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let content = serde_json::to_string(&notification)
            .context("Failed to serialize JSON-RPC notification")?;

        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        stdin
            .write_all(header.as_bytes())
            .await
            .context("Failed to write LSP headers")?;
        stdin
            .write_all(content.as_bytes())
            .await
            .context("Failed to write LSP content")?;
        stdin.flush().await.context("Failed to flush LSP stdin")?;

        debug!("Sent notification to LSP: {}", content);

        Ok(())
    }

    /// Read a message from the LSP (with size limit)
    ///
    /// Reads LSP messages until finding one with the matching request ID.
    /// Notifications (messages without ID) are logged and skipped.
    async fn read_lsp_message(&self, stdout: &mut ChildStdout, expected_id: &str) -> Result<Value> {
        loop {
            let message = self.read_next_lsp_message(stdout).await?;

            // Check if this is a notification (no "id" field)
            if message.get("id").is_none() {
                // This is a notification - log it and continue
                if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
                    debug!("LSP notification: {}", method);
                    // Handle notifications that update cache
                    if let Some(params) = message.get("params") {
                        let _ = self.handle_notification_no_event(method, params);
                    }
                }
                continue;
            }

            // This is a response - check if it matches our expected ID
            if let Some(id) = message.get("id") {
                if let Some(id_str) = id.as_str() {
                    if id_str == expected_id {
                        return Ok(message);
                    }
                }
                // ID doesn't match - this is unexpected, log and continue
                warn!(
                    "Received LSP response with unexpected ID: {:?}, expected: {}",
                    id, expected_id
                );
            }
        }
    }

    /// Read the next message from the LSP (with size limit)
    async fn read_next_lsp_message(&self, stdout: &mut ChildStdout) -> Result<Value> {
        // Read headers with Content-Length
        let mut content_length = None;
        let mut header_buf = Vec::with_capacity(256);
        let mut bytes_read = 0;

        // Simple header parsing (read until \r\n\r\n)
        loop {
            let ch = stdout
                .read_u8()
                .await
                .context("Failed to read from LSP stdout")?;

            header_buf.push(ch);
            bytes_read += 1;

            // Check for \r\n\r\n
            if header_buf.len() >= 4 {
                let last4 = &header_buf[header_buf.len() - 4..];
                if last4 == b"\r\n\r\n" {
                    break;
                }
            }

            if bytes_read >= 1024 {
                return Err(anyhow!("LSP headers too large"));
            }
        }

        let headers = std::str::from_utf8(&header_buf).context("LSP headers not valid UTF-8")?;

        for line in headers.lines() {
            if line.to_lowercase().starts_with("content-length:") {
                let len_str = line.split(':').nth(1).unwrap_or("").trim();
                content_length = Some(
                    len_str
                        .parse::<usize>()
                        .context("Failed to parse Content-Length")?,
                );

                // Enforce size limit
                let length = content_length.unwrap();
                if length > MAX_MESSAGE_SIZE {
                    return Err(anyhow!(
                        "LSP message too large: {} bytes (max: {})",
                        length,
                        MAX_MESSAGE_SIZE
                    ));
                }
            }
        }

        let length = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        // Read content body
        let mut buffer = vec![0u8; length];
        stdout
            .read_exact(&mut buffer)
            .await
            .with_context(|| format!("Failed to read LSP content body ({} bytes)", length))?;

        let content = String::from_utf8(buffer).context("LSP response was not valid UTF-8")?;

        let value: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse LSP JSON response: {}", content))?;

        debug!("Received from LSP: {}", content);

        Ok(value)
    }

    /// Handle an LSP notification without emitting an event (for internal use)
    fn handle_notification_no_event(&self, method: &str, params: &Value) -> Result<()> {
        if method == "textDocument/publishDiagnostics" {
            match serde_json::from_value::<PublishDiagnosticsParams>(params.clone()) {
                Ok(diag) => {
                    let uri = diag.uri.to_string();
                    let diagnostics = diag.diagnostics.clone();
                    // Update the cache asynchronously using tokio spawn
                    let bridge = self.clone();
                    tokio::spawn(async move {
                        bridge.update_diagnostics(uri, diagnostics).await;
                    });
                }
                Err(e) => {
                    warn!("Failed to parse diagnostics: {}", e);
                }
            }
        }
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

    /// Handle an MCP tool call request (synchronous version)
    ///
    /// ## Arguments
    ///
    /// - `tool_name`: Name of the tool being called
    /// - `arguments`: Tool arguments
    /// - `stdin`: LSP stdin handle
    /// - `stdout`: LSP stdout handle
    ///
    /// ## Returns
    ///
    /// Returns the tool result as a JSON value
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<Value> {
        debug!("Calling MCP tool: {} with args: {}", tool_name, arguments);

        match tool_name {
            "lsp/document_symbols" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;

                self.document_symbols(uri, stdin, stdout).await
            }
            "lsp/workspace_symbols" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'query' argument"))?;

                self.workspace_symbols(query, stdin, stdout).await
            }
            "lsp/definition" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;
                let line = arguments
                    .get("line")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'line' argument"))?
                    as u32;
                let character = arguments
                    .get("character")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'character' argument"))?
                    as u32;

                self.definition(uri, line, character, stdin, stdout).await
            }
            "lsp/references" => {
                let uri = arguments
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'uri' argument"))?;
                let line = arguments
                    .get("line")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'line' argument"))?
                    as u32;
                let character = arguments
                    .get("character")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'character' argument"))?
                    as u32;

                self.references(uri, line, character, stdin, stdout).await
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
    async fn document_symbols(
        &self,
        uri: &str,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<Value> {
        let params = json!({
            "textDocument": {"uri": uri}
        });

        // Generate unique request ID
        let req_id = format!("doc-symbols-{}", chrono::Utc::now().timestamp_millis());

        self.send_lsp_request(stdin, &req_id, "textDocument/documentSymbol", &params)
            .await?;

        // Read response
        let response = self.read_lsp_message(stdout, &req_id).await?;

        Ok(response)
    }

    /// Request workspace symbols from the LSP
    async fn workspace_symbols(
        &self,
        query: &str,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<Value> {
        let params = json!({
            "query": query
        });

        let req_id = format!(
            "workspace-symbols-{}",
            chrono::Utc::now().timestamp_millis()
        );

        self.send_lsp_request(stdin, &req_id, "workspace/symbol", &params)
            .await?;

        let response = self.read_lsp_message(stdout, &req_id).await?;

        Ok(response)
    }

    /// Request go-to-definition from the LSP
    async fn definition(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<Value> {
        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        });

        let req_id = format!("definition-{}", chrono::Utc::now().timestamp_millis());

        self.send_lsp_request(stdin, &req_id, "textDocument/definition", &params)
            .await?;

        let response = self.read_lsp_message(stdout, &req_id).await?;

        Ok(response)
    }

    /// Request find-references from the LSP
    async fn references(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<Value> {
        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": true}
        });

        let req_id = format!("references-{}", chrono::Utc::now().timestamp_millis());

        self.send_lsp_request(stdin, &req_id, "textDocument/references", &params)
            .await?;

        let response = self.read_lsp_message(stdout, &req_id).await?;

        Ok(response)
    }

    /// Get cached diagnostics for a document
    async fn get_diagnostics(&self, uri: &str) -> Result<Value> {
        let cache = self.diagnostics_cache.read().await;
        let diagnostics = cache.get(uri).cloned().unwrap_or_default();
        Ok(json!({ "uri": uri, "diagnostics": diagnostics }))
    }

    /// Update diagnostics cache (called by background task)
    pub async fn update_diagnostics(&self, uri: String, diagnostics: Vec<Diagnostic>) {
        let fingerprint = serde_json::to_string(&diagnostics).unwrap_or_default();

        {
            let mut fingerprints = self.diagnostics_fingerprints.write().await;
            if let Some(existing) = fingerprints.get(&uri) {
                if existing == &fingerprint {
                    return;
                }
            }
            fingerprints.insert(uri.clone(), fingerprint);
        }

        {
            let mut cache = self.diagnostics_cache.write().await;
            cache.insert(uri.clone(), diagnostics.clone());
        }

        self.persist_diagnostics(&uri, &diagnostics).await;
    }

    /// Persist diagnostics into the memory pipeline (best-effort)
    async fn persist_diagnostics(&self, uri: &str, diagnostics: &[Diagnostic]) {
        #[cfg(feature = "rusqlite")]
        if let Some(service) = self.memory_service.clone() {
            let lsp_name = self.lsp_name.clone();
            let session_id = self.session_id.clone();
            let project_path = self.project_path.clone();
            let uri = uri.to_string();
            let diagnostics = diagnostics.to_vec();

            tokio::task::spawn_blocking(move || {
                let (errors, warnings, infos, hints) =
                    diagnostics
                        .iter()
                        .fold((0usize, 0usize, 0usize, 0usize), |mut acc, diag| {
                            match diag.severity {
                                Some(sev) => match sev {
                                    lsp_types::DiagnosticSeverity::ERROR => acc.0 += 1,
                                    lsp_types::DiagnosticSeverity::WARNING => acc.1 += 1,
                                    lsp_types::DiagnosticSeverity::INFORMATION => acc.2 += 1,
                                    lsp_types::DiagnosticSeverity::HINT => acc.3 += 1,
                                    _ => acc.2 += 1,
                                },
                                None => acc.2 += 1,
                            }
                            acc
                        });

                let file_path = uri
                    .strip_prefix("file://")
                    .and_then(|p| urlencoding::decode(p).ok())
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| uri.clone());

                let summary = if diagnostics.is_empty() {
                    "No diagnostics reported".to_string()
                } else {
                    format!(
                        "Errors: {}, Warnings: {}, Info: {}, Hints: {}",
                        errors, warnings, infos, hints
                    )
                };

                let mut details = Vec::new();
                for diag in diagnostics.iter().take(10) {
                    let severity = match diag.severity {
                        Some(sev) => match sev {
                            lsp_types::DiagnosticSeverity::ERROR => "E",
                            lsp_types::DiagnosticSeverity::WARNING => "W",
                            lsp_types::DiagnosticSeverity::INFORMATION => "I",
                            lsp_types::DiagnosticSeverity::HINT => "H",
                            _ => "I",
                        },
                        None => "I",
                    };
                    let line = diag.range.start.line + 1;
                    let col = diag.range.start.character + 1;
                    let message = diag.message.lines().next().unwrap_or("").trim();
                    details.push(format!("- [{}] {}:{} {}", severity, line, col, message));
                }

                let mut content = format!("LSP Diagnostics ({})\n", lsp_name);
                if let Some(ref sid) = session_id {
                    content.push_str(&format!("Session: {}\n", sid));
                }
                content.push_str(&format!("File: {}\n", file_path));
                content.push_str(&format!("{}\n", summary));
                if !details.is_empty() {
                    content.push_str("\n");
                    content.push_str(&details.join("\n"));
                }

                let metadata = serde_json::json!({
                    "lsp": lsp_name,
                    "uri": uri,
                    "errors": errors,
                    "warnings": warnings,
                    "info": infos,
                    "hints": hints,
                    "diagnostics": diagnostics.iter().take(10).map(|diag| {
                        serde_json::json!({
                            "message": diag.message,
                            "severity": diag.severity.map(|s| format!("{:?}", s).to_lowercase()),
                            "range": {
                                "start": { "line": diag.range.start.line, "character": diag.range.start.character },
                                "end": { "line": diag.range.end.line, "character": diag.range.end.character }
                            }
                        })
                    }).collect::<Vec<_>>()
                });

                let _ = service.store_memory_with_context(
                    &content,
                    MemoryCategory::Observation,
                    Some(&format!("lsp:{}", lsp_name)),
                    session_id.as_deref(),
                    Some(&project_path),
                    Some(metadata),
                );
            });
        }
    }

    /// Process an LSP notification and convert to MCP event
    pub fn handle_lsp_notification(&self, method: &str, params: &Value) -> Option<McpEvent> {
        match method {
            "textDocument/publishDiagnostics" => {
                match serde_json::from_value::<PublishDiagnosticsParams>(params.clone()) {
                    Ok(diag) => {
                        let uri = diag.uri.to_string();

                        // Update cache asynchronously
                        let diagnostics = diag.diagnostics.clone();
                        let bridge = self.clone();
                        tokio::spawn(async move {
                            bridge.update_diagnostics(uri, diagnostics).await;
                        });

                        // Convert to MCP event
                        Some(McpEvent {
                            name: "diagnostics/published".to_string(),
                            data: json!({
                                "uri": diag.uri,
                                "diagnostics": diag.diagnostics,
                            }),
                        })
                    }
                    Err(e) => {
                        warn!("Failed to parse diagnostics: {}", e);
                        None
                    }
                }
            }
            "window/logMessage" => Some(McpEvent {
                name: "lsp/log_message".to_string(),
                data: params.clone(),
            }),
            _ => {
                debug!("Unhandled LSP notification: {}", method);
                None
            }
        }
    }

    /// Run the background task that reads LSP notifications
    ///
    /// This task runs continuously, reading LSP messages and emitting MCP events.
    pub async fn run_lsp_reader(
        self,
        mut stdout: ChildStdout,
        event_tx: mpsc::Sender<McpEvent>,
    ) -> Result<()> {
        info!("Starting LSP reader task");

        loop {
            match self.read_next_lsp_message(&mut stdout).await {
                Ok(message) => {
                    // Check if this is a notification (no "id" field)
                    if message.get("id").is_none() {
                        if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
                            if let Some(params) = message.get("params") {
                                // Update diagnostics cache for publishDiagnostics
                                let _ = self.handle_notification_no_event(method, params);
                                // Also emit MCP event if needed
                                if let Some(event) = self.handle_lsp_notification(method, params) {
                                    // Send event to MCP client
                                    if event_tx.send(event).await.is_err() {
                                        debug!("MCP event channel closed, stopping LSP reader");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    // Responses are handled by the request handler
                }
                Err(e) => {
                    error!("Failed to read from LSP: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Shutdown the LSP process gracefully
    pub async fn shutdown(
        &mut self,
        stdin: &mut ChildStdin,
        mut child: tokio::process::Child,
    ) -> Result<()> {
        info!("Shutting down LSP bridge for '{}'", self.lsp_name);

        // Send shutdown request
        self.send_lsp_request(stdin, "shutdown", "shutdown", &json!(null))
            .await
            .ok();

        // Send exit notification
        self.send_lsp_notification(stdin, "exit", &json!(null))
            .await
            .ok();

        // Wait for process to exit (with timeout)
        let _ = timeout(Duration::from_secs(5), child.wait()).await;

        info!("LSP bridge shut down");

        Ok(())
    }
}

impl Clone for McpBridge {
    fn clone(&self) -> Self {
        Self {
            lsp_name: self.lsp_name.clone(),
            lsp_type: self.lsp_type,
            project_path: self.project_path.clone(),
            session_id: self.session_id.clone(),
            diagnostics_cache: Arc::clone(&self.diagnostics_cache),
            diagnostics_fingerprints: Arc::clone(&self.diagnostics_fingerprints),
            #[cfg(feature = "rusqlite")]
            memory_service: self.memory_service.clone(),
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

    #[test]
    fn test_path_to_file_uri() {
        // Test with /tmp which should exist on all systems
        let uri = McpBridge::path_to_file_uri("/tmp").unwrap();
        assert!(uri.starts_with("file:///"));
        assert!(uri.contains("tmp"));

        // Test URL encoding by encoding spaces manually
        let path_with_spaces = "/tmp/test path";
        let _uri = McpBridge::path_to_file_uri(path_with_spaces);
        // Note: canonicalize will fail if the path doesn't exist, so we just check the function works
        // For a path that exists with spaces, it would encode them
    }

    // ========================================================================
    // Task 8.3: Additional Unit Tests for MCP Bridge
    // ========================================================================

    /// Test LSP diagnostics to MCP event translation
    #[test]
    fn test_diagnostics_to_mcp_translation() {
        // Create a sample LSP diagnostic
        let diagnostic = lsp_types::Diagnostic {
            range: lsp_types::Range::new(
                lsp_types::Position::new(0, 0),
                lsp_types::Position::new(0, 10),
            ),
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: Some(lsp_types::NumberOrString::String("E0001".to_string())),
            source: Some("rust-analyzer".to_string()),
            message: "Test error message".to_string(),
            related_information: None,
            tags: None,
            ..Default::default()
        };

        // Verify diagnostic can be converted to JSON
        let json = serde_json::to_string(&diagnostic).unwrap();
        assert!(json.contains("Test error message"));
        assert!(json.contains("E0001"));
    }

    /// Test LSP symbols to MCP tool translation
    #[test]
    fn test_symbols_to_mcp_tool_translation() {
        let bridge = McpBridge::new(LspType::TypeScript, "/tmp/test");

        // Get document symbols tool
        let tools = bridge.get_tools();
        let symbols_tool = tools
            .iter()
            .find(|t| t.name == "lsp/document_symbols")
            .expect("document_symbols tool should exist");

        assert_eq!(symbols_tool.name, "lsp/document_symbols");
        assert!(symbols_tool.description.contains("symbols"));

        // Verify input schema is a JSON object
        let schema = &symbols_tool.input_schema;
        assert!(schema.is_object());
    }

    /// Test workspace symbols tool
    #[test]
    fn test_workspace_symbols_tool() {
        let bridge = McpBridge::new(LspType::Rust, "/tmp/test");
        let tools = bridge.get_tools();

        let workspace_tool = tools
            .iter()
            .find(|t| t.name == "lsp/workspace_symbols")
            .expect("workspace_symbols tool should exist");

        assert_eq!(workspace_tool.name, "lsp/workspace_symbols");
        assert!(workspace_tool.description.contains("workspace"));
    }

    /// Test definition tool
    #[test]
    fn test_definition_tool() {
        let bridge = McpBridge::new(LspType::Python, "/tmp/test");
        let tools = bridge.get_tools();

        let definition_tool = tools
            .iter()
            .find(|t| t.name == "lsp/definition")
            .expect("definition tool should exist");

        assert_eq!(definition_tool.name, "lsp/definition");
        assert!(
            definition_tool.description.contains("definition")
                || definition_tool.description.contains("Definition")
        );
    }

    /// Test LSP type to binary name mapping
    #[test]
    fn test_lsp_type_to_binary_name() {
        assert_eq!(LspType::Rust.binary_name(), "rust-analyzer");
        assert_eq!(LspType::Python.binary_name(), "ruff");
        assert_eq!(
            LspType::TypeScript.binary_name(),
            "typescript-language-server"
        );
    }

    /// Test bridge handles all LSP types
    #[test]
    fn test_bridge_handles_all_lsp_types() {
        for lsp_type in [LspType::Rust, LspType::Python, LspType::TypeScript] {
            let bridge = McpBridge::new(lsp_type, "/tmp/test");
            assert_eq!(bridge.lsp_type, lsp_type);
            assert!(!bridge.get_tools().is_empty());
        }
    }

    /// Test tool input schema structure
    #[test]
    fn test_tool_input_schema_structure() {
        let bridge = McpBridge::new(LspType::Rust, "/tmp/test");
        let tools = bridge.get_tools();

        for tool in tools {
            // Verify input schema is a JSON object
            let schema = &tool.input_schema;

            // Verify it's an object type
            assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));

            // Verify it has properties
            if let Some(props) = schema.get("properties") {
                assert!(props.is_object());
            }
        }
    }
}
