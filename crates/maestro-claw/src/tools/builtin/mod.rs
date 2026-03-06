//! Built-in tools for MaestroClaw
//!
//! This module provides ready-to-use tools:
//! - `CronAddTool`, `CronListTool`, `CronRemoveTool`: Manage scheduled jobs
//! - `ShellTool`: Execute shell commands with safety constraints
//! - `FileTool`: Read/write file operations with path validation
//! - `MemoryTool`: Store/recall operations via Memory trait

pub mod cron_tools;
mod file;
mod memory;
mod shell;

pub use cron_tools::{CronAddTool, CronListTool, CronRemoveTool};
pub use file::{FileTool, FileToolConfig};
pub use memory::{MemoryBackend, MemoryError, MemoryResult, MemoryTool};
pub use shell::{CommandRiskLevel, ShellTool, ShellToolConfig};
