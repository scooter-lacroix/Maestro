//! Built-in tools for MaesterClaw
//!
//! This module provides ready-to-use tools:
//! - `ShellTool`: Execute shell commands with safety constraints
//! - `FileTool`: Read/write file operations with path validation
//! - `MemoryTool`: Store/recall operations via Memory trait

mod file;
mod memory;
mod shell;

pub use file::FileTool;
pub use memory::{MemoryTool, MemoryBackend, MemoryError, MemoryResult};
pub use shell::{CommandRiskLevel, ShellTool};
