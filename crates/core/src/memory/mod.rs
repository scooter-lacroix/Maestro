//! Memory Module - Hybrid Memory Providers for Maestro
//!
//! This module provides multiple memory backends:
//! - `TantivyMemory`: A memory backend using Tantivy for full-text search
//! - `LeIndexProvider`: Graph-aware semantic memory with hybrid retrieval
//! - `HybridRanker`: Combines text (BM25) and vector similarity scores
//! - `NexusVectorStore`: HNSW-backed vector storage with graph boosting
//! - `HotCache`: Real-time memory suggestions during agent execution
//! - `EmbeddingService`: Vector embedding generation
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                     Memory System                                  │
//! │                                                                    │
//! │  ┌──────────────────────────────────────────────────────────────┐ │
//! │  │                    Storage Layer                              │ │
//! │  │  ┌─────────────┐  ┌───────────────┐  ┌─────────────────────┐ │ │
//! │  │  │   Tantivy   │  │ NexusVector   │  │   Turso Backend     │ │ │
//! │  │  │ (Full-Text) │  │ Store (HNSW)  │  │   (Persistence)     │ │ │
//! │  │  └──────┬──────┘  └───────┬───────┘  └──────────┬──────────┘ │ │
//! │  └─────────┼─────────────────┼─────────────────────┼────────────┘ │
//! │            │                 │                     │              │
//! │  ┌─────────┼─────────────────┼─────────────────────┼────────────┐ │
//! │  │         │    Processing Layer                   │            │ │
//! │  │  ┌──────▼──────┐  ┌───────▼───────┐  ┌──────────▼──────────┐ │ │
//! │  │  │   Hybrid    │  │   Embedding   │  │     Compression     │ │ │
//! │  │  │   Ranker    │  │   Service     │  │                     │ │ │
//! │  │  └─────────────┘  └───────────────┘  └─────────────────────┘ │ │
//! │  └──────────────────────────────────────────────────────────────┘ │
//! │                                                                    │
//! │  ┌──────────────────────────────────────────────────────────────┐ │
//! │  │                    Cache Layer                                │ │
//! │  │  ┌─────────────────────────────────────────────────────────┐ │ │
//! │  │  │                      Hot Cache                           │ │ │
//! │  │  │  (Semantic Detection + Suggestion Broadcasting)         │ │ │
//! │  │  └─────────────────────────────────────────────────────────┘ │ │
//! │  └──────────────────────────────────────────────────────────────┘ │
//! └───────────────────────────────────────────────────────────────────┘
//! ```

pub mod embedding;
pub mod hybrid;
pub mod leindex_provider;
pub mod nexus_store;
pub mod tantivy;
pub mod types;

// Conditionally compile hot_cache (requires additional features)
#[cfg(feature = "hot-cache")]
pub mod hot_cache;

// Re-exports from types
pub use types::{
    CompressionResult, EmbeddingMetadata, HotCacheConfig, MemoryCategory, MemoryLaneType,
    MemorySuggestion, VectorSearchResult, VectorStoreConfig, DEFAULT_EMBEDDING_MODEL,
    EMBEDDING_DIMENSION,
};

// Re-exports from hybrid
pub use hybrid::{HybridRanker, RankedResult};

// Re-exports from leindex_provider
pub use leindex_provider::{
    GraphAwareSearchResult, LeIndexConfig, LeIndexProvider, SemanticSignal,
};

// Re-exports from tantivy
pub use tantivy::TantivyMemory;

// Re-exports from nexus_store
pub use nexus_store::{cosine_similarity, NexusVectorStore, VectorStoreStats};

// Re-exports from embedding
pub use embedding::{
    EmbeddingConfig, EmbeddingService, EmbeddingStats, BatchProcessor as EmbeddingBatchProcessor,
};

// Conditional re-exports from hot_cache
#[cfg(feature = "hot-cache")]
pub use hot_cache::{
    DetectedPattern, HotCache, HotCacheStats, SemanticDetector,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Memory;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn integration_store_search_cycle() {
        let tmp = TempDir::new().unwrap();
        let memory = TantivyMemory::new(tmp.path()).await.unwrap();

        // Store documents
        memory
            .store("Rust is fast", json!({"lang": "rust"}))
            .await
            .unwrap();
        memory
            .store("Python is readable", json!({"lang": "python"}))
            .await
            .unwrap();

        // Search
        let results = memory.search("Rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata["lang"], "rust");
    }

    #[test]
    fn hybrid_ranker_basic() {
        let ranker = HybridRanker::new(0.5, 0.5);
        let text = vec![("a".to_string(), 0.8)];
        let vector = vec![("a".to_string(), 0.6)];

        let merged = ranker.merge(&text, &vector, 10);
        assert_eq!(merged.len(), 1);
        // Scores are normalized to max=1.0, so both become 1.0
        // Final = 0.5 * 1.0 + 0.5 * 1.0 = 1.0
        assert!((merged[0].final_score - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn nexus_vector_store_basic() {
        let store = NexusVectorStore::in_memory().await.unwrap();
        assert!(store.is_empty().await);

        // Store embedding
        let embedding = vec![0.5; EMBEDDING_DIMENSION];
        store
            .store_embedding(1, embedding.clone(), 1, MemoryCategory::Facts, None, 3)
            .await
            .unwrap();

        assert_eq!(store.len().await, 1);

        // Search
        let results = store.search_similar(&embedding, 1, 10, 0.0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[tokio::test]
    async fn embedding_service_basic() {
        let service = EmbeddingService::mock().await.unwrap();

        let embedding = service.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIMENSION);

        // Check normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn memory_category_parsing() {
        assert_eq!(
            MemoryCategory::from_str("facts"),
            Some(MemoryCategory::Facts)
        );
        assert_eq!(MemoryCategory::from_str("preferences"), Some(MemoryCategory::Preferences));
        assert_eq!(MemoryCategory::from_str("unknown"), None);
    }

    #[test]
    fn memory_lane_type_boost() {
        assert!(MemoryLaneType::Correction.boost_factor() > MemoryLaneType::Reference.boost_factor());
    }

    #[test]
    fn compression_result_basic() {
        let result = CompressionResult::new(
            "This is the original content",
            "Short",
            vec!["key".to_string()],
        );

        assert!(result.compression_ratio < 1.0);
        assert!(result.is_lossy);
    }
}
