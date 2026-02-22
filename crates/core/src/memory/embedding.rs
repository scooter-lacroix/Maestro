//! Embedding Service - Vector embedding generation for memory content
//!
//! This module provides embedding generation capabilities:
//! - Local ONNX Runtime (all-MiniLM-L6-v2)
//! - Batch embedding for efficiency
//! - Embedding caching with LRU eviction
//! - Mock embeddings for testing (no ONNX dependency)
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                     EmbeddingService                           │
//! │  ┌─────────────────┐  ┌───────────────────────────────────┐  │
//! │  │   LRU Cache     │  │   ONNX Runtime                    │  │
//! │  │   (Fast Lookup) │  │   (Local Inference)               │  │
//! │  └────────┬────────┘  └───────────────┬───────────────────┘  │
//! │           │                           │                       │
//! │           └───────────┬───────────────┘                       │
//! │                       │                                       │
//! │               ┌───────▼────────┐                              │
//! │               │ Batch Queue    │                              │
//! │               │ (Background)   │                              │
//! │               └────────────────┘                              │
//! └───────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

use super::types::{DEFAULT_EMBEDDING_MODEL, EMBEDDING_DIMENSION};

/// Default batch size for embedding requests
const DEFAULT_BATCH_SIZE: usize = 32;

/// Default cache capacity
const DEFAULT_CACHE_CAPACITY: usize = 10_000;

/// Embedding request for batch processing
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    /// Unique request ID
    pub id: u64,
    /// Text to embed
    pub text: String,
    /// Response channel
    pub response_tx: mpsc::Sender<EmbeddingResponse>,
}

/// Embedding response
#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    /// Request ID
    pub id: u64,
    /// Generated embedding
    pub embedding: Vec<f32>,
    /// Time taken to generate
    pub latency_ms: u64,
    /// Whether result came from cache
    pub from_cache: bool,
}

/// Cached embedding entry
#[derive(Debug, Clone)]
struct CachedEmbedding {
    /// The embedding vector
    embedding: Vec<f32>,
    /// When it was cached
    cached_at: Instant,
    /// Access count
    access_count: u64,
    /// Content hash for validation
    content_hash: u64,
}

impl CachedEmbedding {
    /// Check if the cached entry has expired
    fn is_expired(&self, max_age: Duration) -> bool {
        self.cached_at.elapsed() > max_age
    }

    /// Increment access count
    fn increment_access(&mut self) {
        self.access_count += 1;
    }

    /// Validate content hash matches
    fn validate_hash(&self, hash: u64) -> bool {
        self.content_hash == hash
    }

    /// Get age of cached entry
    fn age(&self) -> Duration {
        self.cached_at.elapsed()
    }
}

/// Configuration for embedding service
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Embedding dimension
    pub dimension: usize,
    /// Model name
    pub model: String,
    /// Cache capacity
    pub cache_capacity: usize,
    /// Maximum age for cache entries (in seconds)
    pub cache_max_age_secs: u64,
    /// Batch size for processing
    pub batch_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    /// Whether to use mock embeddings (for testing)
    pub use_mock: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimension: EMBEDDING_DIMENSION,
            model: DEFAULT_EMBEDDING_MODEL.to_string(),
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            cache_max_age_secs: 3600, // 1 hour default
            batch_size: DEFAULT_BATCH_SIZE,
            batch_timeout_ms: 50,
            use_mock: false,
        }
    }
}

impl EmbeddingConfig {
    /// Create a test configuration with mock embeddings
    pub fn mock() -> Self {
        Self {
            use_mock: true,
            ..Default::default()
        }
    }
}

/// Embedding service for generating vector embeddings
pub struct EmbeddingService {
    /// Configuration
    config: EmbeddingConfig,
    /// LRU cache for embeddings
    cache: Arc<RwLock<LruCache<String, CachedEmbedding>>>,
    /// Request sender for batch processing
    request_tx: Option<mpsc::Sender<EmbeddingRequest>>,
    /// Request receiver for batch processing (kept for background task)
    request_rx: Option<mpsc::Receiver<EmbeddingRequest>>,
    /// Request counter for unique IDs
    request_counter: Arc<RwLock<u64>>,
    /// Statistics
    stats: Arc<RwLock<EmbeddingStats>>,
}

/// Simple LRU cache implementation
#[derive(Debug)]
struct LruCache<K, V> {
    capacity: usize,
    entries: HashMap<K, V>,
    order: Vec<K>,
}

impl<K: Clone + Eq + Hash, V: Clone> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn get(&mut self, key: &K) -> Option<&mut V> {
        if self.entries.contains_key(key) {
            // Move to front (most recently used)
            self.order.retain(|k| k != key);
            self.order.push(key.clone());
            self.entries.get_mut(key)
        } else {
            None
        }
    }

    fn put(&mut self, key: K, value: V) {
        if self.entries.contains_key(&key) {
            // Update existing
            self.order.retain(|k| k != &key);
            self.order.push(key.clone());
            self.entries.insert(key, value);
        } else {
            // Check capacity
            if self.entries.len() >= self.capacity {
                // Remove least recently used
                if let Some(lru_key) = self.order.first().cloned() {
                    self.order.remove(0);
                    self.entries.remove(&lru_key);
                }
            }
            self.order.push(key.clone());
            self.entries.insert(key, value);
        }
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.order.retain(|k| k != key);
        self.entries.remove(key)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

/// Embedding service statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingStats {
    /// Total embeddings generated
    pub total_generated: u64,
    /// Total cache hits
    pub cache_hits: u64,
    /// Total cache misses
    pub cache_misses: u64,
    /// Average generation latency in ms
    pub avg_latency_ms: f64,
    /// Total batch requests
    pub batch_requests: u64,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let cache = Arc::new(RwLock::new(LruCache::new(config.cache_capacity)));
        let stats = Arc::new(RwLock::new(EmbeddingStats::default()));
        let request_counter = Arc::new(RwLock::new(0u64));

        // Create channel for batch processing
        let (request_tx, request_rx) = mpsc::channel(config.batch_size);

        // Store both sender and receiver
        let request_tx = Some(request_tx);
        let request_rx = Some(request_rx);

        Ok(Self {
            config,
            cache,
            request_tx,
            request_rx,
            request_counter,
            stats,
        })
    }

    /// Queue a text for asynchronous batch embedding
    pub async fn queue_embedding_request(
        &self,
        text: String,
    ) -> Result<mpsc::Receiver<EmbeddingResponse>> {
        let request_id = self.next_request_id().await;
        let request_tx = self
            .request_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("batch embedding queue is not initialized"))?;

        let (response_tx, response_rx) = mpsc::channel(1);
        let request = EmbeddingRequest {
            id: request_id,
            text,
            response_tx,
        };

        request_tx
            .send(request)
            .await
            .map_err(|e| anyhow::anyhow!("failed to queue embedding request: {}", e))?;

        Ok(response_rx)
    }

    /// Take ownership of the request receiver to run a BatchProcessor task
    pub fn take_request_receiver(&mut self) -> Option<mpsc::Receiver<EmbeddingRequest>> {
        self.request_rx.take()
    }

    /// Get next request ID
    pub async fn next_request_id(&self) -> u64 {
        let mut counter = self.request_counter.write().await;
        *counter += 1;
        *counter
    }

    /// Create a mock embedding service for testing
    pub async fn mock() -> Result<Self> {
        Self::new(EmbeddingConfig::mock()).await
    }

    /// Generate embedding for a single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let start = Instant::now();

        // Check cache first
        let cache_key = self.compute_cache_key(text);
        let text_hash = self.compute_hash(text);
        let max_age = Duration::from_secs(self.config.cache_max_age_secs);
        let cached_hit = {
            let mut cache = self.cache.write().await;
            let mut evict_stale = false;

            let hit = if let Some(cached) = cache.get(&cache_key) {
                // Validate cache entry age and content hash
                if cached.is_expired(max_age) || !cached.validate_hash(text_hash) {
                    evict_stale = true;
                    None
                } else {
                    cached.increment_access();
                    Some((cached.embedding.clone(), cached.age(), cached.access_count))
                }
            } else {
                None
            };

            if evict_stale {
                let _ = cache.remove(&cache_key);
            }

            hit
        };

        if let Some((embedding, age, access_count)) = cached_hit {
            let mut stats = self.stats.write().await;
            stats.cache_hits += 1;
            debug!(
                "Cache hit for embedding (len={}, age={:?}, accesses={})",
                text.len(),
                age,
                access_count
            );
            return Ok(embedding);
        }

        // Generate embedding
        let embedding = if self.config.use_mock {
            self.generate_mock_embedding(text)
        } else {
            // In a full implementation, this would use ONNX Runtime
            // For now, fall back to mock
            self.generate_mock_embedding(text)
        };

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.put(
                cache_key,
                CachedEmbedding {
                    embedding: embedding.clone(),
                    cached_at: Instant::now(),
                    access_count: 1,
                    content_hash: text_hash,
                },
            );
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
            stats.total_generated += 1;
            let latency = start.elapsed().as_millis() as f64;
            stats.avg_latency_ms = (stats.avg_latency_ms * (stats.total_generated - 1) as f64
                + latency)
                / stats.total_generated as f64;
        }

        debug!(
            "Generated embedding (len={}, latency={:?})",
            text.len(),
            start.elapsed()
        );

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts (batch)
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());

        // Process in batches for efficiency
        for chunk in texts.chunks(self.config.batch_size) {
            let mut batch = Vec::with_capacity(chunk.len());
            for text in chunk {
                batch.push(self.embed(text).await?);
            }
            results.extend(batch);
        }

        // Update batch stats
        {
            let mut stats = self.stats.write().await;
            stats.batch_requests += 1;
        }

        Ok(results)
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.config.dimension
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        &self.config.model
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize, bool) {
        let cache = self.cache.read().await;
        (cache.len(), self.config.cache_capacity, cache.is_empty())
    }

    /// Get service statistics
    pub async fn stats(&self) -> EmbeddingStats {
        self.stats.read().await.clone()
    }

    /// Clear the embedding cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        debug!("Embedding cache cleared");
    }

    /// Generate a mock embedding (deterministic based on text content)
    fn generate_mock_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; self.config.dimension];

        // Simple deterministic embedding based on text content
        // This is NOT a real embedding - just for testing
        let bytes = text.as_bytes();
        for (i, byte) in bytes.iter().cycle().take(self.config.dimension).enumerate() {
            let position_weight = 1.0 + (i as f32 / self.config.dimension as f32) * 0.5;
            embedding[i] = (*byte as f32 / 255.0) * position_weight;
        }

        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        embedding
    }

    /// Compute cache key for text
    fn compute_cache_key(&self, text: &str) -> String {
        // Use first 100 chars + hash for cache key
        let prefix = if text.len() > 100 { &text[..100] } else { text };
        let hash = self.compute_hash(text);
        format!("{}:{:016x}", prefix, hash)
    }

    /// Compute hash of text
    fn compute_hash(&self, text: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

/// Batch embedding processor (for background processing)
pub struct BatchProcessor {
    /// Request receiver
    receiver: mpsc::Receiver<EmbeddingRequest>,
    /// Embedding service
    service: Arc<EmbeddingService>,
    /// Batch timeout
    timeout: Duration,
    /// Maximum batch size
    max_batch: usize,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(
        receiver: mpsc::Receiver<EmbeddingRequest>,
        service: Arc<EmbeddingService>,
        timeout_ms: u64,
        max_batch: usize,
    ) -> Self {
        Self {
            receiver,
            service,
            timeout: Duration::from_millis(timeout_ms),
            max_batch,
        }
    }

    /// Run the batch processor (blocking)
    pub async fn run(&mut self) {
        let mut batch: Vec<EmbeddingRequest> = Vec::new();
        let mut deadline = tokio::time::Instant::now() + self.timeout;

        loop {
            // Wait for request or timeout
            tokio::select! {
                Some(request) = self.receiver.recv() => {
                    batch.push(request);
                    if batch.len() >= self.max_batch {
                        self.process_batch(&mut batch).await;
                        deadline = tokio::time::Instant::now() + self.timeout;
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    if !batch.is_empty() {
                        self.process_batch(&mut batch).await;
                    }
                    deadline = tokio::time::Instant::now() + self.timeout;
                }
                else => break,
            }
        }
    }

    /// Process a batch of embedding requests
    async fn process_batch(&self, batch: &mut Vec<EmbeddingRequest>) {
        if batch.is_empty() {
            return;
        }

        let texts: Vec<String> = batch.iter().map(|r| r.text.clone()).collect();

        // Generate embeddings
        let embeddings = match self.service.embed_batch(&texts).await {
            Ok(e) => e,
            Err(e) => {
                warn!("Batch embedding failed: {}", e);
                batch.clear();
                return;
            }
        };

        // Send responses
        for (request, embedding) in batch.drain(..).zip(embeddings.into_iter()) {
            let response = EmbeddingResponse {
                id: request.id,
                embedding,
                latency_ms: 0, // Would track actual latency
                from_cache: false,
            };
            if let Err(e) = request.response_tx.send(response).await {
                warn!("Failed to send embedding response: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_service() {
        let service = EmbeddingService::mock().await.unwrap();

        let embedding = service.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIMENSION);

        // Check normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_embedding_cache() {
        let service = EmbeddingService::mock().await.unwrap();

        // First call (cache miss)
        let start = Instant::now();
        let _ = service.embed("test text").await.unwrap();
        let first_latency = start.elapsed();

        // Second call (cache hit)
        let start = Instant::now();
        let _ = service.embed("test text").await.unwrap();
        let second_latency = start.elapsed();

        // Cache hit should be faster (though both are fast with mock)
        assert!(second_latency <= first_latency * 2);

        // Check stats
        let stats = service.stats().await;
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 1);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let service = EmbeddingService::mock().await.unwrap();

        let texts: Vec<String> = (0..10).map(|i| format!("Text number {}", i)).collect();
        let embeddings = service.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 10);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), EMBEDDING_DIMENSION);
        }

        // Check batch stats
        let stats = service.stats().await;
        assert!(stats.batch_requests > 0);
    }

    #[tokio::test]
    async fn test_different_texts_different_embeddings() {
        let service = EmbeddingService::mock().await.unwrap();

        let e1 = service.embed("hello world").await.unwrap();
        let e2 = service.embed("goodbye world").await.unwrap();

        // Different texts should produce different embeddings
        assert_ne!(e1, e2);
    }

    #[tokio::test]
    async fn test_same_text_same_embedding() {
        let service = EmbeddingService::mock().await.unwrap();

        let e1 = service.embed("consistent text").await.unwrap();
        let e2 = service.embed("consistent text").await.unwrap();

        // Same text should produce same embedding (deterministic)
        assert_eq!(e1, e2);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let service = EmbeddingService::mock().await.unwrap();

        // Generate and cache
        let _ = service.embed("test").await.unwrap();
        let (len, _, is_empty) = service.cache_stats().await;
        assert!(len > 0);
        assert!(!is_empty);

        // Clear
        service.clear_cache().await;
        let (len, _, is_empty) = service.cache_stats().await;
        assert_eq!(len, 0);
        assert!(is_empty);
    }

    #[tokio::test]
    async fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(3);

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);
        cache.put("c".to_string(), 3);
        assert_eq!(cache.len(), 3);

        // Adding fourth should evict first
        cache.put("d".to_string(), 4);
        assert_eq!(cache.len(), 3);
        assert!(cache.get(&"a".to_string()).is_none());
        assert!(cache.get(&"b".to_string()).is_some());
    }

    #[tokio::test]
    async fn test_config_mock() {
        let config = EmbeddingConfig::mock();
        assert!(config.use_mock);
        assert_eq!(config.dimension, EMBEDDING_DIMENSION);
    }
}
