//! Maestro LSP to MCP Bridge Library
//!
//! Provides protocol translation between LSP and MCP.
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
//! Note: The LspStdioProxy remains in leindex-core for internal use.

pub mod mcp_bridge;

pub use mcp_bridge::{McpBridge, McpEvent, McpTool};
