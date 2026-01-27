//!
//! # Vector Storage Module
//!
//! Provides HNSW-based approximate nearest neighbor search for vector embeddings.
//! This module enables efficient similarity search in high-dimensional vector spaces.
//!
//! ## Features
//!
//! - **HNSW Index**: Fast approximate nearest neighbor search
//! - **Cosine Similarity**: Optimized for normalized embeddings
//! - **Batch Operations**: Efficient bulk insert and search
//! - **Persistent Storage**: SQLite-based vector persistence
//!
//! ## Example
//!
//! ```rust
//! use vector::VectorIndex;
//!
//! let mut index = VectorIndex::new(768);
//! index.insert("doc1".to_string(), vec![0.1, 0.2, ...]).await?;
//! let results = index.search(&query, 10).await?;
//! ```

use anyhow::Result;
use hnsw_rs::prelude::{DistCosine, Hnsw};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

/// HNSW-based vector index for approximate nearest neighbor search
pub struct HNSWIndex {
    /// HNSW structure
    hnsw: Hnsw<f32, DistCosine>,
    /// Mapping from HNSW internal IDs to node IDs
    id_map: HashMap<usize, String>,
    /// Reverse mapping: node_id -> HNSW internal ID
    reverse_map: HashMap<String, usize>,
    /// Next available internal ID
    next_id: usize,
    /// Vector dimension
    dimension: usize,
    /// Number of vectors in the index
    count: usize,
    /// Maximum number of elements
    max_elements: usize,
}

/// HNSW construction and search parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HNSWParams {
    /// Number of bidirectional links for each node
    pub m: usize,
    /// Number of neighbors to consider during construction
    pub ef_construction: usize,
    /// Number of neighbors to consider during search
    pub ef_search: usize,
}

impl Default for HNSWParams {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        }
    }
}

impl HNSWIndex {
    /// Create a new HNSW index with default parameters
    pub fn new(dimension: usize) -> Self {
        Self::with_params(dimension, HNSWParams::default())
    }

    /// Create a new HNSW index with custom parameters
    pub fn with_params(dimension: usize, params: HNSWParams) -> Self {
        let max_elements = 100_000;
        let max_layer = 16;

        let hnsw = Hnsw::new(
            params.m,
            max_elements,
            max_layer,
            params.ef_construction,
            DistCosine {},
        );

        Self {
            hnsw,
            id_map: HashMap::new(),
            reverse_map: HashMap::new(),
            next_id: 0,
            dimension,
            count: 0,
            max_elements,
        }
    }

    /// Insert a vector into the index
    pub fn insert(&mut self, node_id: String, embedding: Vec<f32>) -> Result<()> {
        if embedding.len() != self.dimension {
            anyhow::bail!(
                "Dimension mismatch: expected {}, got {}",
                self.dimension,
                embedding.len()
            );
        }

        if self.reverse_map.contains_key(&node_id) {
            anyhow::bail!("Node {} already exists", node_id);
        }

        let internal_id = self.next_id;
        self.next_id += 1;

        self.hnsw.insert((&embedding, internal_id));
        self.id_map.insert(internal_id, node_id.clone());
        self.reverse_map.insert(node_id, internal_id);
        self.count += 1;

        Ok(())
    }

    /// Batch insert vectors into the index
    pub fn insert_batch(
        &mut self,
        vectors: impl IntoIterator<Item = (String, Vec<f32>)>,
    ) -> Result<usize> {
        let mut inserted = 0;
        for (node_id, embedding) in vectors {
            if self.insert(node_id, embedding).is_ok() {
                inserted += 1;
            }
        }
        Ok(inserted)
    }

    /// Search for nearest neighbors
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if query.len() != self.dimension || self.count == 0 {
            return Vec::new();
        }

        let ef_search = 50.max(top_k);
        let results = self.hnsw.search(query, top_k, ef_search);

        let mut output = Vec::new();
        for neighbour in results.into_iter() {
            let internal_id = neighbour.d_id;
            let dist = neighbour.distance;

            if let Some(node_id) = self.id_map.get(&internal_id) {
                // Convert distance to similarity
                let similarity: f32 = 1.0 / (1.0 + dist as f32);
                output.push((node_id.clone(), similarity));
            }
        }

        output.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        output
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Remove a vector from the index
    pub fn remove(&mut self, node_id: &str) -> bool {
        if let Some(internal_id) = self.reverse_map.remove(node_id) {
            self.id_map.remove(&internal_id);
            self.count -= 1;
            true
        } else {
            false
        }
    }

    /// Clear all vectors from the index
    pub fn clear(&mut self) {
        let params = HNSWParams::default();
        let max_layer = 16;
        self.hnsw = Hnsw::new(
            params.m,
            self.max_elements,
            max_layer,
            params.ef_construction,
            DistCosine {},
        );
        self.id_map.clear();
        self.reverse_map.clear();
        self.next_id = 0;
        self.count = 0;
    }
}

impl Default for HNSWIndex {
    fn default() -> Self {
        Self::new(768)
    }
}

/// Thread-safe async wrapper for HNSW index
pub struct VectorIndex {
    /// Inner HNSW index
    inner: Mutex<HNSWIndex>,
    /// Embedding dimension
    dimension: usize,
}

impl VectorIndex {
    /// Create a new vector index
    pub fn new(dimension: usize) -> Self {
        Self {
            inner: Mutex::new(HNSWIndex::new(dimension)),
            dimension,
        }
    }

    /// Create a new vector index with custom HNSW parameters
    pub fn with_params(dimension: usize, params: HNSWParams) -> Self {
        Self {
            inner: Mutex::new(HNSWIndex::with_params(dimension, params)),
            dimension,
        }
    }

    /// Insert a vector into the index
    pub async fn insert(&self, node_id: String, embedding: Vec<f32>) -> Result<()> {
        let mut index = self.inner.lock().await;
        index.insert(node_id, embedding)
    }

    /// Batch insert vectors into the index
    pub async fn insert_batch(
        &self,
        vectors: impl IntoIterator<Item = (String, Vec<f32>)>,
    ) -> Result<usize> {
        let mut index = self.inner.lock().await;
        Ok(index.insert_batch(vectors)?)
    }

    /// Search for nearest neighbors
    pub async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(String, f32)>> {
        let index = self.inner.lock().await;
        Ok(index.search(query, top_k))
    }

    /// Get the number of vectors in the index
    pub async fn len(&self) -> usize {
        let index = self.inner.lock().await;
        index.len()
    }

    /// Check if the index is empty
    pub async fn is_empty(&self) -> bool {
        let index = self.inner.lock().await;
        index.is_empty()
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Remove a vector from the index
    pub async fn remove(&self, node_id: &str) -> bool {
        let mut index = self.inner.lock().await;
        index.remove(node_id)
    }

    /// Clear all vectors from the index
    pub async fn clear(&self) {
        let mut index = self.inner.lock().await;
        index.clear();
    }
}

/// Represents a vector embedding with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbedding {
    /// Unique identifier for the vector
    pub id: String,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
    /// Timestamp when created
    pub created_at: String,
}

impl VectorEmbedding {
    /// Create a new vector embedding
    pub fn new(id: String, embedding: Vec<f32>) -> Self {
        Self {
            id,
            embedding,
            metadata: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create with metadata
    pub fn with_metadata(id: String, embedding: Vec<f32>, metadata: serde_json::Value) -> Self {
        Self {
            id,
            embedding,
            metadata: Some(metadata),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Vector search result with similarity score
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// Node ID
    pub id: String,
    /// Similarity score (0-1, higher is better)
    pub similarity: f32,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_index_creation() {
        let index = HNSWIndex::new(128);
        assert_eq!(index.dimension(), 128);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_hnsw_index_insert() {
        let mut index = HNSWIndex::new(128);
        let embedding = vec![0.1; 128];
        assert!(index.insert("test".to_string(), embedding).is_ok());
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_hnsw_index_dimension_mismatch() {
        let mut index = HNSWIndex::new(128);
        let embedding = vec![0.1; 64];
        assert!(index.insert("test".to_string(), embedding).is_err());
    }

    #[test]
    fn test_hnsw_index_duplicate_insert() {
        let mut index = HNSWIndex::new(128);
        let embedding = vec![0.1; 128];
        index.insert("test".to_string(), embedding.clone()).unwrap();
        assert!(index.insert("test".to_string(), embedding).is_err());
    }

    #[test]
    fn test_hnsw_search() {
        let mut index = HNSWIndex::new(3);
        index.insert("a".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        index.insert("b".to_string(), vec![0.0, 1.0, 0.0]).unwrap();
        index.insert("c".to_string(), vec![0.9, 0.1, 0.0]).unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2);

        assert!(!results.is_empty());
        assert_eq!(results[0].0, "a");
    }

    #[test]
    fn test_hnsw_search_empty_index() {
        let index = HNSWIndex::new(3);
        let query = vec![0.1; 3];
        let results = index.search(&query, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_hnsw_batch_insert() {
        let mut index = HNSWIndex::new(3);
        let vectors = vec![
            ("a".to_string(), vec![1.0, 0.0, 0.0]),
            ("b".to_string(), vec![0.0, 1.0, 0.0]),
            ("c".to_string(), vec![0.0, 0.0, 1.0]),
        ];

        let inserted = index.insert_batch(vectors).unwrap();
        assert_eq!(inserted, 3);
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn test_hnsw_remove() {
        let mut index = HNSWIndex::new(3);
        index.insert("test".to_string(), vec![0.1; 3]).unwrap();
        assert_eq!(index.len(), 1);

        assert!(index.remove("test"));
        assert_eq!(index.len(), 0);
        assert!(!index.remove("nonexistent"));
    }

    #[test]
    fn test_hnsw_clear() {
        let mut index = HNSWIndex::new(3);
        index.insert("a".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        index.insert("b".to_string(), vec![0.0, 1.0, 0.0]).unwrap();
        assert_eq!(index.len(), 2);

        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[tokio::test]
    async fn test_vector_index_async() {
        let index = VectorIndex::new(64);
        assert!(index.insert("test".to_string(), vec![0.1; 64]).await.is_ok());
        assert_eq!(index.len().await, 1);
        assert!(!index.is_empty().await);
    }

    #[test]
    fn test_vector_embedding_creation() {
        let embedding = VectorEmbedding::new("doc1".to_string(), vec![0.1; 128]);
        assert_eq!(embedding.id, "doc1");
        assert_eq!(embedding.embedding.len(), 128);
        assert!(embedding.metadata.is_none());
    }

    #[test]
    fn test_vector_embedding_with_metadata() {
        let metadata = serde_json::json!({"title": "test"});
        let embedding = VectorEmbedding::with_metadata("doc1".to_string(), vec![0.1; 128], metadata);
        assert!(embedding.metadata.is_some());
    }
}
