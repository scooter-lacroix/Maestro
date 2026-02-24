//! Memory operations tool for storing and recalling information
//!
//! The MemoryTool provides integration with maestro-core's Memory trait:
//! - Store content with metadata
//! - Recall/search stored memories
//! - Category-based organization
//! - Semantic search support

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

use crate::tools::{Tool, ToolOutput};

/// Memory backend trait (simplified version for maestro-claw)
/// This mirrors maestro_core::traits::Memory but avoids direct dependency
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Store content with metadata, returns memory ID
    async fn store(&self, content: &str, metadata: JsonValue) -> Result<String, MemoryError>;

    /// Search memories by query
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>, MemoryError>;

    /// Get memory by ID
    async fn get(&self, id: &str) -> Result<Option<MemoryResult>, MemoryError>;

    /// Delete memory by ID
    async fn delete(&self, id: &str) -> Result<bool, MemoryError>;
}

/// Memory operation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryError {
    pub message: String,
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MemoryError {}

/// Result from memory search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResult {
    /// Memory ID
    pub id: String,
    /// Memory content
    pub content: String,
    /// Associated metadata
    pub metadata: JsonValue,
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
}

/// Configuration for MemoryTool
#[derive(Debug, Clone)]
pub struct MemoryToolConfig {
    /// Default search limit
    pub default_search_limit: usize,
    /// Maximum content length to store
    pub max_content_length: usize,
    /// Memory categories for organization
    pub categories: Vec<String>,
}

impl Default for MemoryToolConfig {
    fn default() -> Self {
        Self {
            default_search_limit: 10,
            max_content_length: 100_000, // 100KB
            categories: vec![
                "facts".to_string(),
                "preferences".to_string(),
                "instructions".to_string(),
                "context".to_string(),
                "general".to_string(),
            ],
        }
    }
}

/// Memory tool for store/recall operations
pub struct MemoryTool {
    backend: Arc<dyn MemoryBackend>,
    config: MemoryToolConfig,
}

impl MemoryTool {
    /// Create a new MemoryTool with a backend
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        Self {
            backend,
            config: MemoryToolConfig::default(),
        }
    }

    /// Create a new MemoryTool with custom configuration
    pub fn with_config(backend: Arc<dyn MemoryBackend>, config: MemoryToolConfig) -> Self {
        Self { backend, config }
    }

    /// Get the configuration
    pub fn config(&self) -> &MemoryToolConfig {
        &self.config
    }

    /// Store a memory
    async fn store_memory(&self, content: &str, metadata: JsonValue) -> ToolOutput {
        // Validate content length
        if content.len() > self.config.max_content_length {
            return ToolOutput::error(format!(
                "Content too long: {} bytes (max: {})",
                content.len(),
                self.config.max_content_length
            ));
        }

        // Validate category if specified
        if let Some(category) = metadata.get("category").and_then(|c| c.as_str()) {
            if !self.config.categories.contains(&category.to_string()) {
                return ToolOutput::error(format!(
                    "Invalid category: {}. Valid categories: {:?}",
                    category, self.config.categories
                ));
            }
        }

        match self.backend.store(content, metadata.clone()).await {
            Ok(id) => {
                let result = json!({
                    "id": id,
                    "stored": true,
                    "content_length": content.len()
                });
                ToolOutput::success(serde_json::to_string_pretty(&result).unwrap_or_default())
            }
            Err(e) => ToolOutput::error(format!("Failed to store memory: {}", e.message)),
        }
    }

    /// Search memories
    async fn search_memories(&self, query: &str, limit: Option<usize>) -> ToolOutput {
        let limit = limit.unwrap_or(self.config.default_search_limit);

        match self.backend.search(query, limit).await {
            Ok(results) => {
                let output = json!({
                    "query": query,
                    "count": results.len(),
                    "results": results.iter().map(|r| json!({
                        "id": r.id,
                        "content": r.content,
                        "score": r.score,
                        "metadata": r.metadata
                    })).collect::<Vec<_>>()
                });
                ToolOutput::success(serde_json::to_string_pretty(&output).unwrap_or_default())
            }
            Err(e) => ToolOutput::error(format!("Search failed: {}", e.message)),
        }
    }

    /// Get a specific memory by ID
    async fn get_memory(&self, id: &str) -> ToolOutput {
        match self.backend.get(id).await {
            Ok(Some(result)) => {
                let output = json!({
                    "id": result.id,
                    "content": result.content,
                    "score": result.score,
                    "metadata": result.metadata
                });
                ToolOutput::success(serde_json::to_string_pretty(&output).unwrap_or_default())
            }
            Ok(None) => ToolOutput::error(format!("Memory not found: {}", id)),
            Err(e) => ToolOutput::error(format!("Failed to get memory: {}", e.message)),
        }
    }

    /// Delete a memory
    async fn delete_memory(&self, id: &str) -> ToolOutput {
        match self.backend.delete(id).await {
            Ok(true) => ToolOutput::success(format!("Memory {} deleted", id)),
            Ok(false) => ToolOutput::error(format!("Memory not found: {}", id)),
            Err(e) => ToolOutput::error(format!("Failed to delete memory: {}", e.message)),
        }
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Store and recall information using semantic memory. Supports storing facts, preferences, and context with metadata, and searching by relevance."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["store", "search", "get", "delete"],
                    "description": "The memory operation to perform"
                },
                "content": {
                    "type": "string",
                    "description": "Content to store (for store operation)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query (for search operation)"
                },
                "id": {
                    "type": "string",
                    "description": "Memory ID (for get/delete operations)"
                },
                "metadata": {
                    "type": "object",
                    "description": "Metadata to store with content (optional)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results for search (optional)",
                    "minimum": 1,
                    "maximum": 100
                },
                "category": {
                    "type": "string",
                    "description": "Memory category (facts, preferences, instructions, context, general)"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        // Parse operation
        let operation = match arguments.get("operation") {
            Some(v) => match v.as_str() {
                Some(s) => s,
                None => return ToolOutput::error("operation must be a string".to_string()),
            },
            None => return ToolOutput::error("operation argument required".to_string()),
        };

        match operation {
            "store" => {
                let content = match arguments.get("content") {
                    Some(v) => match v.as_str() {
                        Some(s) => s,
                        None => return ToolOutput::error("content must be a string".to_string()),
                    },
                    None => return ToolOutput::error("content required for store operation".to_string()),
                };

                // Build metadata
                let mut metadata = arguments
                    .get("metadata")
                    .cloned()
                    .unwrap_or(json!({}));

                // Add category if specified
                if let Some(category) = arguments.get("category").and_then(|c| c.as_str()) {
                    metadata["category"] = json!(category);
                }

                // Add timestamp
                metadata["timestamp"] = json!(chrono::Utc::now().to_rfc3339());

                self.store_memory(content, metadata).await
            }
            "search" => {
                let query = match arguments.get("query") {
                    Some(v) => match v.as_str() {
                        Some(s) => s,
                        None => return ToolOutput::error("query must be a string".to_string()),
                    },
                    None => return ToolOutput::error("query required for search operation".to_string()),
                };

                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                self.search_memories(query, limit).await
            }
            "get" => {
                let id = match arguments.get("id") {
                    Some(v) => match v.as_str() {
                        Some(s) => s,
                        None => return ToolOutput::error("id must be a string".to_string()),
                    },
                    None => return ToolOutput::error("id required for get operation".to_string()),
                };

                self.get_memory(id).await
            }
            "delete" => {
                let id = match arguments.get("id") {
                    Some(v) => match v.as_str() {
                        Some(s) => s,
                        None => return ToolOutput::error("id must be a string".to_string()),
                    },
                    None => return ToolOutput::error("id required for delete operation".to_string()),
                };

                self.delete_memory(id).await
            }
            _ => ToolOutput::error(format!("Unknown operation: {}", operation)),
        }
    }
}

/// In-memory mock backend for testing
#[cfg(test)]
pub struct MockMemoryBackend {
    memories: std::sync::RwLock<Vec<MemoryResult>>,
}

#[cfg(test)]
impl MockMemoryBackend {
    /// Create a new mock backend
    pub fn new() -> Self {
        Self {
            memories: std::sync::RwLock::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl Default for MockMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[async_trait]
impl MemoryBackend for MockMemoryBackend {
    async fn store(&self, content: &str, metadata: JsonValue) -> Result<String, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();

        let memory = MemoryResult {
            id: id.clone(),
            content: content.to_string(),
            metadata,
            score: 1.0,
        };

        let mut memories = self.memories.write().map_err(|e| MemoryError {
            message: format!("Lock error: {}", e),
        })?;
        memories.push(memory);

        Ok(id)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>, MemoryError> {
        let memories = self.memories.read().map_err(|e| MemoryError {
            message: format!("Lock error: {}", e),
        })?;

        // Simple substring matching for mock
        let results: Vec<MemoryResult> = memories
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&query.to_lowercase()))
            .take(limit)
            .cloned()
            .collect();

        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryResult>, MemoryError> {
        let memories = self.memories.read().map_err(|e| MemoryError {
            message: format!("Lock error: {}", e),
        })?;

        Ok(memories.iter().find(|m| m.id == id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<bool, MemoryError> {
        let mut memories = self.memories.write().map_err(|e| MemoryError {
            message: format!("Lock error: {}", e),
        })?;

        let initial_len = memories.len();
        memories.retain(|m| m.id != id);

        Ok(memories.len() < initial_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tool() -> MemoryTool {
        let backend = Arc::new(MockMemoryBackend::new());
        MemoryTool::new(backend)
    }

    #[test]
    fn test_tool_name_and_description() {
        let tool = create_test_tool();
        assert_eq!(tool.name(), "memory");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_parameters_schema() {
        let tool = create_test_tool();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["operation"]["enum"].is_array());
        assert!(schema["required"].as_array().unwrap().contains(&json!("operation")));
    }

    #[tokio::test]
    async fn test_store_memory() {
        let tool = create_test_tool();

        let output = tool
            .execute(json!({
                "operation": "store",
                "content": "The user prefers dark mode",
                "category": "preferences"
            }))
            .await;

        assert!(!output.is_error);
        assert!(output.content.contains("id"));
        assert!(output.content.contains("stored"));
    }

    #[tokio::test]
    async fn test_store_with_metadata() {
        let tool = create_test_tool();

        let output = tool
            .execute(json!({
                "operation": "store",
                "content": "API endpoint is /api/v1",
                "metadata": {
                    "source": "documentation",
                    "verified": true
                }
            }))
            .await;

        assert!(!output.is_error);
    }

    #[tokio::test]
    async fn test_store_missing_content() {
        let tool = create_test_tool();

        let output = tool.execute(json!({"operation": "store"})).await;
        assert!(output.is_error);
        assert!(output.content.contains("content required"));
    }

    #[tokio::test]
    async fn test_store_invalid_category() {
        let tool = create_test_tool();

        let output = tool
            .execute(json!({
                "operation": "store",
                "content": "Test content",
                "category": "invalid_category"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("Invalid category"));
    }

    #[tokio::test]
    async fn test_search_memories() {
        let tool = create_test_tool();

        // Store some memories
        tool.execute(json!({
            "operation": "store",
            "content": "User prefers TypeScript over JavaScript"
        }))
        .await;

        tool.execute(json!({
            "operation": "store",
            "content": "The database uses PostgreSQL"
        }))
        .await;

        // Search
        let output = tool
            .execute(json!({
                "operation": "search",
                "query": "prefers"
            }))
            .await;

        assert!(!output.is_error);
        assert!(output.content.contains("TypeScript"));
        assert!(!output.content.contains("PostgreSQL"));
    }

    #[tokio::test]
    async fn test_search_with_limit() {
        let tool = create_test_tool();

        // Store multiple memories with similar content
        for i in 0..5 {
            tool.execute(json!({
                "operation": "store",
                "content": format!("Test content number {}", i)
            }))
            .await;
        }

        let output = tool
            .execute(json!({
                "operation": "search",
                "query": "Test content",
                "limit": 2
            }))
            .await;

        assert!(!output.is_error);
        assert!(output.content.contains("count"));
    }

    #[tokio::test]
    async fn test_search_missing_query() {
        let tool = create_test_tool();

        let output = tool.execute(json!({"operation": "search"})).await;
        assert!(output.is_error);
        assert!(output.content.contains("query required"));
    }

    #[tokio::test]
    async fn test_get_memory() {
        let tool = create_test_tool();

        // Store a memory
        let store_output = tool
            .execute(json!({
                "operation": "store",
                "content": "Test memory for retrieval"
            }))
            .await;

        // Extract ID from response
        let parsed: serde_json::Value = serde_json::from_str(&store_output.content).unwrap();
        let id = parsed["id"].as_str().unwrap();

        // Get the memory
        let get_output = tool
            .execute(json!({
                "operation": "get",
                "id": id
            }))
            .await;

        assert!(!get_output.is_error);
        assert!(get_output.content.contains("Test memory for retrieval"));
    }

    #[tokio::test]
    async fn test_get_nonexistent_memory() {
        let tool = create_test_tool();

        let output = tool
            .execute(json!({
                "operation": "get",
                "id": "nonexistent-id"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_delete_memory() {
        let tool = create_test_tool();

        // Store a memory
        let store_output = tool
            .execute(json!({
                "operation": "store",
                "content": "Memory to be deleted"
            }))
            .await;

        let parsed: serde_json::Value = serde_json::from_str(&store_output.content).unwrap();
        let id = parsed["id"].as_str().unwrap();

        // Delete the memory
        let delete_output = tool
            .execute(json!({
                "operation": "delete",
                "id": id
            }))
            .await;

        assert!(!delete_output.is_error);
        assert!(delete_output.content.contains("deleted"));

        // Verify it's gone
        let get_output = tool
            .execute(json!({
                "operation": "get",
                "id": id
            }))
            .await;

        assert!(get_output.is_error);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_memory() {
        let tool = create_test_tool();

        let output = tool
            .execute(json!({
                "operation": "delete",
                "id": "nonexistent-id"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_unknown_operation() {
        let tool = create_test_tool();

        let output = tool.execute(json!({"operation": "invalid"})).await;
        assert!(output.is_error);
        assert!(output.content.contains("Unknown operation"));
    }

    #[tokio::test]
    async fn test_missing_operation() {
        let tool = create_test_tool();

        let output = tool.execute(json!({})).await;
        assert!(output.is_error);
        assert!(output.content.contains("required"));
    }

    #[test]
    fn test_tool_spec_conversion() {
        let tool = create_test_tool();
        let spec = tool.to_spec();

        assert_eq!(spec.name, "memory");
        assert!(!spec.description.is_empty());
        assert!(spec.parameters.is_object());
    }

    #[tokio::test]
    async fn test_content_too_long() {
        let config = MemoryToolConfig {
            max_content_length: 100,
            ..Default::default()
        };
        let backend = Arc::new(MockMemoryBackend::new());
        let tool = MemoryTool::with_config(backend, config);

        let long_content = "x".repeat(200);
        let output = tool
            .execute(json!({
                "operation": "store",
                "content": long_content
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("too long"));
    }
}
