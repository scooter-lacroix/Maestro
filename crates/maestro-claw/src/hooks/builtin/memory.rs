//! Memory hook for context retention

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::hooks::{Hook, HookContext, HookError};
use crate::session::Turn;

/// Memory hook for storing conversation context
#[derive(Debug)]
pub struct MemoryHook {
    /// Name for this hook instance
    name: String,
    /// Stored memories (turn_id -> content)
    memories: Arc<Mutex<HashMap<String, String>>>,
}

impl MemoryHook {
    /// Create a new memory hook
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            memories: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get stored memories
    pub fn get_memories(&self) -> HashMap<String, String> {
        // Recover from poisoned lock rather than panicking
        self.memories.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Clear all memories
    pub fn clear(&self) {
        // Recover from poisoned lock rather than panicking
        self.memories.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

#[async_trait]
impl Hook for MemoryHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn pre_execute(&self, _context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        // Store user messages in memory
        if matches!(turn.role, crate::session::TurnRole::User) {
            self.memories
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(turn.id.clone(), turn.content.clone());
        }
        Ok(turn.clone())
    }

    async fn post_execute(&self, _context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        // Store assistant responses in memory
        if matches!(turn.role, crate::session::TurnRole::Assistant) {
            self.memories
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(turn.id.clone(), turn.content.clone());
        }
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
    fn test_memory_hook_creation() {
        let hook = MemoryHook::new("test");
        assert_eq!(hook.name(), "test");
    }

    #[tokio::test]
    async fn test_memory_hook_stores_user_messages() {
        let hook = MemoryHook::new("test");
        let context = create_test_context();
        let turn = Turn::new(TurnRole::User, "Hello".to_string());
        let turn_id = turn.id.clone();

        hook.pre_execute(&context, &turn).await.unwrap();

        let memories = hook.get_memories();
        assert_eq!(memories.get(&turn_id), Some(&"Hello".to_string()));
    }

    #[tokio::test]
    async fn test_memory_hook_stores_assistant_responses() {
        let hook = MemoryHook::new("test");
        let context = create_test_context();
        let turn = Turn::new(TurnRole::Assistant, "Hi there".to_string());
        let turn_id = turn.id.clone();

        hook.post_execute(&context, &turn).await.unwrap();

        let memories = hook.get_memories();
        assert_eq!(memories.get(&turn_id), Some(&"Hi there".to_string()));
    }

    #[tokio::test]
    async fn test_memory_hook_clear() {
        let hook = MemoryHook::new("test");
        let context = create_test_context();
        let turn = Turn::new(TurnRole::User, "Hello".to_string());

        hook.pre_execute(&context, &turn).await.unwrap();
        assert!(!hook.get_memories().is_empty());

        hook.clear();
        assert!(hook.get_memories().is_empty());
    }
}
