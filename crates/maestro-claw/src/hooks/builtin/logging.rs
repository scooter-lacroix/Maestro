//! Logging hook for debugging and auditing

use std::fmt::Debug;

use crate::hooks::{Hook, HookContext, HookError};
use crate::session::Turn;

/// Hook that logs all pre/post executions
#[derive(Debug)]
pub struct LoggingHook {
    /// Name for this hook instance
    name: String,
}

impl LoggingHook {
    /// Create a new logging hook
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Hook for LoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn pre_execute(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        tracing::info!(
            session_id = %context.session_id,
            thread_id = %context.thread_id,
            turn_number = context.turn_number,
            role = %turn.role,
            content_len = turn.content.len(),
            "[LoggingHook:{}] Pre-execute",
            self.name
        );
        Ok(turn.clone())
    }

    fn post_execute(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        tracing::info!(
            session_id = %context.session_id,
            thread_id = %context.thread_id,
            turn_number = context.turn_number,
            role = %turn.role,
            content_len = turn.content.len(),
            "[LoggingHook:{}] Post-execute",
            self.name
        );
        Ok(turn.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::TurnRole;

    fn create_test_context() -> HookContext {
        HookContext::new(
            0,
            10,
            "test-session".to_string(),
            "test-thread".to_string(),
            "test-provider".to_string(),
        )
    }

    #[test]
    fn test_logging_hook_creation() {
        let hook = LoggingHook::new("test");
        assert_eq!(hook.name(), "test");
    }

    #[test]
    fn test_logging_hook_pre_execute() {
        let hook = LoggingHook::new("test");
        let context = create_test_context();
        let turn = Turn::new(TurnRole::User, "Hello".to_string());

        let result = hook.pre_execute(&context, &turn).unwrap();
        assert_eq!(result.content, "Hello");
    }

    #[test]
    fn test_logging_hook_post_execute() {
        let hook = LoggingHook::new("test");
        let context = create_test_context();
        let turn = Turn::new(TurnRole::Assistant, "Hi there".to_string());

        let result = hook.post_execute(&context, &turn).unwrap();
        assert_eq!(result.content, "Hi there");
    }
}
