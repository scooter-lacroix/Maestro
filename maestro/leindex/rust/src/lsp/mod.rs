//! LSP Integration Module
//!
//! This module provides Language Server Protocol (LSP) integration for Maestro,
//! including the MCP bridge that translates LSP capabilities to MCP tools/events.
//!
//! ## Architecture
//!
//! The LSP integration consists of:
//!
//! - **MCP Bridge**: Translates LSP diagnostics/symbols to MCP protocol
//! - **LSP Manager**: Manages LSP server lifecycle (in memory module)
//! - **Binary Entry**: `maestro-lsp-mcp-bridge` for standalone operation
//!
//! ## Protocol Translation
//!
//! ### LSP → MCP Events
//!
//! LSP diagnostics are translated to MCP events for real-time notifications:
//!
//! ```text
//! LSP PublishDiagnosticsParams → MCP Event "diagnostics/published"
//! {
//!   "uri": "file:///path/to/file.rs",
//!   "diagnostics": [...]
//! }
//! ```
//!
//! ### LSP → MCP Tools
//!
//! LSP capabilities are exposed as MCP tools:
//!
//! - `lsp/document_symbols` - Query symbols in a document
//! - `lsp/workspace_symbols` - Search symbols across workspace
//! - `lsp/definition` - Go to definition
//! - `lsp/references` - Find references
//! - `lsp/diagnostics` - Get current diagnostics
//!
//! ## Usage
//!
//! ```no_run
//! use leindex_analyzers::lsp::McpBridge;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let bridge = McpBridge::new("rust-analyzer", "/path/to/project").await?;
//!     // Bridge communicates via stdio
//!     Ok(())
//! }
//! ```

pub mod mcp_bridge;

pub use mcp_bridge::*;
