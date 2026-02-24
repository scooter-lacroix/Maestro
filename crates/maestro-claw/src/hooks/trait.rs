//! Hook trait definition
//!
//! The Hook trait provides an interface for pre/post processing
//! around provider requests.

use std::fmt::Debug;

use super::HookContext;
use crate::session::Turn;

/// A hook for pre/post processing around provider requests
pub trait Hook: Send + Sync + Debug {
    /// Get the hook name
    fn name(&self) -> &str;

    /// Called before a provider request
    ///
    /// Returns a potentially modified turn, or an error to abort.
    fn pre_execute(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError>;

    /// Called after a provider response
    ///
    /// Returns a potentially modified turn, or an error (doesn't abort).
    fn post_execute(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError>;
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
