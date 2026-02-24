//! Tools module for dynamic tool management
//!
//! This module provides the tool system for MaesterClaw:
//! - `Tool` trait - Standard interface for tools
//! - `ToolRegistry` - Dynamic registration and lookup
//! - `ToolSpec` - Provider-compatible tool specification
//! - `ToolOutput` - Result from tool execution
//! - `builtin` - Ready-to-use tools (Shell, File, Memory)

mod registry;
mod spec;
mod r#trait;

pub mod builtin;

pub use registry::ToolRegistry;
pub use spec::{ToolOutput, ToolSpec};
pub use r#trait::Tool;
