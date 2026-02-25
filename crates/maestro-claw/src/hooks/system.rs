//! Hook system for registration and execution

use std::sync::Arc;

use super::{Hook, HookContext, HookError};
use crate::session::Turn;

/// System for managing and executing hooks
#[derive(Debug, Default)]
pub struct HookSystem {
    hooks: Vec<Arc<dyn Hook>>,
}

impl HookSystem {
    /// Create a new empty hook system
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Register a hook
    ///
    /// Hooks are executed in registration order.
    pub fn register(&mut self, hook: Arc<dyn Hook>) {
        self.hooks.push(hook);
    }

    /// Execute all pre-hooks in order (Rec-2: async)
    ///
    /// If any hook returns an Abort error, the chain stops immediately.
    /// If a hook returns a regular error, the error is logged but the chain continues.
    /// Returns the potentially modified turn.
    pub async fn execute_pre(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        let mut current_turn = turn.clone();

        for hook in &self.hooks {
            match hook.pre_execute(context, &current_turn).await {
                Ok(modified_turn) => {
                    current_turn = modified_turn;
                }
                Err(e) if e.is_abort() => {
                    return Err(e);
                }
                Err(e) => {
                    // Log error but continue
                    tracing::warn!("Hook pre_execute error (continuing): {}", e);
                }
            }
        }

        Ok(current_turn)
    }

    /// Execute all post-hooks in order (Rec-2: async)
    ///
    /// If any hook returns an Abort error, the chain stops immediately.
    /// If a hook returns a regular error, the error is logged but the chain continues.
    /// Returns the potentially modified turn.
    pub async fn execute_post(&self, context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        let mut current_turn = turn.clone();

        for hook in &self.hooks {
            match hook.post_execute(context, &current_turn).await {
                Ok(modified_turn) => {
                    current_turn = modified_turn;
                }
                Err(e) if e.is_abort() => {
                    return Err(e);
                }
                Err(e) => {
                    // Log error but continue
                    tracing::warn!("Hook post_execute error (continuing): {}", e);
                }
            }
        }

        Ok(current_turn)
    }

    /// Get the number of registered hooks
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Check if there are no hooks
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[derive(Debug)]
    struct TestHook {
        name: String,
    }

    impl TestHook {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Hook for TestHook {
        fn name(&self) -> &str {
            &self.name
        }

        async fn pre_execute(&self, _context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
            // Add a marker to the content
            let mut modified = turn.clone();
            modified.content = format!("[pre:{}] {}", self.name, modified.content);
            Ok(modified)
        }

        async fn post_execute(&self, _context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
            // Add a marker to the content
            let mut modified = turn.clone();
            modified.content = format!("[post:{}] {}", self.name, modified.content);
            Ok(modified)
        }
    }

    #[test]
    fn test_hook_system_new() {
        let system = HookSystem::new();
        assert!(system.is_empty());
    }

    #[test]
    fn test_hook_system_register() {
        let mut system = HookSystem::new();
        system.register(Arc::new(TestHook::new("hook1")));
        assert_eq!(system.len(), 1);
    }

    #[tokio::test]
    async fn test_hook_system_execute_pre() {
        let mut system = HookSystem::new();
        system.register(Arc::new(TestHook::new("hook1")));
        system.register(Arc::new(TestHook::new("hook2")));

        let context = HookContext::new(
            0,
            10,
            "session".to_string(),
            "thread".to_string(),
            "provider".to_string(),
        );
        let turn = Turn::new(crate::session::TurnRole::User, "test".to_string());

        let result = system.execute_pre(&context, &turn).await.unwrap();
        assert!(result.content.contains("[pre:hook1]"));
        assert!(result.content.contains("[pre:hook2]"));
    }

    #[tokio::test]
    async fn test_hook_system_execute_post() {
        let mut system = HookSystem::new();
        system.register(Arc::new(TestHook::new("hook1")));
        system.register(Arc::new(TestHook::new("hook2")));

        let context = HookContext::new(
            0,
            10,
            "session".to_string(),
            "thread".to_string(),
            "provider".to_string(),
        );
        let turn = Turn::new(crate::session::TurnRole::Assistant, "test".to_string());

        let result = system.execute_post(&context, &turn).await.unwrap();
        assert!(result.content.contains("[post:hook1]"));
        assert!(result.content.contains("[post:hook2]"));
    }
}
