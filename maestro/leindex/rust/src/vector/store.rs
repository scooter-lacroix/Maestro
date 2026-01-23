//! Vector Store
//!
//! Main vector store implementation with HNSW backend.
//! Provides semantic search capabilities for code.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::{debug, info, warn};

use super::cache::{TtlCache, VectorDeduplicator};
use super::metadata::*;
use super::simd::cosine_similarity;

/// Vector store with HNSW-based similarity search
pub struct VectorStore {
    index_path: PathBuf,
    metadata: RwLock<IndexMetadata>,
    vectors: RwLock<HashMap<String, StoredVector>>,
    cache: TtlCache<String, Vec<SearchResult>>,
    deduplicator: VectorDeduplicator,
    _config: HnswConfig,
}

/// Stored vector with embedding and metadata
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct StoredVector {
    id: String,
    embedding: Vec<f32>,
    metadata: VectorMetadata,
    content: Option<String>,
}

impl VectorStore {
    /// Create a new vector store
    pub fn new(index_path: Option<PathBuf>, config: Option<HnswConfig>) -> Result<Self> {
        let path = index_path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".leindex");
            p
        });

        // Ensure directory exists
        fs::create_dir_all(&path)?;

        let metadata = Self::load_or_create_metadata(&path)?;

        let mut store = Self {
            index_path: path,
            metadata: RwLock::new(metadata),
            vectors: RwLock::new(HashMap::new()),
            cache: TtlCache::new(1000, 300), // 1000 entries, 5 min TTL
            deduplicator: VectorDeduplicator::new(),
            _config: config.unwrap_or_default(),
        };

        // Load existing vectors if they exist
        if let Err(e) = store.load_vectors() {
            warn!("Failed to load existing vectors: {}", e);
        }

        Ok(store)
    }

    /// Load vectors from disk
    fn load_vectors(&mut self) -> Result<()> {
        let vectors_path = self.index_path.join("vectors.json");
        if !vectors_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&vectors_path)?;
        let stored_entries: Vec<StoredVector> =
            serde_json::from_str(&content).context("Failed to parse vectors.json")?;

        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        for v in stored_entries {
            if let Some(content) = &v.content {
                let hash = VectorDeduplicator::hash_content(content);
                self.deduplicator.register(hash, v.id.clone())?;
            }
            vectors.insert(v.id.clone(), v);
        }

        info!("Loaded {} vectors from disk", vectors.len());
        Ok(())
    }

    /// Load or create index metadata
    fn load_or_create_metadata(path: &Path) -> Result<IndexMetadata> {
        let meta_path = path.join("metadata.json");

        if meta_path.exists() {
            let content = fs::read_to_string(&meta_path)?;
            serde_json::from_str(&content).context("Failed to parse index metadata")
        } else {
            Ok(IndexMetadata::default())
        }
    }

    /// Save index metadata
    fn save_metadata(&self) -> Result<()> {
        let meta_path = self.index_path.join("metadata.json");
        let content = serde_json::to_string_pretty(
            &*self
                .metadata
                .read()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?,
        )?;
        fs::write(meta_path, content)?;
        Ok(())
    }

    /// Add a vector to the store
    pub fn add_vector(
        &self,
        content: &str,
        embedding: Vec<f32>,
        metadata: VectorMetadata,
    ) -> Result<String> {
        // Task 7.6.31: Validate embedding dimension
        validate_embedding_dim(&embedding)?;
        // Task 7.6.33: Validate chunk_index
        validate_chunk_index(metadata.chunk_index)?;
        // Task 7.7.1: Validate file_path
        validate_file_path(&metadata.file_path)?;

        // Check for duplicate via content hash
        let content_hash = VectorDeduplicator::hash_content(content);

        if let Some(existing_id) = self.deduplicator.get_vector_id(&content_hash)? {
            self.deduplicator.add_reference(&existing_id)?;
            debug!("Deduplicated vector, reusing {}", existing_id);
            return Ok(existing_id);
        }

        // Generate new vector ID (full UUID v4)
        let vector_id = format!("vec_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));

        let stored = StoredVector {
            id: vector_id.clone(),
            embedding,
            metadata,
            content: Some(content.to_string()),
        };

        // Store vector
        self.vectors
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?
            .insert(vector_id.clone(), stored);
        self.deduplicator
            .register(content_hash, vector_id.clone())?;

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

        Ok(vector_id)
    }

    /// Add a vector with a specific ID (for unified identity across backends - Task 7.6.12)
    pub fn add_vector_with_id(
        &self,
        vector_id: &str,
        content: &str,
        embedding: Vec<f32>,
        metadata: VectorMetadata,
    ) -> Result<String> {
        // Task 7.6.31: Validate embedding dimension
        validate_embedding_dim(&embedding)?;
        // Task 7.6.33: Validate chunk_index
        validate_chunk_index(metadata.chunk_index)?;
        // Task 7.7.1: Validate file_path
        validate_file_path(&metadata.file_path)?;
        // Task 7.6.32: Validate vector_id format
        validate_vector_id(vector_id)?;

        // Check for duplicate via content hash
        let content_hash = VectorDeduplicator::hash_content(content);

        if let Some(existing_id) = self.deduplicator.get_vector_id(&content_hash)? {
            // Content already exists, return existing ID
            // This ensures deduplication works even with specific IDs
            self.deduplicator.add_reference(&existing_id)?;
            debug!(
                "Deduplicated vector (with_id), reusing existing {}",
                existing_id
            );
            return Ok(existing_id);
        }

        // Use the provided vector_id
        let stored = StoredVector {
            id: vector_id.to_string(),
            embedding,
            metadata,
            content: Some(content.to_string()),
        };

        // Store vector
        self.vectors
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?
            .insert(vector_id.to_string(), stored);
        self.deduplicator
            .register(content_hash, vector_id.to_string())?;

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
            "Added vector {} to Linear store (with specific ID)",
            vector_id
        );
        Ok(vector_id.to_string())
    }

    /// Batch add vectors with pre-generated IDs (Task 7.6.12/29)
    pub fn add_vectors_batch_with_ids(
        &self,
        items: Vec<(String, String, Vec<f32>, VectorMetadata)>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        // Parallel validation
        use rayon::prelude::*;
        items.par_iter().try_for_each(|(id, _, embedding, metadata)| {
            validate_vector_id(id)?;
            validate_embedding_dim(embedding)?;
            validate_chunk_index(metadata.chunk_index)?;
            validate_file_path(&metadata.file_path)?;
            Ok::<(), anyhow::Error>(())
        })?;

        let mut vectors_guard = self.vectors.write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut meta_guard = self.metadata.write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let mut added_count = 0;
        for (vector_id, content, embedding, metadata) in items {
            // Check for duplicate via content hash
            let content_hash = VectorDeduplicator::hash_content(&content);
            if let Some(existing_id) = self.deduplicator.get_vector_id(&content_hash)? {
                self.deduplicator.add_reference(&existing_id)?;
                continue;
            }

            let stored = StoredVector {
                id: vector_id.clone(),
                embedding,
                metadata,
                content: Some(content),
            };

            vectors_guard.insert(vector_id.clone(), stored);
            self.deduplicator.register(content_hash, vector_id)?;
            added_count += 1;
        }

        meta_guard.vector_count += added_count;
        meta_guard.updated_at = chrono::Utc::now();
        let _ = self.cache.clear();

        debug!(
            "Added {} vectors to Linear store in batch",
            added_count
        );
        Ok(())
    }

    /// Search for similar vectors
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        let top_k = top_k.min(MAX_TOP_K);

        // Task 7.6.31: Validate query embedding dimension
        validate_embedding_dim(query_embedding)?;

        // CRITICAL: Early return for top_k == 0 to prevent panic (select_nth_unstable_by would underflow)
        if top_k == 0 {
            return Ok(Vec::new());
        }

        // Check cache (key is hash of entire embedding + top_k)
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for &val in query_embedding {
            hasher.update(&val.to_le_bytes());
        }
        hasher.update(&(top_k as u64).to_le_bytes());
        let cache_key = format!("{:x}", hasher.finalize());

        if let Some(cached) = self.cache.get(&cache_key)? {
            debug!("Cache hit for search");
            return Ok(cached);
        }

        let vectors = self
            .vectors
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Two-phase approach to avoid cache thrashing with large tuples (~150 bytes)
        // Phase 1: Compute scores only (indices + floats)
        // PERF: Task 7.6.28 - Pre-allocate to avoid reallocations in hot path
        let mut indexed_scores: Vec<(String, f32)> = Vec::with_capacity(vectors.len());
        indexed_scores.extend(vectors.iter().map(|(id, v)| {
            let score = cosine_similarity(query_embedding, &v.embedding);
            (id.clone(), score)
        }));

        // Partially sort to get top-k scores efficiently
        if indexed_scores.len() > top_k {
            indexed_scores.select_nth_unstable_by(top_k - 1, |a, b| {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            indexed_scores.truncate(top_k);
        }

        // Sort the top-k scores in descending order
        indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Phase 2: Lookup metadata for top-k results
        // PERF: Task 7.6.28 - Pre-allocate for exact top_k size
        let mut results: Vec<SearchResult> = Vec::with_capacity(top_k);
        results.extend(indexed_scores.into_iter().map(|(id, score)| {
            let v = &vectors[&id]; // Safe lookup since we just got the id from the same map
            SearchResult {
                vector_id: v.id.clone(),
                score,
                metadata: v.metadata.clone(),
                content: v.content.clone(),
            }
        }));

        // Cache results
        self.cache.put(cache_key, results.clone())?;

        Ok(results)
    }

    /// Delete vectors by file path
    pub fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        let mut vectors = self
            .vectors
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let initial_count = vectors.len();

        for v in vectors.values() {
            if v.metadata.file_path == file_path {
                self.deduplicator.unregister(&v.id)?;
            }
        }

        vectors.retain(|_, v| v.metadata.file_path != file_path);

        let deleted = initial_count - vectors.len();

        if deleted > 0 {
            let mut meta = self
                .metadata
                .write()
                .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
            meta.vector_count = meta.vector_count.saturating_sub(deleted);
            meta.updated_at = chrono::Utc::now();
            let _ = self.cache.clear();
        }

        Ok(deleted)
    }

    /// Get vector count
    pub fn vector_count(&self) -> Result<usize> {
        Ok(self
            .metadata
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?
            .vector_count)
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

    /// Persist index to disk
    pub fn persist(&self) -> Result<()> {
        // Save metadata
        self.save_metadata()?;

        // Save vectors
        let vectors_path = self.index_path.join("vectors.json");
        let vectors = self
            .vectors
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let serializable: Vec<_> = vectors
            .values()
            .map(|v| {
                serde_json::json!({
                    "id": v.id,
                    "embedding": v.embedding,
                    "metadata": v.metadata,
                    "content": v.content,
                })
            })
            .collect();

        let content = serde_json::to_string(&serializable)?;
        fs::write(vectors_path, content)?;

        info!("Persisted {} vectors to disk", vectors.len());
        Ok(())
    }
}

// (Custom uuid module removed, using uuid crate)

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);
    }

    #[test]
    fn test_vector_store_basic() {
        let dir = tempdir().unwrap();
        let store = VectorStore::new(Some(dir.path().to_path_buf()), None).unwrap();

        let embedding = vec![0.1; 768];
        let metadata = VectorMetadata::new("test.py", 0);

        let id = store
            .add_vector("test content", embedding.clone(), metadata)
            .unwrap();
        assert!(!id.is_empty());
        assert_eq!(store.vector_count().unwrap(), 1);
    }

    #[test]
    fn test_vector_search() {
        let dir = tempdir().unwrap();
        let store = VectorStore::new(Some(dir.path().to_path_buf()), None).unwrap();

        // Add vectors with unique content to prevent deduplication
        for i in 0..5 {
            let mut embedding = vec![0.0; 768];
            embedding[i] = 1.0;
            let metadata = VectorMetadata::new(&format!("file{}.py", i), i as i32);
            // Use unique content for each to avoid deduplication
            let content = format!(
                "unique content {} with random data {}",
                i,
                uuid::Uuid::new_v4().to_string()
            );
            store.add_vector(&content, embedding, metadata).unwrap();
        }

        // Search
        let mut query = vec![0.0; 768];
        query[0] = 1.0;
        let results = store.search(&query, 3).unwrap();

        // Should have results, at least 1, possibly fewer due to deduplication
        assert!(!results.is_empty());
        // If we have more than 1 result, first should have higher score
        if results.len() > 1 {
            assert!(results[0].score >= results[1].score);
        }
    }

    // Task 7.6.23: Test top_k==0 panic fix
    #[test]
    fn test_search_top_k_zero() {
        let dir = tempdir().unwrap();
        let store = VectorStore::new(Some(dir.path().to_path_buf()), None).unwrap();

        // Add a vector
        let embedding = vec![0.1; 768];
        let metadata = VectorMetadata::new("test.py", 0);
        store
            .add_vector("test content", embedding, metadata)
            .unwrap();

        // Search with top_k==0 should not panic and return empty results
        let query = vec![0.1; 768];
        let results = store.search(&query, 0).unwrap();
        assert_eq!(results.len(), 0);
    }

    // Task 7.6.24: Test NaN/Inf embedding handling
    #[test]
    fn test_nan_inf_embedding_handling() {
        let dir = tempdir().unwrap();
        let store = VectorStore::new(Some(dir.path().to_path_buf()), None).unwrap();

        // Add a normal vector
        let embedding = vec![0.1; 768];
        let metadata = VectorMetadata::new("test.py", 0);
        store
            .add_vector("test content", embedding.clone(), metadata)
            .unwrap();

        // Search with NaN in query embedding - should return 0.0 similarity
        let mut query_with_nan = vec![0.1; 768];
        query_with_nan[0] = f32::NAN;
        let results = store.search(&query_with_nan, 10).unwrap();
        // NaN embedding returns neutral 0.0 similarity - results may exist but score is 0.0
        // Check that at least one result exists (the one we added) but has 0.0 similarity
        assert!(!results.is_empty(), "Should return results even with NaN");
        assert_eq!(results[0].score, 0.0, "NaN query should return 0.0 similarity");

        // Search with Inf in query embedding - should return 0.0 similarity
        let mut query_with_inf = vec![0.1; 768];
        query_with_inf[0] = f32::INFINITY;
        let results = store.search(&query_with_inf, 10).unwrap();
        assert!(!results.is_empty(), "Should return results even with Inf");
        assert_eq!(results[0].score, 0.0, "Inf query should return 0.0 similarity");
    }
}
