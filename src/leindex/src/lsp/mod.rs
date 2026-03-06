//! LSP Integration Module
//!
//! This module provides Language Server Protocol (LSP) integration for Maestro.
//!
//! ## Architecture
//!
//! The LSP integration consists of:
//!
//! - **LSP Manager**: Manages LSP server lifecycle (in memory module)
//! - **MCP Bridge**: Protocol translation (extracted to `maestro-lsp-mcp-bridge` crate)
//!
//! ## MCP Bridge
//!
//! The LSP to MCP bridge has been extracted to the `maestro-lsp-mcp-bridge` crate.
//!
//! ```text
//! ┌─────────────┐     JSON-RPC      ┌─────────────┐     JSON-RPC      ┌─────────────┐
//! │   MCP       │  ─────────────▶   │   Bridge    │  ─────────────▶   │   LSP       │
//! │   Client    │                    │   (crate)   │                    │   Server    │
//! │             │  ◀─────────────    │             │  ◀─────────────    │             │
//! └─────────────┘                    └─────────────┘                    └─────────────┘
//! ```
//!
//! To use the bridge, depend on `maestro-lsp-mcp-bridge`:
//!
//! ```no_run
//! use maestro_lsp_mcp_bridge::McpBridge;
//! use leindex_core::memory::lsp_manager::LspType;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let bridge = McpBridge::new(LspType::Rust, "/path/to/project");
//!     Ok(())
//! }
//! ```

// Stdio proxy remains in leindex-core for internal use by LspManager
pub mod stdio_proxy;
pub use stdio_proxy::*;
