//! Hook context for passing information to hooks

use serde::{Deserialize, Serialize};

/// Context passed to hooks during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// Current turn number (0-indexed)
    pub turn_number: usize,
    /// Maximum turns allowed
    pub max_turns: usize,
    /// Session ID
    pub session_id: String,
    /// Thread ID
    pub thread_id: String,
    /// Provider name being used
    pub provider_name: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl HookContext {
    /// Create a new hook context
    pub fn new(
        turn_number: usize,
        max_turns: usize,
        session_id: String,
        thread_id: String,
        provider_name: String,
    ) -> Self {
        Self {
            turn_number,
            max_turns,
            session_id,
            thread_id,
            provider_name,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if this is the last allowed turn
    ///
    /// Returns true when `max_turns` is zero (no turns allowed) or when
    /// `turn_number` has reached the last allowed index.
    pub fn is_last_turn(&self) -> bool {
        self.max_turns == 0 || self.turn_number >= self.max_turns.saturating_sub(1)
    }

    /// Get remaining turns
    pub fn remaining_turns(&self) -> usize {
        self.max_turns.saturating_sub(self.turn_number + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_context_creation() {
        let ctx = HookContext::new(
            0,
            10,
            "session-1".to_string(),
            "thread-1".to_string(),
            "claude".to_string(),
        );
        assert_eq!(ctx.turn_number, 0);
        assert_eq!(ctx.max_turns, 10);
        assert_eq!(ctx.provider_name, "claude");
    }

    #[test]
    fn test_hook_context_is_last_turn() {
        let ctx = HookContext::new(
            9,
            10,
            "session-1".to_string(),
            "thread-1".to_string(),
            "claude".to_string(),
        );
        assert!(ctx.is_last_turn());
    }

    #[test]
    fn test_hook_context_remaining_turns() {
        let ctx = HookContext::new(
            5,
            10,
            "session-1".to_string(),
            "thread-1".to_string(),
            "claude".to_string(),
        );
        assert_eq!(ctx.remaining_turns(), 4);
    }

    #[test]
    fn test_hook_context_is_last_turn_zero_max() {
        // max_turns = 0 must NOT panic (was usize underflow)
        let ctx = HookContext::new(
            0,
            0,
            "session-1".to_string(),
            "thread-1".to_string(),
            "claude".to_string(),
        );
        assert!(
            ctx.is_last_turn(),
            "zero max_turns should be treated as last turn"
        );
        assert_eq!(ctx.remaining_turns(), 0);
    }

    #[test]
    fn test_hook_context_remaining_turns_zero() {
        let ctx = HookContext::new(
            0,
            0,
            "session-1".to_string(),
            "thread-1".to_string(),
            "claude".to_string(),
        );
        assert_eq!(ctx.remaining_turns(), 0);
    }

    #[test]
    fn test_hook_context_with_metadata() {
        let ctx = HookContext::new(
            0,
            10,
            "session-1".to_string(),
            "thread-1".to_string(),
            "claude".to_string(),
        )
        .with_metadata("key".to_string(), "value".to_string());
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }
}
