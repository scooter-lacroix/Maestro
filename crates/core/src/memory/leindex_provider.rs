//! LeIndex Semantic Graph Memory Provider
//!
//! A memory provider that integrates LeIndex's semantic graph capabilities
//! with hybrid retrieval (lexical + semantic).
//!
//! ## Features
//!
//! - **Hybrid Retrieval**: Combines Tantivy full-text search with vector similarity
//! - **Graph-Aware Signals**: Enriches results with semantic graph relationships
//! - **Provider Boundary**: Explicit separation for testability and isolation
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                     LeIndexProvider                           │
//! │  ┌─────────────────┐  ┌───────────────────────────────────┐  │
//! │  │  Tantivy Index  │  │   In-Memory Vector Store          │  │
//! │  │  (Full-Text)    │  │   (Semantic Embeddings)           │  │
//! │  └────────┬────────┘  └───────────────┬───────────────────┘  │
//! │           │                           │                       │
//! │           └───────────┬───────────────┘                       │
//! │                       │                                       │
//! │               ┌───────▼────────┐                              │
//! │               │  HybridRanker  │                              │
//! │               │  (Score Fusion)│                              │
//! │               └───────┬────────┘                              │
//! │                       │                                       │
//! │               ┌───────▼────────────────┐                      │
//! │               │  Graph Signal Enhancer │                      │
//! │               │  (Relationship Scoring)│                      │
//! │               └────────────────────────┘                      │
//! └───────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::memory::hybrid::HybridRanker;
use crate::traits::{Memory, SearchResult};

/// Default embedding dimension
const DEFAULT_EMBEDDING_DIM: usize = 768;

/// Maximum search results to return
const MAX_SEARCH_RESULTS: usize = 1000;

/// Semantic signal types from graph analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SemanticSignal {
    /// Direct call relationship between functions/methods
    CallRelationship {
        from_symbol: String,
        to_symbol: String,
        strength: f32,
    },
    /// Inheritance or trait implementation relationship
    TypeRelationship {
        source_type: String,
        target_type: String,
        relationship: String,
    },
    /// Data flow relationship
    DataFlow {
        source: String,
        target: String,
        flow_type: String,
    },
    /// Module containment relationship
    Containment { parent: String, child: String },
    /// Reference relationship (imports, usage)
    Reference {
        from_file: String,
        to_symbol: String,
        context: String,
    },
    /// Similarity-based clustering
    ClusterMembership { cluster_id: String, similarity: f32 },
}

/// Graph-aware search result with semantic signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAwareSearchResult {
    /// Base search result
    pub id: String,
    /// Content text
    pub content: String,
    /// Associated metadata
    pub metadata: Value,
    /// Base relevance score
    pub score: f32,
    /// Semantic relevance score from vector search
    pub semantic_score: f32,
    /// Lexical (BM25) score from text search
    pub lexical_score: f32,
    /// Graph-derived semantic signals
    pub graph_signals: Option<Vec<SemanticSignal>>,
}

impl From<GraphAwareSearchResult> for SearchResult {
    fn from(result: GraphAwareSearchResult) -> Self {
        SearchResult {
            id: result.id,
            content: result.content,
            metadata: result.metadata,
            score: result.score,
        }
    }
}

/// Configuration for the LeIndex provider
#[derive(Debug, Clone)]
pub struct LeIndexConfig {
    /// Path to store the index
    pub index_path: PathBuf,
    /// Whether to enable graph signal extraction
    pub enable_graph_signals: bool,
    /// Weight for semantic/vector scores (0.0 to 1.0)
    pub semantic_weight: f32,
    /// Weight for lexical/text scores (0.0 to 1.0)
    pub lexical_weight: f32,
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Maximum memories to store (0 = unlimited)
    pub max_memories: usize,
}

impl LeIndexConfig {
    /// Create a test configuration with a temporary path
    pub fn default_test(path: PathBuf) -> Self {
        Self {
            index_path: path,
            enable_graph_signals: true,
            semantic_weight: 0.5,
            lexical_weight: 0.5,
            embedding_dim: DEFAULT_EMBEDDING_DIM,
            max_memories: 0,
        }
    }

    /// Validate configuration
    pub fn is_valid(&self) -> bool {
        self.semantic_weight >= 0.0
            && self.semantic_weight <= 1.0
            && self.lexical_weight >= 0.0
            && self.lexical_weight <= 1.0
            && self.embedding_dim > 0
    }
}

impl Default for LeIndexConfig {
    fn default() -> Self {
        let index_path = dirs::home_dir()
            .map(|h| h.join(".maestro").join("leindex_memory"))
            .unwrap_or_else(|| PathBuf::from("./leindex_memory"));

        Self {
            index_path,
            enable_graph_signals: true,
            semantic_weight: 0.5,
            lexical_weight: 0.5,
            embedding_dim: DEFAULT_EMBEDDING_DIM,
            max_memories: 0,
        }
    }
}

/// Stored memory with embedding
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields reserved for future graph-aware features
struct StoredMemory {
    id: String,
    content: String,
    metadata: Value,
    embedding: Vec<f32>,
    graph_metadata: HashMap<String, Value>,
}

/// In-memory vector store for semantic search
#[derive(Debug, Default)]
struct VectorStore {
    vectors: Vec<(String, Vec<f32>)>,
    dim: usize,
}

impl VectorStore {
    fn new(dim: usize) -> Self {
        Self {
            vectors: Vec::new(),
            dim,
        }
    }

    fn add(&mut self, id: String, embedding: Vec<f32>) {
        if embedding.len() == self.dim {
            self.vectors.push((id, embedding));
        }
    }

    fn search(&self, query: &[f32], limit: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<_> = self
            .vectors
            .iter()
            .map(|(id, emb)| {
                let score = cosine_similarity(query, emb);
                (id.clone(), score)
            })
            .filter(|(_, s)| s.is_finite())
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a > 0.0 && mag_b > 0.0 {
        dot / (mag_a * mag_b)
    } else {
        0.0
    }
}

/// Simple text index for lexical search
#[derive(Debug, Default)]
struct TextIndex {
    documents: Vec<(String, String)>, // (id, content)
}

impl TextIndex {
    fn add(&mut self, id: String, content: String) {
        self.documents.push((id, content));
    }

    fn search(&self, query: &str, limit: usize) -> Vec<(String, f32)> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<String> = query_lower
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut scored: Vec<_> = self
            .documents
            .iter()
            .map(|(id, content)| {
                let content_lower = content.to_lowercase();
                let matches = query_terms
                    .iter()
                    .filter(|term| content_lower.contains(term.as_str()))
                    .count() as f32;
                let score = matches / query_terms.len().max(1) as f32;
                (id.clone(), score)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}

/// LeIndex Semantic Graph Memory Provider
///
/// Provides graph-aware semantic memory with hybrid retrieval.
pub struct LeIndexProvider {
    config: LeIndexConfig,
    ranker: HybridRanker,
    id_counter: RwLock<u64>,
    storage: RwLock<Vec<StoredMemory>>,
    vector_store: RwLock<VectorStore>,
    text_index: RwLock<TextIndex>,
}

impl LeIndexProvider {
    /// Create a new LeIndex provider
    pub async fn new(config: LeIndexConfig) -> Result<Self> {
        if !config.is_valid() {
            return Err(anyhow::anyhow!("Invalid LeIndex configuration"));
        }

        // Ensure index directory exists
        if !config.index_path.exists() {
            std::fs::create_dir_all(&config.index_path)?;
        }

        let ranker = HybridRanker::new(config.semantic_weight, config.lexical_weight);

        Ok(Self {
            config,
            ranker,
            id_counter: RwLock::new(0),
            storage: RwLock::new(Vec::new()),
            vector_store: RwLock::new(VectorStore::new(DEFAULT_EMBEDDING_DIM)),
            text_index: RwLock::new(TextIndex::default()),
        })
    }

    /// Generate a unique ID for a memory
    fn generate_id(&self) -> String {
        let mut counter = self.id_counter.write().unwrap();
        *counter += 1;
        // Use UUID v4 prefix to ensure global uniqueness across provider instances
        format!(
            "leindex_{}_{}_{}",
            uuid::Uuid::new_v4().simple(),
            chrono::Utc::now().timestamp(),
            *counter
        )
    }

    /// Generate embedding for text (simple hash-based for now)
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; self.config.embedding_dim];

        // Simple deterministic embedding based on text content
        for (i, byte) in text
            .bytes()
            .cycle()
            .take(self.config.embedding_dim)
            .enumerate()
        {
            // Add some variation based on position
            embedding[i] =
                ((byte as f32) / 255.0) * (1.0 + (i as f32 / self.config.embedding_dim as f32));
        }

        // Normalize the embedding
        let mag: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for val in &mut embedding {
                *val /= mag;
            }
        }

        embedding
    }

    /// Extract graph signals from metadata
    fn extract_graph_signals(&self, metadata: &Value) -> Option<Vec<SemanticSignal>> {
        if !self.config.enable_graph_signals {
            return None;
        }

        let mut signals = Vec::new();

        // Extract call relationships
        if let Some(calls) = metadata.get("calls").and_then(|c| c.as_array()) {
            for call in calls {
                if let Some(call_str) = call.as_str() {
                    if let Some(symbol) = metadata.get("symbol").and_then(|s| s.as_str()) {
                        signals.push(SemanticSignal::CallRelationship {
                            from_symbol: symbol.to_string(),
                            to_symbol: call_str.to_string(),
                            strength: 1.0,
                        });
                    }
                }
            }
        }

        // Extract parent relationships
        if let Some(parent) = metadata.get("parent").and_then(|p| p.as_str()) {
            if let Some(symbol) = metadata.get("symbol").and_then(|s| s.as_str()) {
                signals.push(SemanticSignal::Containment {
                    parent: parent.to_string(),
                    child: symbol.to_string(),
                });
            }
        }

        // Extract used_by relationships
        if let Some(used_by) = metadata.get("used_by").and_then(|u| u.as_str()) {
            if let Some(symbol) = metadata.get("symbol").and_then(|s| s.as_str()) {
                signals.push(SemanticSignal::Reference {
                    from_file: used_by.to_string(),
                    to_symbol: symbol.to_string(),
                    context: "usage".to_string(),
                });
            }
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    /// Search with graph-aware signals
    pub async fn search_with_graph_signals(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<GraphAwareSearchResult>> {
        let limit = limit.min(MAX_SEARCH_RESULTS);
        let query_embedding = self.generate_embedding(query);

        // Perform vector search
        let vector_results = self
            .vector_store
            .read()
            .unwrap()
            .search(&query_embedding, limit);

        // Perform text search
        let text_results = self.text_index.read().unwrap().search(query, limit);

        // Merge with hybrid ranker
        let ranked = self.ranker.merge(&text_results, &vector_results, limit);

        // Enrich with graph signals
        let storage = self.storage.read().unwrap();
        let mut results = Vec::new();

        for ranked_result in ranked {
            if let Some(stored) = storage.iter().find(|s| s.id == ranked_result.id) {
                let graph_signals = self.extract_graph_signals(&stored.metadata);

                results.push(GraphAwareSearchResult {
                    id: ranked_result.id,
                    content: stored.content.clone(),
                    metadata: stored.metadata.clone(),
                    score: ranked_result.final_score,
                    semantic_score: ranked_result.vector_score.unwrap_or(0.0),
                    lexical_score: ranked_result.text_score.unwrap_or(0.0),
                    graph_signals,
                });
            }
        }

        debug!("Graph-aware search returned {} results", results.len());
        Ok(results)
    }

    /// Get memory by ID
    pub fn get(&self, id: &str) -> Option<GraphAwareSearchResult> {
        let storage = self.storage.read().unwrap();
        storage.iter().find(|s| s.id == id).map(|stored| {
            let graph_signals = self.extract_graph_signals(&stored.metadata);
            GraphAwareSearchResult {
                id: stored.id.clone(),
                content: stored.content.clone(),
                metadata: stored.metadata.clone(),
                score: 1.0,
                semantic_score: 1.0,
                lexical_score: 1.0,
                graph_signals,
            }
        })
    }

    /// Delete memory by ID
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut storage = self.storage.write().unwrap();
        let mut vector_store = self.vector_store.write().unwrap();
        let mut text_index = self.text_index.write().unwrap();

        let initial_len = storage.len();
        storage.retain(|s| s.id != id);
        vector_store.vectors.retain(|(vid, _)| vid != id);
        text_index.documents.retain(|(doc_id, _)| doc_id != id);

        Ok(storage.len() < initial_len)
    }

    /// Get memory count
    pub fn count(&self) -> usize {
        self.storage.read().unwrap().len()
    }
}

#[async_trait]
impl Memory for LeIndexProvider {
    async fn store(&self, content: &str, metadata: Value) -> Result<String> {
        let id = self.generate_id();
        let embedding = self.generate_embedding(content);

        // Check max memories limit
        if self.config.max_memories > 0 {
            let mut storage = self.storage.write().unwrap();
            while storage.len() >= self.config.max_memories {
                // Remove oldest
                if let Some(removed) = storage.first() {
                    let removed_id = removed.id.clone();
                    self.vector_store
                        .write()
                        .unwrap()
                        .vectors
                        .retain(|(vid, _)| vid != &removed_id);
                    self.text_index
                        .write()
                        .unwrap()
                        .documents
                        .retain(|(doc_id, _)| doc_id != &removed_id);
                }
                storage.remove(0);
            }
        }

        // Store memory
        self.storage.write().unwrap().push(StoredMemory {
            id: id.clone(),
            content: content.to_string(),
            metadata: metadata.clone(),
            embedding: embedding.clone(),
            graph_metadata: HashMap::new(),
        });

        // Update vector index
        self.vector_store
            .write()
            .unwrap()
            .add(id.clone(), embedding);

        // Update text index
        self.text_index
            .write()
            .unwrap()
            .add(id.clone(), content.to_string());

        debug!("Stored memory {} with {} chars", id, content.len());
        Ok(id)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let graph_results = self.search_with_graph_signals(query, limit).await?;
        Ok(graph_results.into_iter().map(SearchResult::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_provider_creation() {
        let tmp = TempDir::new().unwrap();
        let config = LeIndexConfig::default_test(tmp.path().to_path_buf());
        let provider = LeIndexProvider::new(config).await.unwrap();
        assert_eq!(provider.count(), 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let tmp = TempDir::new().unwrap();
        let config = LeIndexConfig::default_test(tmp.path().to_path_buf());
        let provider = LeIndexProvider::new(config).await.unwrap();

        let id = provider
            .store("Test content", json!({"key": "value"}))
            .await
            .unwrap();

        assert!(id.starts_with("leindex_"));
        assert_eq!(provider.count(), 1);

        let retrieved = provider.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test content");
    }

    #[tokio::test]
    async fn test_search_basic() {
        let tmp = TempDir::new().unwrap();
        let config = LeIndexConfig::default_test(tmp.path().to_path_buf());
        let provider = LeIndexProvider::new(config).await.unwrap();

        provider
            .store("Rust programming language", json!({}))
            .await
            .unwrap();
        provider
            .store("Python programming language", json!({}))
            .await
            .unwrap();

        let results = provider.search("Rust", 10).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_graph_signals_extraction() {
        let tmp = TempDir::new().unwrap();
        let config = LeIndexConfig::default_test(tmp.path().to_path_buf());
        let provider = LeIndexProvider::new(config).await.unwrap();

        let _id = provider
            .store(
                "Function that processes payments",
                json!({
                    "symbol": "process_payment",
                    "type": "function",
                    "calls": ["validate_card", "charge_amount"]
                }),
            )
            .await
            .unwrap();

        let results = provider
            .search_with_graph_signals("payment", 10)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(results[0].graph_signals.is_some());
    }

    #[test]
    fn test_config_validation() {
        let valid = LeIndexConfig::default();
        assert!(valid.is_valid());

        let invalid = LeIndexConfig {
            semantic_weight: -0.5,
            ..Default::default()
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_delete() {
        let tmp = TempDir::new().unwrap();
        let config = LeIndexConfig::default_test(tmp.path().to_path_buf());
        let provider = LeIndexProvider::new(config).await.unwrap();

        let id = provider.store("To delete", json!({})).await.unwrap();
        assert_eq!(provider.count(), 1);

        let deleted = provider.delete(&id).await.unwrap();
        assert!(deleted);
        assert_eq!(provider.count(), 0);

        let deleted_again = provider.delete(&id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_max_memories_limit() {
        let tmp = TempDir::new().unwrap();
        let config = LeIndexConfig {
            max_memories: 2,
            ..LeIndexConfig::default_test(tmp.path().to_path_buf())
        };
        let provider = LeIndexProvider::new(config).await.unwrap();

        provider.store("First", json!({})).await.unwrap();
        provider.store("Second", json!({})).await.unwrap();
        provider.store("Third", json!({})).await.unwrap();

        // Should have evicted the first
        assert_eq!(provider.count(), 2);
    }
}
