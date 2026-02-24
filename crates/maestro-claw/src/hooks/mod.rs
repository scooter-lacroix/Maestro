//! Hooks module for pre/post processing
//!
//! This module provides the hook system for MaesterClaw:
//! - `Hook` trait - Interface for pre/post processing
//! - `HookContext` - Context passed to hooks
//! - `HookSystem` - Hook registration and execution
//! - Built-in hooks for logging and memory

mod context;
mod r#trait;
mod system;

pub mod builtin;

pub use context::HookContext;
pub use r#trait::{Hook, HookError};
pub use system::HookSystem;
