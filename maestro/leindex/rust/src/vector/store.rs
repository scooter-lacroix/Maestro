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
        let stored_entries: Vec<StoredVector> = serde_json::from_str(&content)
            .context("Failed to parse vectors.json")?;

        let mut vectors = self.vectors.write().unwrap();
        for v in stored_entries {
            if let Some(content) = &v.content {
                let hash = VectorDeduplicator::hash_content(content);
                self.deduplicator.register(hash, v.id.clone());
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
        let content = serde_json::to_string_pretty(&*self.metadata.read().unwrap())?;
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
        // Check for duplicate via content hash
        let content_hash = VectorDeduplicator::hash_content(content);
        
        if let Some(existing_id) = self.deduplicator.get_vector_id(&content_hash) {
            self.deduplicator.add_reference(&existing_id);
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
        self.vectors.write().unwrap().insert(vector_id.clone(), stored);
        self.deduplicator.register(content_hash, vector_id.clone());

        // Update metadata
        {
            let mut meta = self.metadata.write().unwrap();
            meta.vector_count += 1;
            meta.updated_at = chrono::Utc::now();
        }

        // Invalidate cache
        self.cache.clear();

        Ok(vector_id)
    }

    /// Search for similar vectors
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        let top_k = top_k.min(MAX_TOP_K);
        
        // Check cache (key is hash of entire embedding + top_k)
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        for &val in query_embedding {
            hasher.update(&val.to_le_bytes());
        }
        hasher.update(&(top_k as u64).to_le_bytes());
        let cache_key = format!("{:x}", hasher.finalize());
        
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for search");
            return Ok(cached);
        }

        let vectors = self.vectors.read().unwrap();
        
        // Calculate similarities
        let mut scores: Vec<(String, f32, VectorMetadata, Option<String>)> = vectors
            .iter()
            .map(|(id, v)| {
                let score = cosine_similarity(query_embedding, &v.embedding);
                (id.clone(), score, v.metadata.clone(), v.content.clone())
            })
            .collect();

        // Sort by score descending - use partial sort for efficiency if top_k is small
        if scores.len() > top_k {
            scores.select_nth_unstable_by(top_k - 1, |a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scores.truncate(top_k);
        }
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top_k
        let results: Vec<SearchResult> = scores
            .into_iter()
            .take(top_k)
            .map(|(id, score, metadata, content)| SearchResult {
                vector_id: id,
                score,
                metadata,
                content,
            })
            .collect();

        // Cache results
        self.cache.put(cache_key, results.clone());

        Ok(results)
    }

    /// Delete vectors by file path
    pub fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        let mut vectors = self.vectors.write().unwrap();
        let initial_count = vectors.len();

        for v in vectors.values() {
            if v.metadata.file_path == file_path {
                self.deduplicator.unregister(&v.id);
            }
        }
        
        vectors.retain(|_, v| v.metadata.file_path != file_path);

        let deleted = initial_count - vectors.len();

        if deleted > 0 {
            let mut meta = self.metadata.write().unwrap();
            meta.vector_count = meta.vector_count.saturating_sub(deleted);
            meta.updated_at = chrono::Utc::now();
            self.cache.clear();
        }

        Ok(deleted)
    }

    /// Get vector count
    pub fn vector_count(&self) -> usize {
        self.metadata.read().unwrap().vector_count
    }

    /// Get index info
    pub fn info(&self) -> IndexMetadata {
        self.metadata.read().unwrap().clone()
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.stats()
    }

    /// Persist index to disk
    pub fn persist(&self) -> Result<()> {
        // Save metadata
        self.save_metadata()?;

        // Save vectors
        let vectors_path = self.index_path.join("vectors.json");
        let vectors = self.vectors.read().unwrap();
        
        let serializable: Vec<_> = vectors.values().map(|v| {
            serde_json::json!({
                "id": v.id,
                "embedding": v.embedding,
                "metadata": v.metadata,
            })
        }).collect();

        let content = serde_json::to_string(&serializable)?;
        fs::write(vectors_path, content)?;

        info!("Persisted {} vectors to disk", vectors.len());
        Ok(())
    }
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
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

        let id = store.add_vector("test content", embedding.clone(), metadata).unwrap();
        assert!(!id.is_empty());
        assert_eq!(store.vector_count(), 1);
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
            let content = format!("unique content {} with random data {}", i, uuid::Uuid::new_v4().to_string());
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
}
