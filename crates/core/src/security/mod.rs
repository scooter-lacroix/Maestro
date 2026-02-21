//! Security module for Maestro Core.
//!
//! Contains:
//! - Leak detection and secret scanning
//! - Approval management for tool execution
//! - Policy hooks for channel-aware security
//! - Secret redaction for logs

pub mod approval;
pub mod base;
pub mod redaction;

// Re-export commonly used items
pub use approval::*;
pub use base::*;
pub use redaction::*;
