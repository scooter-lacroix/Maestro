//! Hook trait definition
//!
//! The Hook trait provides an interface for pre/post processing
//! around provider requests.
//!
//! ## Rec-2: Async-capable hooks
//!
//! The trait is now `async` (via `async_trait`) so implementations can drive
//! async I/O directly — e.g. persisting turns to a remote database — without
//! the CRIT-2 workaround of fire-and-forgetting a `tokio::spawn`.

use std::fmt::Debug;

use async_trait::async_trait;

use super::HookContext;
use crate::session::Turn;

/// A hook for pre/post processing around provider requests (Rec-2: async)
#[async_trait]
pub trait Hook: Send + Sync + Debug {
    /// Get the hook name
    fn name(&self) -> &str;

    /// Called before a provider request
    ///
    /// Returns a potentially modified turn, or an error to abort.
    async fn pre_execute(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError>;

    /// Called after a provider response
    ///
    /// Returns a potentially modified turn, or an error (doesn't abort).
    async fn post_execute(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError>;
}

/// Error from hook execution
#[derive(Debug, Clone, thiserror::Error)]
pub enum HookError {
    /// The hook failed with a message
    #[error("Hook '{name}' failed: {message}")]
    Failed {
        /// Hook name
        name: String,
        /// Error message
        message: String,
    },

    /// The hook requests aborting the agent loop
    #[error("Hook '{name}' requested abort: {reason}")]
    Abort {
        /// Hook name
        name: String,
        /// Reason for abort
        reason: String,
    },
}

impl HookError {
    /// Create a failed error
    pub fn failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Failed {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Create an abort error
    pub fn abort(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Abort {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Check if this is an abort error
    pub fn is_abort(&self) -> bool {
        matches!(self, Self::Abort { .. })
    }
}
