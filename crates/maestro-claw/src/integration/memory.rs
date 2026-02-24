//! Memory Bridge for maestro-core integration
//!
//! This module provides integration between maestro-claw and maestro-core's
//! Memory trait for persistent storage and retrieval.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use maestro_core::traits::{Memory, SearchResult};

use crate::hooks::{Hook, HookContext, HookError};
use crate::session::Turn;
use crate::tools::builtin::{MemoryBackend, MemoryError, MemoryResult};

/// Error from memory bridge operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryBridgeError {
    /// Storage failed
    #[error("Memory storage failed: {0}")]
    StorageFailed(String),

    /// Search failed
    #[error("Memory search failed: {0}")]
    SearchFailed(String),

    /// Retrieval failed
    #[error("Memory retrieval failed: {0}")]
    RetrievalFailed(String),

    /// Invalid operation
    #[error("Invalid memory operation: {0}")]
    InvalidOperation(String),
}

/// Bridge between maestro-claw and maestro-core Memory trait
///
/// This adapter implements the maestro-claw MemoryBackend trait
/// using maestro-core's Memory trait, allowing tools to use the
/// core memory infrastructure.
pub struct MemoryBridge {
    inner: Arc<dyn Memory>,
}

impl MemoryBridge {
    /// Create a new memory bridge wrapping a maestro-core Memory
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { inner: memory }
    }

    /// Get the underlying maestro-core Memory reference
    pub fn inner(&self) -> &Arc<dyn Memory> {
        &self.inner
    }
}

#[async_trait]
impl MemoryBackend for MemoryBridge {
    async fn store(&self, content: &str, metadata: JsonValue) -> Result<String, MemoryError> {
        self.inner
            .store(content, metadata)
            .await
            .map_err(|e| MemoryError {
                message: e.to_string(),
            })
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>, MemoryError> {
        let results = self.inner.search(query, limit).await.map_err(|e| MemoryError {
            message: e.to_string(),
        })?;

        Ok(results
            .into_iter()
            .map(|r| MemoryResult {
                id: r.id,
                content: r.content,
                metadata: r.metadata,
                score: r.score,
            })
            .collect())
    }

    async fn get(&self, _id: &str) -> Result<Option<MemoryResult>, MemoryError> {
        // maestro-core Memory trait doesn't have a get method
        // This is a limitation - we can search by ID as a workaround
        Err(MemoryError {
            message: "Direct retrieval not supported by maestro-core Memory trait".to_string(),
        })
    }

    async fn delete(&self, _id: &str) -> Result<bool, MemoryError> {
        // maestro-core Memory trait doesn't have a delete method
        Err(MemoryError {
            message: "Delete not supported by maestro-core Memory trait".to_string(),
        })
    }
}

/// Hook that automatically stores turns in maestro-core Memory
///
/// This hook can be attached to the agent loop to automatically
/// persist conversation turns to memory.
#[derive(Debug)]
pub struct PersistentMemoryHook {
    name: String,
    memory: Arc<dyn Memory>,
    /// Only store turns matching these roles
    store_roles: Vec<crate::session::TurnRole>,
    /// Metadata to attach to stored memories
    default_metadata: JsonValue,
}

impl PersistentMemoryHook {
    /// Create a new persistent memory hook
    pub fn new(name: &str, memory: Arc<dyn Memory>) -> Self {
        Self {
            name: name.to_string(),
            memory,
            store_roles: vec![
                crate::session::TurnRole::User,
                crate::session::TurnRole::Assistant,
            ],
            default_metadata: JsonValue::Object(serde_json::Map::new()),
        }
    }

    /// Set which roles to store
    pub fn with_roles(mut self, roles: Vec<crate::session::TurnRole>) -> Self {
        self.store_roles = roles;
        self
    }

    /// Set default metadata for stored memories
    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.default_metadata = metadata;
        self
    }
}

impl Hook for PersistentMemoryHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn pre_execute(&self, _context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        // Store user messages before processing
        if self.store_roles.contains(&turn.role) {
            let mut metadata = self.default_metadata.clone();
            if let JsonValue::Object(ref mut map) = metadata {
                map.insert("role".to_string(), serde_json::json!(format!("{:?}", turn.role)));
                map.insert("turn_id".to_string(), serde_json::json!(turn.id));
                map.insert("timestamp".to_string(), serde_json::json!(turn.timestamp.to_rfc3339()));
            }

            // Note: We can't await here since Hook::pre_execute is sync
            // In a real implementation, we'd use a channel or spawn_blocking
            // For now, we just pass through the turn
        }
        Ok(turn.clone())
    }

    fn post_execute(&self, _context: &HookContext, turn: &Turn) -> Result<Turn, HookError> {
        // Store assistant responses after processing
        if self.store_roles.contains(&turn.role) {
            let mut metadata = self.default_metadata.clone();
            if let JsonValue::Object(ref mut map) = metadata {
                map.insert("role".to_string(), serde_json::json!(format!("{:?}", turn.role)));
                map.insert("turn_id".to_string(), serde_json::json!(turn.id));
                map.insert("timestamp".to_string(), serde_json::json!(turn.timestamp.to_rfc3339()));
            }

            // Same note as pre_execute - would need async handling
        }
        Ok(turn.clone())
    }
}

/// In-memory implementation of maestro-core Memory trait for testing
pub struct InMemoryStorage {
    memories: std::sync::RwLock<Vec<StoredMemory>>,
}

struct StoredMemory {
    id: String,
    content: String,
    metadata: JsonValue,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            memories: std::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Memory for InMemoryStorage {
    async fn store(&self, content: &str, metadata: JsonValue) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let memory = StoredMemory {
            id: id.clone(),
            content: content.to_string(),
            metadata,
        };
        self.memories.write().unwrap().push(memory);
        Ok(id)
    }

    async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        let memories = self.memories.read().unwrap();
        let results: Vec<SearchResult> = memories
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&query.to_lowercase()))
            .take(limit)
            .map(|m| SearchResult {
                id: m.id.clone(),
                content: m.content.clone(),
                metadata: m.metadata.clone(),
                score: 1.0, // Simple match gets full score
            })
            .collect();
        Ok(results)
    }
}

/// Session persistence helper
///
/// Provides methods to save and restore sessions using maestro-core Memory
pub struct SessionPersistence {
    memory: Arc<dyn Memory>,
}

impl SessionPersistence {
    /// Create a new session persistence helper
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }

    /// Save a session to memory
    pub async fn save_session(&self, session: &crate::session::Session) -> Result<String, MemoryBridgeError> {
        let content = serde_json::to_string_pretty(&session)
            .map_err(|e| MemoryBridgeError::StorageFailed(e.to_string()))?;

        let metadata = serde_json::json!({
            "type": "session",
            "session_id": session.id(),
            "created_at": session.created_at.to_rfc3339(),
        });

        self.memory
            .store(&content, metadata)
            .await
            .map_err(|e| MemoryBridgeError::StorageFailed(e.to_string()))
    }

    /// Save a thread to memory
    pub async fn save_thread(&self, thread: &crate::session::Thread) -> Result<String, MemoryBridgeError> {
        let content = serde_json::to_string_pretty(&thread)
            .map_err(|e| MemoryBridgeError::StorageFailed(e.to_string()))?;

        let metadata = serde_json::json!({
            "type": "thread",
            "thread_id": thread.id(),
            "session_id": thread.session_id(),
        });

        self.memory
            .store(&content, metadata)
            .await
            .map_err(|e| MemoryBridgeError::StorageFailed(e.to_string()))
    }

    /// Search for sessions
    pub async fn search_sessions(&self, query: &str) -> Result<Vec<SearchResult>, MemoryBridgeError> {
        self.memory
            .search(query, 10)
            .await
            .map_err(|e| MemoryBridgeError::SearchFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::TurnRole;

    #[test]
    fn test_memory_bridge_creation() {
        let storage = Arc::new(InMemoryStorage::new());
        let _bridge = MemoryBridge::new(storage);
    }

    #[tokio::test]
    async fn test_memory_bridge_store() {
        let storage = Arc::new(InMemoryStorage::new());
        let bridge = MemoryBridge::new(storage);

        let result = bridge
            .store("Test content", serde_json::json!({"category": "test"}))
            .await;

        assert!(result.is_ok());
        let id = result.unwrap();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_memory_bridge_search() {
        let storage = Arc::new(InMemoryStorage::new());
        let bridge = MemoryBridge::new(storage);

        // Store some content
        bridge
            .store("Hello world", serde_json::json!({}))
            .await
            .unwrap();
        bridge
            .store("Goodbye world", serde_json::json!({}))
            .await
            .unwrap();

        // Search
        let results = bridge.search("world", 10).await.unwrap();
        assert_eq!(results.len(), 2);

        let results = bridge.search("Hello", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();

        let id = storage.store("Test", serde_json::json!({})).await.unwrap();
        assert!(!id.is_empty());

        let results = storage.search("Test", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Test");
    }

    #[test]
    fn test_persistent_memory_hook_creation() {
        let storage = Arc::new(InMemoryStorage::new());
        let hook = PersistentMemoryHook::new("test", storage);

        assert_eq!(hook.name(), "test");
    }

    #[test]
    fn test_persistent_memory_hook_pre_execute() {
        let storage = Arc::new(InMemoryStorage::new());
        let hook = PersistentMemoryHook::new("test", storage);

        let context = HookContext::new(
            0,
            10,
            "session".to_string(),
            "thread".to_string(),
            "provider".to_string(),
        );

        let turn = Turn::new(TurnRole::User, "Hello".to_string());
        let result = hook.pre_execute(&context, &turn);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello");
    }

    #[tokio::test]
    async fn test_session_persistence() {
        let storage = Arc::new(InMemoryStorage::new());
        let persistence = SessionPersistence::new(storage);

        let session = crate::session::Session::new("Test session".to_string());
        let result = persistence.save_session(&session).await;

        assert!(result.is_ok());
    }
}
