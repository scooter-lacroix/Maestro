//! HNSW-based Vector Store
//!
//! True HNSW (Hierarchical Navigable Small World) implementation
//! for approximate nearest neighbor search with cosine similarity.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::{debug, info, warn};

use hnswx::{CosineSimilarity, HnswConfig, HnswStats, HNSW};

use super::cache::TtlCache;
use super::metadata::*;

/// HNSW-based vector store for approximate nearest neighbor search
pub struct HnswVectorStore {
    index_path: PathBuf,
    metadata: RwLock<IndexMetadata>,
    hnsw: RwLock<HNSW<CosineSimilarity>>,
    /// Maps HNSW internal ID -> our vector data with embedding
    id_to_data: RwLock<HashMap<usize, VectorDataWithEmbedding>>,
    /// Tracks deleted vector IDs (tombstones)
    tombstones: RwLock<HashSet<usize>>,
    cache: TtlCache<String, Vec<SearchResult>>,
    _config: HnswConfig,
}

/// Vector data stored alongside HNSW index - INCLUDES EMBEDDING
#[derive(Clone)]
struct VectorDataWithEmbedding {
    id: String,
    embedding: Vec<f32>,
    metadata: VectorMetadata,
    content: Option<String>,
}

impl HnswVectorStore {
    /// Create a new HNSW vector store
    pub fn new(index_path: Option<PathBuf>, config: Option<HnswConfig>) -> Result<Self> {
        let path = index_path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".leindex_hnsw");
            p
        });

        fs::create_dir_all(&path)?;

        let metadata = Self::load_or_create_metadata(&path)?;
        let hnsw_config = config.unwrap_or_else(Self::default_config);

        // Create HNSW index with cosine similarity metric
        let hnsw = HNSW::new(hnsw_config.clone(), CosineSimilarity::new());

        let store = Self {
            index_path: path,
            metadata: RwLock::new(metadata),
            hnsw: RwLock::new(hnsw),
            id_to_data: RwLock::new(HashMap::new()),
            tombstones: RwLock::new(HashSet::new()),
            // PERF: Task 7.6.27 - Double cache capacity and increase TTL for better hit rates
            // 2000 entries (was 1000), 10min TTL (was 5min)
            cache: TtlCache::new(2000, 600),
            _config: hnsw_config,
        };

        // Load existing data if available
        if let Err(e) = store.load_vectors() {
            warn!("Failed to load existing vectors: {}", e);
        }

        info!("HNSW VectorStore initialized at {:?}", store.index_path);
        Ok(store)
    }

    /// Default HNSW configuration
    fn default_config() -> HnswConfig {
        HnswConfig {
            max_elements: 1_000_000, // Increased to support benchmarking up to 500K vectors
            level_multiplier: 1.0 / std::f64::consts::LN_2,
            m: 32,
            m_max: 32,
            m_max_0: 64,
            ef_construction: 200,
            ef_search: 10,
            allow_replace_deleted: true,
            num_threads: 0,
            batch_size: 64,
        }
    }

    /// Load vectors from disk
    fn load_vectors(&self) -> Result<()> {
        let vectors_path = self.index_path.join("vectors.json");
        if !vectors_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&vectors_path)?;
        // FIX: Include content field in deserialization (Task 7.6.11)
        let stored_entries: Vec<(String, Vec<f32>, VectorMetadata, Option<String>)> =
            serde_json::from_str(&content).context("Failed to parse vectors.json")?;

        let mut id_map = self
            .id_to_data
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut hnsw = self
            .hnsw
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Clear existing data first
        id_map.clear();
        // Note: We can't clear the HNSW index directly, so we rebuild it

        // Create a new HNSW index to replace the old one
        let config = self._config.clone();
        let mut new_hnsw = HNSW::new(config, CosineSimilarity::new());

        for (id, embedding, metadata, content) in stored_entries {
            // Insert into new HNSW - it returns the internal ID
            let internal_id = new_hnsw.insert(embedding.clone());

            // Store metadata WITH embedding AND content for persistence
            id_map.insert(
                internal_id,
                VectorDataWithEmbedding {
                    id,
                    embedding,
                    metadata,
                    content, // FIX: Restore content field (Task 7.6.11)
                },
            );
        }

        // Replace the HNSW index
        *self
            .hnsw
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))? = new_hnsw;

        info!("Loaded {} vectors into HNSW index", id_map.len());
        Ok(())
    }

    /// Load or create index metadata
    fn load_or_create_metadata(path: &Path) -> Result<IndexMetadata> {
        let meta_path = path.join("metadata.json");

        if meta_path.exists() {
            let content = fs::read_to_string(&meta_path)?;
            serde_json::from_str(&content).context("Failed to parse index metadata")
        } else {
            Ok(IndexMetadata {
                backend: "hnsw".to_string(),
                ..Default::default()
            })
        }
    }

    /// Add a vector to the HNSW index
    pub fn add_vector(
        &self,
        content: &str,
        embedding: Vec<f32>,
        metadata: VectorMetadata,
    ) -> Result<String> {
        // Insert into HNSW index - returns internal ID
        let internal_id = {
            let mut hnsw = self
                .hnsw
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            hnsw.insert(embedding.clone())
        };

        let external_id = format!("vec_{}", internal_id);

        // Store metadata WITH embedding
        {
            let mut id_map = self
                .id_to_data
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            id_map.insert(
                internal_id,
                VectorDataWithEmbedding {
                    id: external_id.clone(),
                    embedding,
                    metadata: metadata.clone(),
                    content: Some(content.to_string()),
                },
            );
        }

        // Update metadata
        {
            let mut meta = self
                .metadata
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            meta.vector_count += 1;
            meta.updated_at = chrono::Utc::now();
        }

        // Invalidate cache
        let _ = self.cache.clear();

        debug!("Added vector {} to HNSW index", external_id);
        Ok(external_id)
    }

    /// Add a vector with a specific external ID (for unified identity across backends - Task 7.6.12)
    pub fn add_vector_with_id(
        &self,
        external_id: &str,
        content: &str,
        embedding: Vec<f32>,
        metadata: VectorMetadata,
    ) -> Result<String> {
        // Validate external_id format (must start with "vec_")
        if !external_id.starts_with("vec_") {
            return Err(anyhow::anyhow!(
                "Invalid external_id format: must start with 'vec_', got: {}",
                external_id
            ));
        }

        // Insert into HNSW index - returns internal ID
        let internal_id = {
            let mut hnsw = self
                .hnsw
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            hnsw.insert(embedding.clone())
        };

        // Store metadata WITH embedding, using the provided external_id
        {
            let mut id_map = self
                .id_to_data
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            id_map.insert(
                internal_id,
                VectorDataWithEmbedding {
                    id: external_id.to_string(),
                    embedding,
                    metadata: metadata.clone(),
                    content: Some(content.to_string()),
                },
            );
        }

        // Update metadata
        {
            let mut meta = self
                .metadata
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            meta.vector_count += 1;
            meta.updated_at = chrono::Utc::now();
        }

        // Invalidate cache
        let _ = self.cache.clear();

        debug!(
            "Added vector {} to HNSW index (with specific external ID)",
            external_id
        );
        Ok(external_id.to_string())
    }

    /// Search for similar vectors using HNSW
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        let top_k = top_k.min(MAX_TOP_K);

        // Check cache
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for &val in query_embedding {
            hasher.update(&val.to_le_bytes());
        }
        hasher.update(&(top_k as u64).to_le_bytes());
        let cache_key = format!("{:x}", hasher.finalize());

        if let Some(cached) = self.cache.get(&cache_key)? {
            debug!("Cache hit for HNSW search");
            return Ok(cached);
        }

        // Search HNSW index
        let results = {
            let hnsw = self
                .hnsw
                .read()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            hnsw.search_knn(query_embedding, top_k)
        };

        // Convert results, filtering out tombstones
        let id_map = self
            .id_to_data
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let tombstones = self
            .tombstones
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut search_results = Vec::new();

        for result in results {
            // Skip if this ID is marked as a tombstone
            if tombstones.contains(&result.id) {
                continue;
            }

            if let Some(data) = id_map.get(&result.id) {
                // HNSW returns distance, convert to similarity score
                // For cosine similarity: distance = 1 - similarity, so similarity = 1 - distance
                let similarity = 1.0 - result.distance.max(0.0).min(1.0);

                search_results.push(SearchResult {
                    vector_id: data.id.clone(),
                    score: similarity,
                    metadata: data.metadata.clone(),
                    content: data.content.clone(),
                });
            }
        }

        // Sort by score descending (highest similarity first)
        search_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Cache results
        self.cache.put(cache_key, search_results.clone())?;

        Ok(search_results)
    }

    /// Delete vectors by file path - marks as tombstones instead of rebuilding
    pub fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        let mut id_map = self
            .id_to_data
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut tombstones = self
            .tombstones
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Find IDs to mark as deleted
        let to_delete: Vec<usize> = id_map
            .iter()
            .filter(|(_, data)| data.metadata.file_path == file_path)
            .map(|(id, _)| *id)
            .collect();

        // Mark as tombstones
        for id in &to_delete {
            tombstones.insert(*id);
        }

        let deleted = to_delete.len();

        if deleted > 0 {
            // Update metadata
            let mut meta = self
                .metadata
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            meta.vector_count = meta.vector_count.saturating_sub(deleted);
            meta.updated_at = chrono::Utc::now();
            drop(meta); // Release lock early

            // Check if we need to rebuild the index due to too many tombstones
            let total_elements = id_map.len();
            let tombstone_count = tombstones.len();

            if tombstone_count > 0
                && (tombstone_count as f64) / ((total_elements + tombstone_count) as f64) > 0.1
            {
                // More than 10% are tombstones, rebuild the index
                self.rebuild_index()?;
            }

            self.cache.clear();
        }

        Ok(deleted)
    }

    /// Rebuild the index when tombstones exceed threshold
    fn rebuild_index(&self) -> Result<()> {
        let id_map = self
            .id_to_data
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let tombstones = self
            .tombstones
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Create new HNSW index
        let config = self._config.clone();
        let mut new_hnsw = HNSW::new(config, CosineSimilarity::new());

        // Create new id_to_data HashMap with new internal IDs
        let mut new_id_map = HashMap::new();

        // Re-insert only non-tombstoned vectors, capturing new internal IDs
        for (old_id, data) in id_map.iter() {
            if !tombstones.contains(old_id) {
                // CRITICAL: Capture the new internal ID returned by insert()
                let new_internal_id = new_hnsw.insert(data.embedding.clone());
                // Build new HashMap with NEW internal IDs as keys
                new_id_map.insert(new_internal_id, data.clone());
            }
        }

        // Get active count BEFORE moving new_id_map
        let active_count = new_id_map.len();

        // Atomically swap BOTH HNSW and id_to_data
        *self
            .hnsw
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))? = new_hnsw;
        *self
            .id_to_data
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))? = new_id_map;

        // Update vector_count to reflect actual count (excluding tombstones)
        {
            let mut metadata = self
                .metadata
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            metadata.vector_count = active_count;
            metadata.updated_at = chrono::Utc::now();
        }

        // Clear tombstones since we've rebuilt without them
        self.tombstones
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?
            .clear();

        debug!(
            "HNSW index rebuilt with {} vectors (pruned {} tombstones), ID mapping synchronized",
            active_count,
            tombstones.len()
        );
        Ok(())
    }

    /// Get vector count
    pub fn vector_count(&self) -> Result<usize> {
        let metadata = self
            .metadata
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(metadata.vector_count)
    }

    /// Get index info
    pub fn info(&self) -> Result<IndexMetadata> {
        Ok(self
            .metadata
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?
            .clone())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<super::cache::CacheStats> {
        self.cache.stats()
    }

    /// Get HNSW statistics
    pub fn hnsw_stats(&self) -> Result<HnswStats> {
        let hnsw = self
            .hnsw
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(hnsw.stats())
    }

    /// Persist index to disk - COMPLETE PERSISTENCE
    pub fn persist(&self) -> Result<()> {
        // Save metadata
        let meta_path = self.index_path.join("metadata.json");
        let content = serde_json::to_string_pretty(
            &*self
                .metadata
                .read()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?,
        )?;
        fs::write(meta_path, content)?;

        // Save vectors WITH embeddings for proper rebuilding
        let vectors_path = self.index_path.join("vectors.json");
        let id_map = self
            .id_to_data
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let tombstones = self
            .tombstones
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Serialize vectors with their embeddings AND content, excluding tombstones
        let serializable: Vec<_> = id_map
            .iter()
            .filter(|(id, _)| !tombstones.contains(id))
            .map(|(_, data)| {
                (
                    data.id.clone(),
                    data.embedding.clone(),
                    data.metadata.clone(),
                    data.content.clone(), // FIX: Include content field (Task 7.6.11)
                )
            })
            .collect();

        let vectors_content = serde_json::to_string_pretty(&serializable)?;
        fs::write(vectors_path, vectors_content)?;

        info!(
            "Persisted HNSW vector store with {} vectors including embeddings",
            id_map.len() - tombstones.len()
        );
        Ok(())
    }
}

// Task 7.6.22: Test HNSW basic operations
// Note: Full rebuild testing is too slow for CI - tested in integration benchmarks
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_store_creation() {
        // Task 7.6.22: Test that HNSW store can be created and configured
        // Full rebuild and ID mapping testing is done in integration benchmarks
        // due to slow HNSW insert performance (even with minimal config)
        let config = HnswConfig {
            max_elements: 100,
            level_multiplier: 1.0 / std::f64::consts::LN_2,
            m: 8,
            m_max: 8,
            m_max_0: 16,
            ef_construction: 50,
            ef_search: 10,
            allow_replace_deleted: true,
            num_threads: 0,
            batch_size: 16,
        };

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let store = HnswVectorStore::new(Some(temp_dir.path().to_path_buf()), Some(config))
            .expect("Failed to create HNSW store");

        // Verify store is initialized
        let count = store.vector_count().expect("Failed to get count");
        assert_eq!(count, 0);

        // Verify we can get stats
        let stats = store.hnsw_stats().expect("Failed to get stats");
        assert_eq!(stats.node_count, 0);

        // Verify cache stats work
        let cache_stats = store.cache_stats().expect("Failed to get cache stats");
        assert_eq!(cache_stats.hits, 0);
        assert_eq!(cache_stats.misses, 0);
    }
}

impl Default for HnswVectorStore {
    fn default() -> Self {
        Self::new(None, None).unwrap()
    }
}
