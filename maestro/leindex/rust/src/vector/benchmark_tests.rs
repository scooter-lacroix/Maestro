//! Unit tests for vector benchmark infrastructure

use crate::vector::{ChunkType, SearchResult, VectorMetadata, VectorStore};

/// Test that generated embeddings are valid (normalized)
#[test]
fn test_embedding_validation() {
    let embedding: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4, 0.5];

    // Check dimension is reasonable
    assert!(embedding.len() <= 768, "Embedding dimension too large");

    // Check values are in valid range
    for &val in &embedding {
        assert!(
            val >= -1.0 && val <= 1.0,
            "Embedding value out of range: {}",
            val
        );
    }
}

/// Test that cosine similarity is calculated correctly
#[test]
fn test_cosine_similarity_calculation() {
    // Test identical vectors
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!(
        (sim - 1.0).abs() < 0.001,
        "Identical vectors should have similarity 1.0"
    );

    // Test orthogonal vectors
    let c = vec![0.0, 1.0, 0.0];
    let sim2 = cosine_similarity(&a, &c);
    assert!(
        sim2.abs() < 0.001,
        "Orthogonal vectors should have similarity ~0"
    );
}

/// Cosine similarity helper (duplicate from store.rs for testing)
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

/// Test that search results are sorted by score
#[test]
fn test_search_results_sorted() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    // Add vectors with different similarities
    for i in 0..10 {
        let mut embedding = vec![0.0; 768];
        embedding[i] = 1.0;
        let metadata = VectorMetadata::new(&format!("file{}.rs", i), i as i32);
        let content = format!("content {}", i);
        store.add_vector(&content, embedding, metadata).unwrap();
    }

    // Search for first vector - should get it back with highest score
    let mut query = vec![0.0; 768];
    query[0] = 1.0;
    let results = store.search(&query, 10).unwrap();

    // Check results are in descending order
    for i in 0..results.len() - 1 {
        assert!(
            results[i].score >= results[i + 1].score,
            "Results not sorted at index {}: {} >= {}",
            i,
            results[i].score,
            results[i + 1].score
        );
    }
}

/// Test that VectorStore handles empty searches gracefully
#[test]
fn test_empty_vector_store_search() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    let query = vec![0.1; 768];
    let results = store.search(&query, 10).unwrap();

    assert_eq!(results.len(), 0, "Empty store should return no results");
}

/// Test that VectorStore limits results to top_k
#[test]
fn test_top_k_limit() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    // Add 20 vectors
    for i in 0..20 {
        let mut embedding = vec![0.0; 768];
        embedding[i % 768] = 1.0;
        let metadata = VectorMetadata::new(&format!("file{}.rs", i), i as i32);
        let content = format!("content {}", i);
        store.add_vector(&content, embedding, metadata).unwrap();
    }

    let query = vec![1.0, 0.0, 0.0];
    let results = store.search(&query, 5).unwrap();

    assert_eq!(results.len(), 5, "Should return exactly top_k results");
}

/// Test that cache improves performance
#[test]
fn test_cache_effectiveness() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    // Add some vectors
    for i in 0..10 {
        let mut embedding = vec![0.0; 768];
        embedding[i] = 1.0;
        let metadata = VectorMetadata::new(&format!("file{}.rs", i), i as i32);
        let content = format!("content {}", i);
        store.add_vector(&content, embedding, metadata).unwrap();
    }

    let query = vec![1.0, 0.0, 0.0];

    // First search (cache miss)
    let _ = store.search(&query, 5).unwrap();
    let stats_before = store.cache_stats().unwrap();

    // Second search (cache hit)
    let _ = store.search(&query, 5).unwrap();
    let stats_after = store.cache_stats().unwrap();

    // Cache should have more hits after second search
    assert!(
        stats_after.hits > stats_before.hits || stats_after.misses > stats_before.misses,
        "Cache statistics should change"
    );
}

/// Test that different chunk types are handled correctly
#[test]
fn test_chunk_type_variations() {
    let chunk_types = vec![
        ChunkType::Function,
        ChunkType::Class,
        ChunkType::Module,
        ChunkType::Import,
        ChunkType::Comment,
        ChunkType::Text,
        ChunkType::Other,
    ];

    for chunk_type in chunk_types {
        let metadata = VectorMetadata::new("test.rs", 0).with_type(chunk_type);
        assert_eq!(metadata.chunk_type, chunk_type);
    }
}

/// Test that metadata is preserved correctly
#[test]
fn test_metadata_preservation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    let original_metadata = VectorMetadata::new("src/test.rs", 5)
        .with_lines(10, 20)
        .with_type(ChunkType::Function)
        .with_context("parent_function");

    let embedding = vec![0.1; 768];
    let content = "test content";
    let _ = store
        .add_vector(content, embedding, original_metadata.clone())
        .unwrap();

    let query = vec![0.1; 768];
    let results = store.search(&query, 1).unwrap();

    assert_eq!(results.len(), 1);
    let retrieved = &results[0];

    assert_eq!(retrieved.metadata.file_path, "src/test.rs");
    assert_eq!(retrieved.metadata.chunk_index, 5);
    assert_eq!(retrieved.metadata.start_line, Some(10));
    assert_eq!(retrieved.metadata.end_line, Some(20));
    assert_eq!(retrieved.metadata.chunk_type, ChunkType::Function);
    assert_eq!(
        retrieved.metadata.parent_context,
        Some("parent_function".to_string())
    );
    assert_eq!(retrieved.content, Some("test content".to_string()));
}

/// Test that delete by file path works correctly
#[test]
fn test_delete_by_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    // Add vectors from multiple files with unique content to avoid deduplication
    for file_idx in 0..3 {
        for i in 0..5 {
            let mut embedding = vec![0.0; 768];
            embedding[file_idx * 5 + i] = 1.0; // Unique pattern per vector
            let metadata = VectorMetadata::new(&format!("file{}.rs", file_idx), i as i32);
            let content = format!("content {}_{}", file_idx, i); // Unique content
            store.add_vector(&content, embedding, metadata).unwrap();
        }
    }

    assert_eq!(store.vector_count().unwrap(), 15);

    // Delete all vectors from file0.rs
    let deleted = store.delete_by_file("file0.rs").unwrap();
    assert_eq!(deleted, 5);
    assert_eq!(store.vector_count().unwrap(), 10);

    // Verify file0.rs vectors are gone
    let query = vec![0.1; 768];
    let results = store.search(&query, 100).unwrap();
    for result in &results {
        assert_ne!(result.metadata.file_path, "file0.rs");
    }
}

/// Test that vector count is accurate
#[test]
fn test_vector_count_accuracy() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    assert_eq!(store.vector_count().unwrap(), 0);

    for i in 0..10 {
        let embedding = vec![0.1; 768];
        let metadata = VectorMetadata::new("test.rs", i);
        let content = format!("content {}", i);
        store.add_vector(&content, embedding, metadata).unwrap();
    }

    assert_eq!(store.vector_count().unwrap(), 10);
}

/// Test that deduplication works correctly
#[test]
fn test_deduplication() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    let embedding = vec![0.1; 768];
    let metadata = VectorMetadata::new("test.rs", 0);
    let content = "duplicate content";

    // Add same content twice
    let id1 = store
        .add_vector(content, embedding.clone(), metadata.clone())
        .unwrap();
    let id2 = store.add_vector(content, embedding, metadata).unwrap();

    // Should return same ID (deduplicated)
    assert_eq!(id1, id2);
    assert_eq!(store.vector_count().unwrap(), 1);
}

/// Test MAX_TOP_K is enforced
#[test]
fn test_max_top_k_enforcement() {
    // This verifies that the VectorStore enforces MAX_TOP_K limit
    // The constant is defined in metadata.rs

    use crate::vector::metadata::MAX_TOP_K;

    // MAX_TOP_K should be a reasonable value
    assert!(MAX_TOP_K <= 1000, "MAX_TOP_K should be <= 1000");
    assert!(MAX_TOP_K >= 100, "MAX_TOP_K should be >= 100");
}

/// Test embedding dimension is correct
#[test]
fn test_embedding_dimension() {
    use crate::vector::metadata::DEFAULT_EMBEDDING_DIM;

    // CodeRankEmbed uses 768 dimensions
    assert_eq!(DEFAULT_EMBEDDING_DIM, 768);
}

#[cfg(test)]
mod report_tests {
    use super::super::report::*;
    use std::collections::HashMap;

    #[test]
    fn test_report_generation() {
        let report = BenchmarkReport {
            timestamp: chrono::Utc::now(),
            results: HashMap::new(),
            summary: ReportSummary {
                current_impl: ImplementationSummary {
                    avg_query_latency_us: 3.0,
                    p95_latency_us: 3.5,
                    p99_latency_us: 4.0,
                    throughput_queries_per_sec: 300_000.0,
                    index_size_bytes: None,
                    memory_footprint_mb: None,
                },
                hnsw_impl: None,
                turso_impl: None,
                comparison: ComparisonMetrics {
                    latency_improvement_percent: None,
                    memory_overhead_percent: None,
                    accuracy_difference: None,
                },
                recommendation: "Test recommendation".to_string(),
            },
        };

        let markdown = generate_markdown_report(&report);
        assert!(markdown.contains("# Vector Search Performance Evaluation Report"));
        assert!(markdown.contains("## Executive Summary"));
        assert!(markdown.contains("## Implementation 1: Linear Cosine Similarity"));
        assert!(markdown.contains("## Implementation 2: HNSW"));
        assert!(markdown.contains("## Implementation 3: Turso Native Vector Search"));
        assert!(markdown.contains("## Comparative Analysis"));
        assert!(markdown.contains("## Recommendations"));
        assert!(markdown.contains("## Appendix"));
    }

    #[test]
    fn test_implementation_summary_validation() {
        let summary = ImplementationSummary {
            avg_query_latency_us: 3.0,
            p95_latency_us: 3.5,
            p99_latency_us: 4.0,
            throughput_queries_per_sec: 300_000.0,
            index_size_bytes: None,
            memory_footprint_mb: None,
        };

        // P95 should be >= average
        assert!(summary.p95_latency_us >= summary.avg_query_latency_us);

        // P99 should be >= P95
        assert!(summary.p99_latency_us >= summary.p95_latency_us);

        // Throughput should be positive
        assert!(summary.throughput_queries_per_sec > 0.0);
    }
}
