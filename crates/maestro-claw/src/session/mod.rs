//! Session module for conversation state hierarchy
//!
//! This module provides the core data models for tracking AI agent conversations:
//! - `Session`: Top-level container with multiple threads
//! - `Thread`: Conversation branch with ordered turns
//! - `Turn`: Individual message exchange with tool support

mod session;
mod thread;
mod turn;

pub use session::{Session, SessionMetadata};
pub use thread::{Thread, ProviderMessage, ToolCallMessage};
pub use turn::{ToolCall, ToolResult, Turn, TurnRole};
