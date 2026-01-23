//! maestro-lsp-mcp-bridge binary
//!
//! Standalone binary that bridges LSP servers to MCP protocol.
//!
//! ## Usage
//!
//! ```bash
//! maestro-lsp-mcp-bridge --lsp-type rust --project-path /path/to/project
//! ```
//!
//! The bridge communicates via stdio using JSON-RPC 2.0 protocol compatible with MCP.
//!
//! ## Protocol
//!
//! ### MCP Tools
//!
//! The bridge exposes the following MCP tools:
//!
//! - `lsp/document_symbols` - Get symbols in a document
//! - `lsp/workspace_symbols` - Search workspace symbols
//! - `lsp/definition` - Go to definition
//! - `lsp/references` - Find references
//! - `lsp/diagnostics` - Get diagnostics for a document
//!
//! ### MCP Events
//!
//! The bridge emits the following MCP events:
//!
//! - `diagnostics/published` - When LSP publishes diagnostics
//! - `lsp/log_message` - When LSP logs a message

use anyhow::{anyhow, Result};
use clap::Parser;
use leindex_analyzers::lsp::McpBridge;
use leindex_analyzers::memory::lsp_manager::LspType;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

/// LSP to MCP Bridge
///
/// Bridges Language Server Protocol (LSP) servers to Model Context Protocol (MCP).
#[derive(Parser, Debug)]
#[command(name = "maestro-lsp-mcp-bridge")]
#[command(author = "Maestro Project")]
#[command(version = "2.0.0")]
#[command(about = "Bridge LSP servers to MCP protocol", long_about = None)]
struct Args {
    /// Type of LSP server to bridge (rust, python, typescript)
    #[arg(long, value_enum)]
    lsp_type: LspTypeCli,

    /// Path to the project root
    #[arg(long)]
    project_path: String,

    /// Enable debug logging
    #[arg(long, default_value = "false")]
    debug: bool,
}

/// CLI wrapper for LspType that implements clap ValueEnum
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum LspTypeCli {
    Rust,
    Python,
    TypeScript,
}

impl From<LspTypeCli> for LspType {
    fn from(value: LspTypeCli) -> Self {
        match value {
            LspTypeCli::Rust => LspType::Rust,
            LspTypeCli::Python => LspType::Python,
            LspTypeCli::TypeScript => LspType::TypeScript,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug {
        "debug"
    } else {
        // Only log errors and warnings by default to avoid interfering with stdio communication
        "warn"
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(log_level.parse()?))
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!(
        "Starting maestro-lsp-mcp-bridge for {} on project: {}",
        match args.lsp_type {
            LspTypeCli::Rust => "rust-analyzer",
            LspTypeCli::Python => "ruff-lsp",
            LspTypeCli::TypeScript => "typescript-language-server",
        },
        args.project_path
    );

    run_bridge(LspType::from(args.lsp_type), &args.project_path).await
}

async fn run_bridge(lsp_type: LspType, project_path: &str) -> Result<()> {
    // Create the bridge
    let bridge = Arc::new(McpBridge::new(lsp_type, project_path));

    // Start LSP process
    let (mut stdin, mut stdout, _child) = bridge.start_lsp_process().await?;

    // Initialize LSP
    bridge.initialize_lsp(&mut stdin, &mut stdout).await?;

    info!("LSP bridge ready, starting main stdio loop");

    // Enter the main communication loop
    let (_stdin, _stdout) = run_stdio_loop(bridge, stdin, stdout).await?;

    // Shutdown gracefully
    // Note: We can't call shutdown on &Arc<McpBridge> since shutdown takes &mut self
    // The child process will be killed on drop due to kill_on_drop(true)
    info!("LSP bridge shutting down (child will be killed on drop)");

    Ok(())
}

/// Main stdio communication loop
///
/// Reads MCP JSON-RPC requests from stdin, processes them, and writes responses to stdout.
///
/// Uses tokio for async I/O to avoid blocking the runtime.
async fn run_stdio_loop(
    bridge: Arc<McpBridge>,
    mut stdin: tokio::process::ChildStdin,
    mut stdout: tokio::process::ChildStdout,
) -> Result<(tokio::process::ChildStdin, tokio::process::ChildStdout)> {
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut stdout_writer = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - client disconnected
                info!("Client disconnected (EOF)");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                debug!("Received: {}", trimmed);

                // Parse JSON-RPC request
                let request: serde_json::Value =
                    serde_json::from_str(trimmed).unwrap_or_else(|e| {
                        warn!("Failed to parse JSON: {}", e);
                        serde_json::Value::Null
                    });

                if request.is_null() {
                    continue;
                }

                // Clone request before moving it
                let request_id = request.get("id").cloned();

                // Process the request
                let response =
                    match process_request(bridge.clone(), &mut stdin, &mut stdout, request).await {
                        Ok(Some(resp)) => resp,
                        Ok(None) => continue, // Notification, no response
                        Err(e) => {
                            error!("Request error: {}", e);
                            json!({
                                "jsonrpc": "2.0",
                                "id": request_id,
                                "error": {
                                    "code": -32603,
                                    "message": e.to_string()
                                }
                            })
                        }
                    };

                // Write response
                let response_str = serde_json::to_string(&response).unwrap_or_default();
                if let Err(e) = stdout_writer.write_all(response_str.as_bytes()).await {
                    error!("Failed to write response: {}", e);
                    break;
                }
                if let Err(e) = stdout_writer.write_all(b"\n").await {
                    error!("Failed to write newline: {}", e);
                    break;
                }
                let _ = stdout_writer.flush().await;
            }
            Err(e) => {
                error!("Failed to read from stdin: {}", e);
                break;
            }
        }
    }

    Ok((stdin, stdout))
}

/// Process an MCP request
///
/// Returns Ok(Some(response)) for requests, Ok(None) for notifications, or Err on failure.
async fn process_request(
    bridge: Arc<McpBridge>,
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut tokio::process::ChildStdout,
    request: serde_json::Value,
) -> Result<Option<serde_json::Value>> {
    let jsonrpc = request.get("jsonrpc").and_then(|v| v.as_str());
    if jsonrpc != Some("2.0") {
        return Err(anyhow!("Invalid JSON-RPC version"));
    }

    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing method"))?;
    let params = request
        .get("params")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    match method {
        // MCP: tools/list
        "tools/list" => {
            let tools = bridge.get_tools();
            Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tools
                }
            })))
        }

        // MCP: tools/call
        "tools/call" => {
            let tool_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing tool name"))?;

            let arguments = params.get("arguments").cloned().unwrap_or_default();

            let result = bridge
                .call_tool(tool_name, arguments, stdin, stdout)
                .await?;

            Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string(&result).unwrap_or_default()
                        }
                    ]
                }
            })))
        }

        // MCP: ping
        "ping" => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }))),

        // MCP: initialize
        "initialize" => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "events": {}
                },
                "serverInfo": {
                    "name": "maestro-lsp-mcp-bridge",
                    "version": "2.0.0"
                }
            }
        }))),

        // MCP: notifications/initialized (notification)
        "notifications/initialized" => {
            info!("MCP client initialized");
            Ok(None)
        }

        // Unknown method
        _ => Err(anyhow!("Unknown method: {}", method)),
    }
}
