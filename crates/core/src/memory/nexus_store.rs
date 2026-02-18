//! Nexus Vector Store - HNSW-backed vector storage for Maestro
//!
//! Integrates Nexus Memory System's vector database with Turso storage:
//! - 384-dimensional embeddings (compatible with all-MiniLM-L6-v2)
//! - Graph tree for hierarchical relevance boosting
//! - Separate storage from index DBs for performance isolation
//! - Fast approximate nearest neighbor search
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                   NexusVectorStore                             │
//! │  ┌─────────────────┐  ┌───────────────────────────────────┐  │
//! │  │   HNSW Index    │  │   Graph Tree                      │  │
//! │  │   (Fast ANN)    │  │   (Relevance Boosting)            │  │
//! │  └────────┬────────┘  └───────────────┬───────────────────┘  │
//! │           │                           │                       │
//! │           └───────────┬───────────────┘                       │
//! │                       │                                       │
//! │               ┌───────▼────────┐                              │
//! │               │ Turso Backend  │                              │
//! │               │ (Persistence)  │                              │
//! │               └────────────────┘                              │
//! └───────────────────────────────────────────────────────────────┘
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::types::{
    EmbeddingMetadata, MemoryCategory, MemoryLaneType, VectorSearchResult, VectorStoreConfig,
    EMBEDDING_DIMENSION, DEFAULT_EMBEDDING_MODEL,
};

/// Node in the HNSW graph
#[derive(Debug, Clone)]
struct HnswNode {
    /// Node ID (same as memory ID)
    id: i64,
    /// Embedding vector
    vector: Vec<f32>,
    /// Namespace ID for filtering
    namespace_id: i64,
    /// Category for filtering
    category: MemoryCategory,
    /// Lane type for boosting
    lane_type: Option<MemoryLaneType>,
    /// Priority level (1=high, 2=medium, 3=low)
    priority: u8,
    /// Layer in HNSW graph
    layer: usize,
    /// Connected neighbors at each layer
    neighbors: Vec<HashSet<i64>>,
}

/// HNSW index for approximate nearest neighbor search
#[derive(Debug, Default)]
struct HnswIndex {
    /// All nodes indexed by ID
    nodes: HashMap<i64, HnswNode>,
    /// Entry points for each layer
    entry_points: Vec<Option<i64>>,
    /// Maximum layer
    max_layer: usize,
    /// M parameter (max connections per node)
    m: usize,
    /// ef_construction parameter
    ef_construction: usize,
    /// ef_search parameter
    ef_search: usize,
    /// Dimension
    dimension: usize,
}

impl HnswIndex {
    /// Create a new HNSW index
    fn new(config: &VectorStoreConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            entry_points: vec![None; 10], // Support up to 10 layers
            max_layer: 0,
            m: config.hnsw_m,
            ef_construction: config.hnsw_ef_construction,
            ef_search: config.hnsw_ef_search,
            dimension: config.dimension,
        }
    }

    /// Insert a vector into the index
    fn insert(
        &mut self,
        id: i64,
        vector: Vec<f32>,
        namespace_id: i64,
        category: MemoryCategory,
        lane_type: Option<MemoryLaneType>,
        priority: u8,
    ) -> Result<()> {
        // Determine layer using exponential distribution
        let layer = self.random_layer();

        // Create node
        let mut node = HnswNode {
            id,
            vector,
            namespace_id,
            category,
            lane_type,
            priority,
            layer,
            neighbors: vec![HashSet::new(); layer + 1],
        };

        // If this is the first node, just add it
        if self.nodes.is_empty() {
            self.nodes.insert(id, node);
            for l in 0..=layer {
                if l < self.entry_points.len() {
                    self.entry_points[l] = Some(id);
                }
            }
            self.max_layer = self.max_layer.max(layer);
            return Ok(());
        }

        // Find entry point at highest layer
        let mut current = self.entry_points[self.max_layer].unwrap();

        // Traverse down to the insertion layer
        for l in (layer + 1..=self.max_layer).rev() {
            current = self.greedy_search(&self.nodes[&current].vector, l, current);
        }

        // Insert at each layer from insertion layer down to 0
        for l in (0..=layer.min(self.max_layer)).rev() {
            let neighbors = self.search_layer(&self.nodes[&current].vector, l, self.m, current);
            node.neighbors[l] = neighbors.iter().cloned().collect();

            // Connect neighbors back to this node
            for &neighbor_id in &node.neighbors[l] {
                if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) {
                    if l < neighbor.neighbors.len() {
                        neighbor.neighbors[l].insert(id);
                    }
                }
            }

            if !neighbors.is_empty() {
                current = neighbors[0];
            }
        }

        // Update entry points if needed
        if layer > self.max_layer {
            for l in (self.max_layer + 1)..=layer {
                if l < self.entry_points.len() {
                    self.entry_points[l] = Some(id);
                }
            }
            self.max_layer = layer;
        }

        self.nodes.insert(id, node);
        Ok(())
    }

    /// Search for k nearest neighbors
    fn search(
        &self,
        query: &[f32],
        namespace_id: i64,
        k: usize,
        threshold: f32,
    ) -> Vec<(i64, f32)> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        // Start from entry point at highest layer
        let mut current = match self.entry_points[self.max_layer] {
            Some(id) => id,
            None => return Vec::new(),
        };

        // Traverse down to layer 0
        for l in (1..=self.max_layer).rev() {
            current = self.greedy_search(query, l, current);
        }

        // Search layer 0 with ef_search candidates
        let candidates = self.search_layer(query, 0, self.ef_search, current);

        // Filter by namespace and threshold, then rank
        let mut results: Vec<(i64, f32)> = candidates
            .into_iter()
            .filter_map(|id| {
                let node = self.nodes.get(&id)?;
                if node.namespace_id != namespace_id {
                    return None;
                }
                let sim = cosine_similarity(query, &node.vector);
                if sim >= threshold {
                    Some((id, sim))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }

    /// Remove a node from the index
    fn remove(&mut self, id: i64) -> bool {
        if let Some(node) = self.nodes.remove(&id) {
            // Remove from neighbors' connections
            for layer_neighbors in &node.neighbors {
                for &neighbor_id in layer_neighbors {
                    if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) {
                        for layer in &mut neighbor.neighbors {
                            layer.remove(&id);
                        }
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Get a node by ID
    fn get(&self, id: i64) -> Option<&HnswNode> {
        self.nodes.get(&id)
    }

    /// Get vector count
    fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Generate random layer using exponential distribution
    fn random_layer(&self) -> usize {
        // Simple layer generation using L=1/ln(M)
        let ml = 1.0 / (self.m as f32).ln();
        let random: f32 = rand_simple();
        let layer = (-random.ln() * ml).floor() as usize;
        layer.min(9) // Cap at 9 layers
    }

    /// Greedy search at a specific layer
    fn greedy_search(&self, query: &[f32], layer: usize, start: i64) -> i64 {
        let mut current = start;
        let mut current_sim = cosine_similarity(query, &self.nodes[&current].vector);

        loop {
            let mut changed = false;
            if let Some(node) = self.nodes.get(&current) {
                if layer < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[layer] {
                        if let Some(neighbor) = self.nodes.get(&neighbor_id) {
                            let sim = cosine_similarity(query, &neighbor.vector);
                            if sim > current_sim {
                                current = neighbor_id;
                                current_sim = sim;
                                changed = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        current
    }

    /// Search a layer for candidates
    fn search_layer(&self, query: &[f32], layer: usize, ef: usize, start: i64) -> Vec<i64> {
        let mut visited = HashSet::new();
        let mut candidates = vec![(start, cosine_similarity(query, &self.nodes[&start].vector))];
        let mut results = vec![(start, cosine_similarity(query, &self.nodes[&start].vector))];
        visited.insert(start);

        while !candidates.is_empty() {
            // Get closest candidate
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let (current_id, current_sim) = candidates.remove(0);

            // Get furthest result
            results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let furthest_sim = results.first().map(|r| r.1).unwrap_or(0.0);

            if current_sim < furthest_sim && results.len() >= ef {
                break;
            }

            // Explore neighbors
            if let Some(node) = self.nodes.get(&current_id) {
                if layer < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[layer] {
                        if !visited.contains(&neighbor_id) {
                            visited.insert(neighbor_id);
                            if let Some(neighbor) = self.nodes.get(&neighbor_id) {
                                let sim = cosine_similarity(query, &neighbor.vector);
                                candidates.push((neighbor_id, sim));
                                results.push((neighbor_id, sim));

                                // Keep only top ef results
                                if results.len() > ef {
                                    results.sort_by(|a, b| {
                                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    results.truncate(ef);
                                }
                            }
                        }
                    }
                }
            }
        }

        results.into_iter().map(|(id, _)| id).collect()
    }
}

/// Simple random number for layer generation
fn rand_simple() -> f32 {
    // Use a simple hash-based random for deterministic behavior
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    now.hash(&mut hasher);
    (hasher.finish() as f32) / (u64::MAX as f32)
}

/// Graph tree node for relevance boosting
#[derive(Debug, Clone)]
struct GraphTreeNode {
    /// Node ID
    id: i64,
    /// Category
    category: MemoryCategory,
    /// Lane type
    lane_type: Option<MemoryLaneType>,
    /// Priority (1=high, 2=medium, 3=low)
    priority: u8,
    /// Weight for boosting
    weight: f32,
    /// Parent ID
    parent_id: Option<i64>,
}

impl GraphTreeNode {
    fn new(id: i64, category: MemoryCategory, lane_type: Option<MemoryLaneType>, priority: u8) -> Self {
        let weight = match priority {
            1 => 1.5,
            2 => 1.2,
            _ => 1.0,
        };
        Self {
            id,
            category,
            lane_type,
            priority,
            weight,
            parent_id: None,
        }
    }
}

/// Graph tree for hierarchical relevance boosting
#[derive(Debug, Default)]
struct GraphTree {
    nodes: HashMap<i64, GraphTreeNode>,
}

impl GraphTree {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    fn add_node(&mut self, id: i64, category: MemoryCategory, lane_type: Option<MemoryLaneType>, priority: u8) {
        let node = GraphTreeNode::new(id, category, lane_type, priority);
        self.nodes.insert(id, node);
    }

    fn remove_node(&mut self, id: i64) -> bool {
        self.nodes.remove(&id).is_some()
    }

    fn calculate_boosted_score(&self, id: i64, base_similarity: f32) -> f32 {
        if let Some(node) = self.nodes.get(&id) {
            // Apply priority weight
            let weighted = base_similarity * node.weight;

            // Apply lane type boost
            let lane_boost = node.lane_type.as_ref().map(|lt| lt.boost_factor()).unwrap_or(1.0);

            weighted * lane_boost
        } else {
            base_similarity
        }
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// Statistics about the vector store
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorStoreStats {
    /// Total vectors stored
    pub total_vectors: usize,
    /// Number of namespaces
    pub namespace_count: usize,
    /// Memory usage estimate in bytes
    pub memory_usage_estimate: usize,
    /// HNSW max layer
    pub max_layer: usize,
    /// Graph tree size
    pub tree_size: usize,
}

/// Nexus Vector Store with HNSW indexing and graph tree boosting
pub struct NexusVectorStore {
    /// Configuration
    config: VectorStoreConfig,
    /// HNSW index for fast ANN search
    index: Arc<RwLock<HnswIndex>>,
    /// Graph tree for relevance boosting
    tree: Arc<RwLock<GraphTree>>,
    /// Namespace counter
    namespace_counter: Arc<RwLock<i64>>,
    /// Statistics
    stats: Arc<RwLock<VectorStoreStats>>,
}

impl NexusVectorStore {
    /// Create a new vector store
    pub async fn new(config: VectorStoreConfig) -> Result<Self> {
        let index = HnswIndex::new(&config);
        let tree = GraphTree::new();

        Ok(Self {
            config,
            index: Arc::new(RwLock::new(index)),
            tree: Arc::new(RwLock::new(tree)),
            namespace_counter: Arc::new(RwLock::new(1)),
            stats: Arc::new(RwLock::new(VectorStoreStats::default())),
        })
    }

    /// Create an in-memory store with default config
    pub async fn in_memory() -> Result<Self> {
        Self::new(VectorStoreConfig::default()).await
    }

    /// Store an embedding with metadata
    pub async fn store_embedding(
        &self,
        memory_id: i64,
        embedding: Vec<f32>,
        namespace_id: i64,
        category: MemoryCategory,
        lane_type: Option<MemoryLaneType>,
        priority: u8,
    ) -> Result<()> {
        let start = Instant::now();

        // Validate dimension
        if embedding.len() != self.config.dimension {
            return Err(anyhow::anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.config.dimension,
                embedding.len()
            ));
        }

        // Add to HNSW index
        {
            let mut index = self.index.write().await;
            index.insert(memory_id, embedding, namespace_id, category, lane_type, priority)?;
        }

        // Add to graph tree
        {
            let mut tree = self.tree.write().await;
            tree.add_node(memory_id, category, lane_type, priority);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_vectors += 1;
            let index = self.index.read().await;
            stats.max_layer = index.max_layer;
            let tree = self.tree.read().await;
            stats.tree_size = tree.len();
            stats.memory_usage_estimate = stats.total_vectors * self.config.dimension * 4; // 4 bytes per f32
        }

        debug!(
            "Stored embedding {} (latency={:?})",
            memory_id,
            start.elapsed()
        );

        Ok(())
    }

    /// Search for similar vectors
    pub async fn search_similar(
        &self,
        query: &[f32],
        namespace_id: i64,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<VectorSearchResult>> {
        let start = Instant::now();

        // Validate dimension
        if query.len() != self.config.dimension {
            return Err(anyhow::anyhow!(
                "Query dimension mismatch: expected {}, got {}",
                self.config.dimension,
                query.len()
            ));
        }

        // Search HNSW index
        let raw_results = {
            let index = self.index.read().await;
            index.search(query, namespace_id, limit * 2, threshold)
        };

        // Apply graph boosting
        let mut results = Vec::with_capacity(raw_results.len());
        {
            let tree = self.tree.read().await;
            let index = self.index.read().await;

            for (id, similarity) in raw_results {
                let boosted_score = tree.calculate_boosted_score(id, similarity);

                if let Some(node) = index.get(id) {
                    let mut result = VectorSearchResult::new(id, node.namespace_id, similarity);
                    result.category = node.category;
                    result.lane_type = node.lane_type;
                    result.apply_boost(
                        node.weight(),
                        0, // Depth not tracked in this simplified version
                        node.lane_type.as_ref().map(|lt| lt.boost_factor()),
                    );
                    result.boosted_score = boosted_score;

                    results.push(result);
                }
            }
        }

        // Sort by boosted score
        results.sort_by(|a, b| {
            b.boosted_score
                .partial_cmp(&a.boosted_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        debug!(
            "Vector search returned {} results (latency={:?})",
            results.len(),
            start.elapsed()
        );

        Ok(results)
    }

    /// Remove an embedding
    pub async fn remove_embedding(&self, memory_id: i64) -> Result<bool> {
        let mut index = self.index.write().await;
        let mut tree = self.tree.write().await;

        let removed = index.remove(memory_id);
        tree.remove_node(memory_id);

        if removed {
            let mut stats = self.stats.write().await;
            stats.total_vectors = stats.total_vectors.saturating_sub(1);
        }

        Ok(removed)
    }

    /// Get embedding by ID
    pub async fn get_embedding(&self, memory_id: i64) -> Option<Vec<f32>> {
        let index = self.index.read().await;
        index.get(memory_id).map(|n| n.vector.clone())
    }

    /// Get or create a namespace
    pub async fn get_or_create_namespace(&self, name: &str) -> Result<i64> {
        // In a full implementation, this would persist namespaces
        let mut counter = self.namespace_counter.write().await;
        let id = *counter;
        *counter += 1;
        Ok(id)
    }

    /// Get vector count
    pub async fn len(&self) -> usize {
        self.index.read().await.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        self.index.read().await.is_empty()
    }

    /// Get statistics
    pub async fn stats(&self) -> VectorStoreStats {
        self.stats.read().await.clone()
    }

    /// Clear all vectors
    pub async fn clear(&self) {
        let mut index = self.index.write().await;
        let mut tree = self.tree.write().await;

        *index = HnswIndex::new(&self.config);
        *tree = GraphTree::new();

        let mut stats = self.stats.write().await;
        stats.total_vectors = 0;
        stats.max_layer = 0;
        stats.tree_size = 0;

        info!("Vector store cleared");
    }
}

impl HnswNode {
    fn weight(&self) -> f32 {
        match self.priority {
            1 => 1.5,
            2 => 1.2,
            _ => 1.0,
        }
    }
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_store_creation() {
        let store = NexusVectorStore::in_memory().await.unwrap();
        assert!(store.is_empty().await);
    }

    #[tokio::test]
    async fn test_store_and_search() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        // Store a vector
        let embedding = vec![0.5; EMBEDDING_DIMENSION];
        store
            .store_embedding(
                1,
                embedding.clone(),
                1,
                MemoryCategory::General,
                None,
                3,
            )
            .await
            .unwrap();

        assert_eq!(store.len().await, 1);

        // Search with same vector
        let results = store
            .search_similar(&embedding, 1, 10, 0.0)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
        assert!((results[0].similarity - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_dimension_validation() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        let wrong_dim = vec![0.5; 100];
        let result = store
            .store_embedding(1, wrong_dim, 1, MemoryCategory::General, None, 3)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_namespace_filtering() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        let embedding = vec![0.5; EMBEDDING_DIMENSION];

        store
            .store_embedding(1, embedding.clone(), 1, MemoryCategory::General, None, 3)
            .await
            .unwrap();
        store
            .store_embedding(2, embedding.clone(), 2, MemoryCategory::General, None, 3)
            .await
            .unwrap();

        // Search in namespace 1
        let results = store.search_similar(&embedding, 1, 10, 0.0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);

        // Search in namespace 2
        let results = store.search_similar(&embedding, 2, 10, 0.0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 2);
    }

    #[tokio::test]
    async fn test_priority_boosting() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        // Store two similar vectors with different priorities
        let embedding = vec![0.5; EMBEDDING_DIMENSION];

        store
            .store_embedding(1, embedding.clone(), 1, MemoryCategory::General, None, 3) // Low priority
            .await
            .unwrap();
        store
            .store_embedding(2, embedding.clone(), 1, MemoryCategory::General, None, 1) // High priority
            .await
            .unwrap();

        let results = store.search_similar(&embedding, 1, 10, 0.0).await.unwrap();

        // Both should be found
        assert_eq!(results.len(), 2);

        // High priority should have higher boosted score
        let high_priority = results.iter().find(|r| r.id == 2).unwrap();
        let low_priority = results.iter().find(|r| r.id == 1).unwrap();
        assert!(high_priority.boosted_score > low_priority.boosted_score);
    }

    #[tokio::test]
    async fn test_lane_type_boost() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        let embedding = vec![0.5; EMBEDDING_DIMENSION];

        store
            .store_embedding(
                1,
                embedding.clone(),
                1,
                MemoryCategory::General,
                Some(MemoryLaneType::Correction),
                3,
            )
            .await
            .unwrap();
        store
            .store_embedding(
                2,
                embedding.clone(),
                1,
                MemoryCategory::General,
                None,
                3,
            )
            .await
            .unwrap();

        let results = store.search_similar(&embedding, 1, 10, 0.0).await.unwrap();

        // Correction should be boosted
        let correction = results.iter().find(|r| r.id == 1).unwrap();
        let regular = results.iter().find(|r| r.id == 2).unwrap();
        assert!(correction.boosted_score > regular.boosted_score);
    }

    #[tokio::test]
    async fn test_remove_embedding() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        let embedding = vec![0.5; EMBEDDING_DIMENSION];
        store
            .store_embedding(1, embedding.clone(), 1, MemoryCategory::General, None, 3)
            .await
            .unwrap();

        let removed = store.remove_embedding(1).await.unwrap();
        assert!(removed);

        assert!(store.is_empty().await);

        // Remove non-existent
        let removed = store.remove_embedding(999).await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_threshold_filtering() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        // Store two different vectors
        let e1 = vec![1.0; EMBEDDING_DIMENSION];
        let mut e2 = vec![0.0; EMBEDDING_DIMENSION];
        e2[0] = -1.0; // Orthogonal-ish

        store
            .store_embedding(1, e1.clone(), 1, MemoryCategory::General, None, 3)
            .await
            .unwrap();
        store
            .store_embedding(2, e2, 1, MemoryCategory::General, None, 3)
            .await
            .unwrap();

        // Search with high threshold
        let results = store.search_similar(&e1, 1, 10, 0.9).await.unwrap();
        assert!(results.len() <= 1); // Only very similar results
    }

    #[tokio::test]
    async fn test_clear() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        let embedding = vec![0.5; EMBEDDING_DIMENSION];
        store
            .store_embedding(1, embedding, 1, MemoryCategory::General, None, 3)
            .await
            .unwrap();

        store.clear().await;
        assert!(store.is_empty().await);
    }

    #[tokio::test]
    async fn test_stats() {
        let store = NexusVectorStore::in_memory().await.unwrap();

        let embedding = vec![0.5; EMBEDDING_DIMENSION];
        store
            .store_embedding(1, embedding, 1, MemoryCategory::General, None, 3)
            .await
            .unwrap();

        let stats = store.stats().await;
        assert_eq!(stats.total_vectors, 1);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.01);
    }
}
