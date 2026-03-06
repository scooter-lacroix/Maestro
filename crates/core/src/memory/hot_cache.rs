//! Hot Cache - Semantic detection during agent loop
//!
//! This module provides real-time memory suggestions during agent execution:
//! - Semantic similarity detection on agent output
//! - LRU cache with configurable TTL
//! - Background embedding computation
//! - Subtle suggestion broadcasting to UI
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                        HotCache                                │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │                  Semantic Detector                       │ │
//! │  │   Agent Output ──► Pattern Match ──► Embedding Search   │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! │                           │                                   │
//! │  ┌────────────────────────┼────────────────────────────────┐ │
//! │  │                  Suggestion Buffer                       │ │
//! │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐    │ │
//! │  │  │ Sugg #1 │  │ Sugg #2 │  │ Sugg #3 │  │ Sugg #4 │    │ │
//! │  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘    │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! │                           │                                   │
//! │                   Broadcast Channel                           │
//! │                           │                                   │
//! │                    UI Subscribers                              │
//! └───────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, info, warn};

use super::embedding::EmbeddingService;
use super::types::{
    HotCacheConfig, MemoryCategory, MemoryLaneType, MemorySuggestion, VectorSearchResult,
};

/// Default TTL for suggestions (5 minutes)
const DEFAULT_TTL_SECS: u64 = 300;

/// Maximum broadcast channel capacity
const BROADCAST_CAPACITY: usize = 256;

/// Pattern types for semantic detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedPattern {
    /// Error message pattern
    Error,
    /// Warning pattern
    Warning,
    /// Question pattern
    Question,
    /// Decision pattern
    Decision,
    /// Code change pattern
    CodeChange,
    /// File reference pattern
    FileReference,
    /// Command pattern
    Command,
    /// URL pattern
    Url,
    /// Key-value pattern
    KeyValue,
    /// Todo/list pattern
    Todo,
    /// Context boundary pattern
    ContextBoundary,
}

impl DetectedPattern {
    /// Detect patterns in text
    pub fn detect(text: &str) -> Vec<Self> {
        let mut patterns = Vec::new();
        let lower = text.to_lowercase();

        // Error patterns
        if lower.contains("error") || lower.contains("exception") || lower.contains("failed") {
            patterns.push(Self::Error);
        }

        // Warning patterns
        if lower.contains("warning") || lower.contains("deprecated") || lower.contains("caution") {
            patterns.push(Self::Warning);
        }

        // Question patterns
        if lower.contains("?") || lower.contains("how do i") || lower.contains("what is") {
            patterns.push(Self::Question);
        }

        // Decision patterns
        if lower.contains("decided") || lower.contains("chose") || lower.contains("selected") {
            patterns.push(Self::Decision);
        }

        // Code change patterns
        if text.contains("```") || text.contains("fn ") || text.contains("def ") {
            patterns.push(Self::CodeChange);
        }

        // File reference patterns
        if text.contains('/')
            && (text.contains(".rs") || text.contains(".py") || text.contains(".ts"))
        {
            patterns.push(Self::FileReference);
        }

        // Command patterns
        if text.starts_with('$') || text.starts_with('>') || lower.contains("run ") {
            patterns.push(Self::Command);
        }

        // URL patterns
        if text.contains("http://") || text.contains("https://") {
            patterns.push(Self::Url);
        }

        // Key-value patterns
        if text.contains(':') && text.contains('=') {
            patterns.push(Self::KeyValue);
        }

        // Todo patterns
        if lower.contains("todo") || lower.contains("fixme") || lower.starts_with("- [ ]") {
            patterns.push(Self::Todo);
        }

        // Context boundary patterns
        if text.contains("---") || text.contains("===") || text.starts_with('#') {
            patterns.push(Self::ContextBoundary);
        }

        patterns
    }

    /// Get relevance boost for this pattern
    pub fn relevance_boost(&self) -> f32 {
        match self {
            Self::Error => 1.3,
            Self::Warning => 1.1,
            Self::Question => 1.2,
            Self::Decision => 1.4,
            Self::CodeChange => 1.2,
            Self::FileReference => 1.1,
            Self::Command => 1.0,
            Self::Url => 0.9,
            Self::KeyValue => 1.0,
            Self::Todo => 1.1,
            Self::ContextBoundary => 0.8,
        }
    }
}

/// Hot cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The suggestion
    suggestion: MemorySuggestion,
    /// When it was cached
    cached_at: Instant,
    /// Access count
    access_count: u64,
}

/// Semantic detector for agent output
pub struct SemanticDetector {
    /// Minimum content length to process
    min_length: usize,
    /// Maximum content length to process
    max_length: usize,
    /// Keywords to prioritize
    priority_keywords: Vec<String>,
}

impl Default for SemanticDetector {
    fn default() -> Self {
        Self {
            min_length: 10,
            max_length: 10000,
            priority_keywords: vec![
                "error".to_string(),
                "warning".to_string(),
                "fix".to_string(),
                "implement".to_string(),
                "change".to_string(),
                "refactor".to_string(),
            ],
        }
    }
}

impl SemanticDetector {
    /// Create a new semantic detector
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect if content is worth processing
    pub fn should_process(&self, content: &str) -> bool {
        let len = content.len();
        len >= self.min_length && len <= self.max_length
    }

    /// Extract key phrases from content
    pub fn extract_key_phrases(&self, content: &str) -> Vec<String> {
        let mut phrases = Vec::new();

        // Split into sentences
        for sentence in content.split(&['.', '!', '?', '\n'][..]) {
            let trimmed = sentence.trim();
            if trimmed.len() < 5 || trimmed.len() > 200 {
                continue;
            }

            // Check for priority keywords
            let lower = trimmed.to_lowercase();
            for keyword in &self.priority_keywords {
                if lower.contains(keyword) {
                    phrases.push(trimmed.to_string());
                    break;
                }
            }
        }

        // Limit to top phrases
        phrases.truncate(5);
        phrases
    }

    /// Calculate relevance score for content
    pub fn calculate_relevance(&self, content: &str, patterns: &[DetectedPattern]) -> f32 {
        if content.is_empty() {
            return 0.0;
        }

        // Base score from patterns
        let pattern_score: f32 = patterns.iter().map(|p| p.relevance_boost()).sum::<f32>()
            / (patterns.len().max(1) as f32);

        // Adjust for content length
        let length_factor = if content.len() < 50 {
            0.7
        } else if content.len() < 200 {
            1.0
        } else {
            0.9
        };

        // Check for priority keywords
        let keyword_bonus = self
            .priority_keywords
            .iter()
            .filter(|kw| content.to_lowercase().contains(kw.as_str()))
            .count() as f32
            * 0.1;

        ((pattern_score * length_factor) + keyword_bonus).min(1.0)
    }
}

/// Hot cache for real-time memory suggestions
pub struct HotCache {
    /// Configuration
    config: HotCacheConfig,
    /// Cached suggestions
    cache: Arc<RwLock<HashMap<i64, CacheEntry>>>,
    /// Access order for LRU eviction
    access_order: Arc<RwLock<Vec<i64>>>,
    /// Embedding service
    embedding_service: Arc<EmbeddingService>,
    /// Semantic detector
    detector: SemanticDetector,
    /// Suggestion broadcast sender
    suggestion_tx: broadcast::Sender<MemorySuggestion>,
    /// Background processing sender
    process_tx: Option<mpsc::Sender<ProcessRequest>>,
    /// Statistics
    stats: Arc<RwLock<HotCacheStats>>,
}

/// Background processing request
struct ProcessRequest {
    content: String,
    namespace_id: i64,
}

/// Hot cache statistics
#[derive(Debug, Clone, Default)]
pub struct HotCacheStats {
    /// Total process requests
    pub total_processed: u64,
    /// Total suggestions generated
    pub total_suggestions: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Average processing latency in ms
    pub avg_latency_ms: f64,
}

impl HotCache {
    /// Create a new hot cache
    pub async fn new(
        config: HotCacheConfig,
        embedding_service: Arc<EmbeddingService>,
    ) -> Result<Self> {
        let (suggestion_tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        Ok(Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            embedding_service,
            detector: SemanticDetector::new(),
            suggestion_tx,
            process_tx: None,
            stats: Arc::new(RwLock::new(HotCacheStats::default())),
        })
    }

    /// Process agent output for semantic detection
    pub async fn process_output(
        &self,
        output: &str,
        namespace_id: i64,
    ) -> Result<Vec<MemorySuggestion>> {
        let start = Instant::now();

        // Check if content should be processed
        if !self.detector.should_process(output) {
            return Ok(Vec::new());
        }

        // Detect patterns
        let patterns = DetectedPattern::detect(output);

        // Extract key phrases
        let key_phrases = self.detector.extract_key_phrases(output);

        // Calculate relevance
        let relevance = self.detector.calculate_relevance(output, &patterns);

        if relevance < self.config.min_relevance {
            debug!("Content below relevance threshold: {}", relevance);
            return Ok(Vec::new());
        }

        // Generate embedding for content
        let embedding = self.embedding_service.embed(output).await?;

        // In a full implementation, we would search the vector store here
        // For now, create a mock suggestion
        let suggestion = MemorySuggestion::new(
            0, // Would be actual memory ID from search
            namespace_id,
            output,
            relevance,
            0.5, // Would be actual similarity
            relevance * 1.1,
            MemoryCategory::General,
        );

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_processed += 1;
            stats.total_suggestions += 1;
            let latency = start.elapsed().as_millis() as f64;
            stats.avg_latency_ms = (stats.avg_latency_ms * (stats.total_processed - 1) as f64
                + latency)
                / stats.total_processed as f64;
        }

        // Broadcast suggestion
        if self.config.enable_flash {
            let _ = self.suggestion_tx.send(suggestion.clone());
        }

        debug!(
            "Processed output (relevance={}, latency={:?})",
            relevance,
            start.elapsed()
        );

        Ok(vec![suggestion])
    }

    /// Get current suggestions (non-blocking)
    pub async fn get_suggestions(&self) -> Vec<MemorySuggestion> {
        let cache = self.cache.read().await;
        cache
            .values()
            .filter(|entry| {
                let age = entry.cached_at.elapsed().as_secs();
                age < self.config.ttl_secs
            })
            .map(|entry| entry.suggestion.clone())
            .collect()
    }

    /// Get top N suggestions by relevance
    pub async fn get_top_suggestions(&self, limit: usize) -> Vec<MemorySuggestion> {
        let mut suggestions = self.get_suggestions().await;
        suggestions.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions.truncate(limit);
        suggestions
    }

    /// Subscribe to suggestion updates
    pub fn subscribe(&self) -> broadcast::Receiver<MemorySuggestion> {
        self.suggestion_tx.subscribe()
    }

    /// Add a suggestion to the cache
    pub async fn add_suggestion(&self, suggestion: MemorySuggestion) -> Result<()> {
        let mut cache = self.cache.write().await;
        let mut order = self.access_order.write().await;

        let id = suggestion.memory_id;
        let is_update = cache.contains_key(&id);

        // Check capacity and evict if needed
        if !is_update && cache.len() >= self.config.capacity {
            if let Some(lru_id) = order.first().copied() {
                order.remove(0);
                cache.remove(&lru_id);

                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }

        // Add or update entry
        cache.insert(
            id,
            CacheEntry {
                suggestion,
                cached_at: Instant::now(),
                access_count: 1,
            },
        );

        // Update access order
        if is_update {
            order.retain(|&i| i != id);
        }
        order.push(id);

        Ok(())
    }

    /// Invalidate stale entries
    pub async fn invalidate_stale(&self) {
        let mut cache = self.cache.write().await;
        let mut order = self.access_order.write().await;

        let stale_ids: Vec<i64> = cache
            .iter()
            .filter(|(_, entry)| entry.cached_at.elapsed().as_secs() >= self.config.ttl_secs)
            .map(|(id, _)| *id)
            .collect();

        for id in &stale_ids {
            cache.remove(id);
        }

        order.retain(|id| !stale_ids.contains(id));

        if !stale_ids.is_empty() {
            debug!("Invalidated {} stale entries", stale_ids.len());
        }
    }

    /// Clear all suggestions
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let mut order = self.access_order.write().await;
        cache.clear();
        order.clear();
        info!("Hot cache cleared");
    }

    /// Get cache statistics
    pub async fn stats(&self) -> HotCacheStats {
        self.stats.read().await.clone()
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Mark a suggestion as viewed
    pub async fn mark_viewed(&self, memory_id: i64) {
        let mut cache = self.cache.write().await;
        if let Some(entry) = cache.get_mut(&memory_id) {
            entry.suggestion.mark_viewed();
            entry.access_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_cache() -> HotCache {
        let config = HotCacheConfig::default();
        let embedding_service = Arc::new(EmbeddingService::mock().await.unwrap());
        HotCache::new(config, embedding_service).await.unwrap()
    }

    #[tokio::test]
    async fn test_pattern_detection() {
        let patterns = DetectedPattern::detect("Error: something went wrong");
        assert!(patterns.contains(&DetectedPattern::Error));

        let patterns = DetectedPattern::detect("How do I fix this?");
        assert!(patterns.contains(&DetectedPattern::Question));

        let patterns = DetectedPattern::detect("```rust\nfn main() {}\n```");
        assert!(patterns.contains(&DetectedPattern::CodeChange));
    }

    #[tokio::test]
    async fn test_semantic_detector() {
        let detector = SemanticDetector::new();

        assert!(detector.should_process("This is a test string"));
        assert!(!detector.should_process("short"));
        assert!(!detector.should_process(&"x".repeat(20000)));

        let phrases = detector.extract_key_phrases("Fix this error in the code.");
        assert!(!phrases.is_empty());
    }

    #[tokio::test]
    async fn test_process_output() {
        let cache = create_test_cache().await;

        let suggestions = cache
            .process_output("Error: failed to compile", 1)
            .await
            .unwrap();

        // Should generate at least one suggestion for error content
        // (though with mock vector store it may be empty)
        let stats = cache.stats().await;
        assert!(stats.total_processed > 0);
    }

    #[tokio::test]
    async fn test_low_relevance_rejected() {
        let cache = create_test_cache().await;

        let suggestions = cache
            .process_output("x y z", 1) // Low relevance content
            .await
            .unwrap();

        // Should not generate suggestions for low relevance
        assert!(suggestions.is_empty() || suggestions[0].relevance_score < 0.5);
    }

    #[tokio::test]
    async fn test_add_and_get_suggestions() {
        let cache = create_test_cache().await;

        let suggestion = MemorySuggestion::new(
            123,
            1,
            "test content",
            0.8,
            0.7,
            0.85,
            MemoryCategory::Facts,
        );

        cache.add_suggestion(suggestion).await.unwrap();

        let suggestions = cache.get_suggestions().await;
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].memory_id, 123);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let mut config = HotCacheConfig::default();
        config.capacity = 2;

        let embedding_service = Arc::new(EmbeddingService::mock().await.unwrap());
        let cache = HotCache::new(config, embedding_service).await.unwrap();

        cache
            .add_suggestion(MemorySuggestion::new(
                1,
                1,
                "a",
                0.5,
                0.5,
                0.5,
                MemoryCategory::General,
            ))
            .await
            .unwrap();
        cache
            .add_suggestion(MemorySuggestion::new(
                2,
                1,
                "b",
                0.5,
                0.5,
                0.5,
                MemoryCategory::General,
            ))
            .await
            .unwrap();
        cache
            .add_suggestion(MemorySuggestion::new(
                3,
                1,
                "c",
                0.5,
                0.5,
                0.5,
                MemoryCategory::General,
            ))
            .await
            .unwrap();

        let size = cache.size().await;
        assert_eq!(size, 2);

        let stats = cache.stats().await;
        assert!(stats.evictions >= 1);
    }

    #[tokio::test]
    async fn test_invalidate_stale() {
        let mut config = HotCacheConfig::default();
        config.ttl_secs = 0; // Immediate staleness

        let embedding_service = Arc::new(EmbeddingService::mock().await.unwrap());
        let cache = HotCache::new(config, embedding_service).await.unwrap();

        cache
            .add_suggestion(MemorySuggestion::new(
                1,
                1,
                "test",
                0.5,
                0.5,
                0.5,
                MemoryCategory::General,
            ))
            .await
            .unwrap();

        // Sleep briefly to ensure staleness
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        cache.invalidate_stale().await;

        let suggestions = cache.get_suggestions().await;
        assert!(suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_subscribe() {
        let cache = create_test_cache().await;
        let mut receiver = cache.subscribe();

        let suggestion = MemorySuggestion::new(1, 1, "test", 0.8, 0.7, 0.85, MemoryCategory::Facts);
        cache.add_suggestion(suggestion.clone()).await.unwrap();

        // Manually broadcast for test
        let _ = cache.suggestion_tx.send(suggestion);

        // Should receive the broadcast
        let received = receiver.try_recv();
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_mark_viewed() {
        let cache = create_test_cache().await;

        let suggestion =
            MemorySuggestion::new(1, 1, "test", 0.5, 0.5, 0.5, MemoryCategory::General);
        cache.add_suggestion(suggestion).await.unwrap();

        assert!(cache.get_suggestions().await[0].is_new);

        cache.mark_viewed(1).await;

        assert!(!cache.get_suggestions().await[0].is_new);
    }

    #[tokio::test]
    async fn test_get_top_suggestions() {
        let cache = create_test_cache().await;

        cache
            .add_suggestion(MemorySuggestion::new(
                1,
                1,
                "a",
                0.5,
                0.5,
                0.5,
                MemoryCategory::General,
            ))
            .await
            .unwrap();
        cache
            .add_suggestion(MemorySuggestion::new(
                2,
                1,
                "b",
                0.9,
                0.9,
                0.9,
                MemoryCategory::General,
            ))
            .await
            .unwrap();
        cache
            .add_suggestion(MemorySuggestion::new(
                3,
                1,
                "c",
                0.7,
                0.7,
                0.7,
                MemoryCategory::General,
            ))
            .await
            .unwrap();

        let top = cache.get_top_suggestions(2).await;
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].memory_id, 2); // Highest relevance
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = create_test_cache().await;

        cache
            .add_suggestion(MemorySuggestion::new(
                1,
                1,
                "test",
                0.5,
                0.5,
                0.5,
                MemoryCategory::General,
            ))
            .await
            .unwrap();
        assert!(!cache.get_suggestions().await.is_empty());

        cache.clear().await;
        assert!(cache.get_suggestions().await.is_empty());
    }
}
