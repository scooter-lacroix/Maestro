//! Maestro LSP to MCP Bridge Server
//!
//! A standalone server that translates between LSP (Language Server Protocol)
//! and MCP (Model Context Protocol), enabling AI tools to access LSP capabilities.
//!
//! ## Usage
//!
//! ```bash
//! # Start the bridge for a Rust project
//! maestro-lsp-mcp-bridge --lsp rust --project /path/to/project
//!
//! # Start the bridge for a Python project with custom session ID
//! maestro-lsp-mcp-bridge --lsp python --project /path/to/project --session my-session
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     MCP (JSON-RPC)    ┌─────────────┐     LSP (JSON-RPC)    ┌─────────────┐
//! │   MCP       │  ─────────────────▶   │   Bridge    │  ─────────────────▶   │   LSP       │
//! │   Client    │                       │   Server    │                       │   Process   │
//! │   (AI Tool) │  ◀─────────────────    │             │  ◀─────────────────    │             │
//! └─────────────┘                       └─────────────┘                       └─────────────┘
//! ```
//!
//! ## Supported LSPs
//!
//! All LSPs used are written in Rust for performance:
//! - **rust-analyzer**: Rust language server
//! - **ruff**: Python language server (via `ruff server`)
//! - **typescript-language-server**: TypeScript/JavaScript language server

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use leindex_core::memory::lsp_manager::LspType;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use tokio::process::{ChildStdin, ChildStdout};
use tracing::{debug, error, info};

/// Re-export the bridge library for use by the binary
pub use maestro_lsp_mcp_bridge::mcp_bridge::{McpBridge, McpEvent, McpTool};

/// LSP to MCP Bridge Server
#[derive(Parser, Debug)]
#[command(name = "maestro-lsp-mcp-bridge")]
#[command(version = "2.5.0")]
#[command(about = "Maestro LSP to MCP Bridge - Protocol translation for AI tools", long_about = None)]
struct Args {
    /// LSP type to bridge (rust, python, typescript)
    #[arg(short, long, value_enum, default_value = "rust")]
    lsp: LspCliType,

    /// Project root path
    #[arg(short, long)]
    project: String,

    /// Optional session ID for multi-session scenarios
    #[arg(short, long)]
    session: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// LSP type CLI argument
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum LspCliType {
    Rust,
    Python,
    TypeScript,
}

impl From<LspCliType> for LspType {
    fn from(value: LspCliType) -> Self {
        match value {
            LspCliType::Rust => LspType::Rust,
            LspCliType::Python => LspType::Python,
            LspCliType::TypeScript => LspType::TypeScript,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let env_filter = if args.verbose {
        tracing::level_filters::LevelFilter::DEBUG
    } else {
        tracing::level_filters::LevelFilter::INFO
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(env_filter.into())
                .from_env_lossy(),
        )
        .init();

    info!(
        "Starting Maestro LSP-MCP Bridge v2.5.0");
    let lsp_type: LspType = args.lsp.into();
    info!("LSP: {:?}, Project: {}", lsp_type, args.project);

    // Create the bridge
    let bridge = McpBridge::new_with_session(
        lsp_type,
        &args.project,
        args.session.clone(),
    );

    // Run the MCP server (stdio-based)
    if let Err(e) = run_mcp_server(bridge, &args.project, args.session).await {
        error!("Bridge server error: {}", e);
        return Err(e);
    }

    Ok(())
}

/// Run the MCP server using stdio transport
///
/// This implements a simple JSON-RPC server over stdin/stdout that:
/// 1. Responds to "initialize" and "tools/list" requests
/// 2. Forwards tool calls to the LSP
/// 3. Emits LSP notifications as MCP events
async fn run_mcp_server(
    mut bridge: McpBridge,
    _project_path: &str,
    _session_id: Option<String>,
) -> Result<()> {
    let mut initialized = false;

    // Start the LSP process
    let (mut lsp_stdin, mut lsp_stdout, lsp_child): (
        ChildStdin,
        ChildStdout,
        tokio::process::Child,
    ) = bridge
        .start_lsp_process()
        .await
        .context("Failed to start LSP process")?;

    // Initialize the LSP
    bridge
        .initialize_lsp(&mut lsp_stdin, &mut lsp_stdout)
        .await
        .context("Failed to initialize LSP")?;

    info!("LSP initialized, starting MCP server on stdio");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    // Spawn background task to read LSP notifications
    let (_event_tx, event_rx) = tokio::sync::mpsc::channel::<McpEvent>(100);
    let lsp_stdout_reader = tokio::spawn(async move {
        // Note: We can't easily split stdout for concurrent reading in the current architecture
        // For now, LSP notifications will be handled inline with request responses
        drop(event_rx);
        Ok::<(), anyhow::Error>(())
    });

    // Main request loop
    while let Some(Ok(line)) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!("Received MCP request: {}", trimmed);

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse JSON-RPC: {}", e);
                send_error(&mut stdout, -32700, &format!("Parse error: {}", e), None)?;
                continue;
            }
        };

        // Check if this is a notification (no "id" field)
        let is_notification = request.get("id").is_none();

        if is_notification {
            // Handle notifications
            if let Some(method) = request.get("method").and_then(|m| m.as_str()) {
                debug!("Received notification: {}", method);
                if method == "shutdown" {
                    info!("Shutdown notification received");
                    break;
                }
            }
            continue;
        }

        // Extract request fields
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str());
        let params = request.get("params").cloned().unwrap_or(json!(null));

        let result = match method {
            Some("initialize") => {
                initialized = true;
                json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "maestro-lsp-mcp-bridge",
                        "version": "2.5.0"
                    },
                    "capabilities": {
                        "tools": {}
                    }
                })
            }
            Some("tools/list") => {
                let tools = bridge.get_tools();
                json!({
                    "tools": tools.into_iter().map(|tool| {
                        json!({
                            "name": tool.name,
                            "description": tool.description,
                            "inputSchema": tool.input_schema
                        })
                    }).collect::<Vec<_>>()
                })
            }
            Some("tools/call") => {
                if !initialized {
                    send_error(&mut stdout, -32002, "Server not initialized", id.as_ref())?;
                    continue;
                }

                // Extract tool name and arguments
                let tool_name = params
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow!("Missing tool name"))?;

                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                // Call the LSP tool
                match bridge
                    .call_tool(tool_name, arguments, &mut lsp_stdin, &mut lsp_stdout)
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        send_error(&mut stdout, -32603, &format!("Tool error: {}", e), id.as_ref())?;
                        continue;
                    }
                }
            }
            Some(&_) => {
                send_error(
                    &mut stdout,
                    -32601,
                    &format!("Method not found: {:?}", method),
                    id.as_ref(),
                )?;
                continue;
            }
            None => {
                send_error(&mut stdout, -32600, "Invalid Request", id.as_ref())?;
                continue;
            }
        };

        // Send success response
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        let response_str = serde_json::to_string(&response)?;
        writeln!(stdout, "{}", response_str)?;
        stdout.flush()?;
    }

    // Graceful shutdown
    info!("Shutting down bridge");
    lsp_stdout_reader.abort();
    let _ = bridge.shutdown(&mut lsp_stdin, lsp_child).await;

    Ok(())
}

/// Send a JSON-RPC error response
fn send_error(
    stdout: &mut io::Stdout,
    code: i64,
    message: &str,
    id: Option<&Value>,
) -> Result<()> {
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });

    let response_str = serde_json::to_string(&response)?;
    writeln!(stdout, "{}", response_str)?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_cli_type_conversion() {
        assert_eq!(LspType::from(LspCliType::Rust), LspType::Rust);
        assert_eq!(LspType::from(LspCliType::Python), LspType::Python);
        assert_eq!(LspType::from(LspCliType::TypeScript), LspType::TypeScript);
    }

    #[test]
    fn test_args_default() {
        let args = Args::try_parse_from(["maestro-lsp-mcp-bridge", "--project", "/tmp/test"]).unwrap();
        assert_eq!(args.lsp, LspCliType::Rust);
        assert_eq!(args.project, "/tmp/test");
        assert!(args.session.is_none());
        assert!(!args.verbose);
    }
}
