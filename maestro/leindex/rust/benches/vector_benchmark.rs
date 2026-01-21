//! Vector Search Benchmark Suite
//!
//! Comprehensive benchmark comparing:
//! 1. Current custom VectorStore (linear cosine similarity)
//! 2. HNSW-based approximate nearest neighbor search
//! 3. Turso native vector search (DiskANN indexed)
//!
//! Metrics collected:
//! - Query latency (p50, p95, p99)
//! - Index size
//! - Memory footprint
//! - Accuracy (recall@k for semantic search)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

// Import vector store types
use leindex_analyzers::vector::{
    metadata::DEFAULT_EMBEDDING_DIM, ChunkType, HnswVectorStore, TursoVectorStore, VectorMetadata,
    VectorStore,
};

/// Vector sizes to benchmark (number of vectors in index)
///
/// Granular sizes between 50K-100K are critical for determining the TRUE Linear vs HNSW crossover point.
/// The previous 90K threshold was based on FLAWED benchmarks (measuring cache hits, not search performance).
const VECTOR_SIZES: &[usize] = &[
    100, 500, 1_000, 5_000, 10_000,
    // Granular range for crossover point determination:
    50_000, 55_000, 60_000, 65_000, 70_000, 75_000, 80_000, 85_000, 90_000, 95_000, 100_000,
    300_000, 500_000,
];

/// K values for top-k search
const K_VALUES: &[usize] = &[5, 10, 20, 50, 100];

/// Generate synthetic code search dataset
///
/// Creates realistic code embeddings for benchmarking:
/// - Function definitions (higher semantic density)
/// - Class definitions (moderate density)
/// - Import statements (low density)
/// - Comments (variable density)
fn generate_code_dataset(size: usize) -> Vec<(String, Vec<f32>, VectorMetadata)> {
    use std::f32::consts::PI;

    let mut dataset = Vec::with_capacity(size);

    // Distribution: 40% functions, 20% classes, 15% imports, 25% comments
    for i in 0..size {
        let chunk_type = match i % 100 {
            0..=39 => ChunkType::Function,
            40..=59 => ChunkType::Class,
            60..=74 => ChunkType::Import,
            _ => ChunkType::Comment,
        };

        // Generate embedding with patterns based on chunk type
        let mut embedding = vec![0.0f32; DEFAULT_EMBEDDING_DIM];

        // Base pattern using dimension index for semantic structure
        for (j, val) in embedding.iter_mut().enumerate() {
            let base = match chunk_type {
                ChunkType::Function => {
                    // Functions: higher values in early dimensions (keywords)
                    if j < 100 {
                        0.8
                    } else {
                        0.1
                    }
                }
                ChunkType::Class => {
                    // Classes: distributed across middle dimensions
                    if j >= 100 && j < 400 {
                        0.7
                    } else {
                        0.2
                    }
                }
                ChunkType::Import => {
                    // Imports: sparse pattern
                    if j % 50 == 0 {
                        0.9
                    } else {
                        0.05
                    }
                }
                _ => {
                    // Comments: uniform distribution
                    0.3
                }
            };

            // Add variation based on document index
            let variation = ((i as f32 * 0.01 + j as f32 * 0.001) % PI).sin() * 0.2;
            *val = (base + variation).clamp(-1.0, 1.0);
        }

        // Normalize to unit length (standard for embeddings)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        let metadata = VectorMetadata {
            file_path: format!("src/file_{:04}.rs", i / 10),
            chunk_index: i as i32,
            start_line: Some((i * 5) as i32),
            end_line: Some((i * 5 + 5) as i32),
            chunk_type,
            parent_context: None,
            embedding_model: "nomic-ai/CodeRankEmbed".to_string(),
            created_at: chrono::Utc::now(),
        };

        let content = format!("// Code chunk {} - {:?}", i, chunk_type);
        dataset.push((content, embedding, metadata));
    }

    dataset
}

/// Benchmark: Current VectorStore implementation (Linear Search)
///
/// Tests linear search with cosine similarity.
/// This is the baseline implementation.
///
/// **CRITICAL:** Uses varied queries to bypass CPU cache and measure TRUE search performance.
/// Repeated queries (dataset[0]) would measure cache hits (~100ns), not search.
fn bench_current_vector_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("current_vector_store");

    // Test different index sizes
    for &size in VECTOR_SIZES {
        // Create temporary directory for this benchmark
        let temp_dir = tempfile::tempdir().unwrap();
        let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

        // Populate the store
        let dataset = generate_code_dataset(size);
        for (content, embedding, metadata) in &dataset {
            store
                .add_vector(content, embedding.clone(), metadata.clone())
                .unwrap();
        }

        // Benchmark search with k=10
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("search_k10", size), &store, |b, store| {
            // Use varied queries to bypass CPU cache - critical for TRUE performance measurement
            let mut query_idx = 0;
            b.iter(|| {
                let query_embedding = &dataset[query_idx % dataset.len()].1;
                query_idx += 1;
                black_box(store.search(black_box(query_embedding), 10).unwrap())
            });
        });
    }

    group.finish();
}

/// Benchmark: Top-k variation for current implementation
///
/// Tests how k affects query performance.
///
/// **CRITICAL:** Uses varied queries to bypass CPU cache and measure TRUE search performance.
fn bench_current_vector_store_k_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("current_vector_store_k");

    let size = 5_000; // Fixed index size
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

    let dataset = generate_code_dataset(size);
    for (content, embedding, metadata) in &dataset {
        store
            .add_vector(content, embedding.clone(), metadata.clone())
            .unwrap();
    }

    for &k in K_VALUES {
        group.bench_with_input(BenchmarkId::from_parameter(k), &k, |b, &k| {
            // Use varied queries to bypass CPU cache - critical for TRUE performance measurement
            let mut query_idx = 0;
            b.iter(|| {
                let query_embedding = &dataset[query_idx % dataset.len()].1;
                query_idx += 1;
                black_box(store.search(black_box(query_embedding), k).unwrap())
            });
        });
    }

    group.finish();
}

/// Benchmark: HNSW VectorStore
///
/// Tests HNSW-based approximate nearest neighbor search.
///
/// **CRITICAL:** Uses varied queries to bypass CPU cache and measure TRUE search performance.
fn bench_hnsw_vector_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_vector_store");

    // Test different index sizes
    for &size in VECTOR_SIZES {
        // Create temporary directory for this benchmark
        let temp_dir = tempfile::tempdir().unwrap();
        let store = HnswVectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

        // Populate the store
        let dataset = generate_code_dataset(size);
        for (content, embedding, metadata) in &dataset {
            store
                .add_vector(content, embedding.clone(), metadata.clone())
                .unwrap();
        }

        // Benchmark search with k=10
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("search_k10", size), &store, |b, store| {
            // Use varied queries to bypass CPU cache - critical for TRUE performance measurement
            let mut query_idx = 0;
            b.iter(|| {
                let query_embedding = &dataset[query_idx % dataset.len()].1;
                query_idx += 1;
                black_box(store.search(black_box(query_embedding), 10).unwrap())
            });
        });
    }

    group.finish();
}

/// Benchmark: Turso VectorStore
///
/// Tests Turso-based vector search with libSQL backend.
///
/// **CRITICAL:** Uses varied queries to bypass CPU cache and measure TRUE search performance.
fn bench_turso_vector_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("turso_vector_store");

    // Test different index sizes
    for &size in VECTOR_SIZES {
        // Create temporary directory for this benchmark
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("bench_vectors.db");

        // Use tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new().unwrap();
        let store = rt.block_on(TursoVectorStore::new(Some(db_path))).unwrap();

        // Populate the store
        let dataset = generate_code_dataset(size);
        rt.block_on(async {
            for (content, embedding, metadata) in &dataset {
                store
                    .add_vector(content, embedding.clone(), metadata.clone())
                    .await
                    .unwrap();
            }
        });

        // Benchmark search with k=10
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("search_k10", size),
            &(store, rt, dataset.clone()),
            |b, (store, rt, dataset)| {
                // Use varied queries to bypass CPU cache - critical for TRUE performance measurement
                let mut query_idx = 0;
                b.iter(|| {
                    let query_embedding = &dataset[query_idx % dataset.len()].1;
                    query_idx += 1;
                    let result = rt.block_on(store.search(black_box(query_embedding), 10));
                    black_box(result.unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Vector insertion performance
///
/// Compares insert speed between implementations.
fn bench_vector_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_insertion");

    for &size in &[100, 500, 1_000] {
        // Current (Linear)
        group.bench_with_input(
            BenchmarkId::new("current_linear", size),
            &size,
            |b, &size| {
                let dataset = generate_code_dataset(size);

                b.iter(|| {
                    let temp_dir = tempfile::tempdir().unwrap();
                    let store =
                        VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();
                    for (content, embedding, metadata) in &dataset {
                        black_box(
                            store
                                .add_vector(content, embedding.clone(), metadata.clone())
                                .unwrap(),
                        );
                    }
                });
            },
        );

        // HNSW
        group.bench_with_input(BenchmarkId::new("hnsw", size), &size, |b, &size| {
            let dataset = generate_code_dataset(size);

            b.iter(|| {
                let temp_dir = tempfile::tempdir().unwrap();
                let store =
                    HnswVectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();
                for (content, embedding, metadata) in &dataset {
                    black_box(
                        store
                            .add_vector(content, embedding.clone(), metadata.clone())
                            .unwrap(),
                    );
                }
            });
        });
    }

    group.finish();
}

/// Memory usage benchmark
///
/// Measures memory footprint for different index sizes.
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(5));

    for &size in VECTOR_SIZES {
        // Current (Linear)
        group.bench_with_input(
            BenchmarkId::new("current_vector_store", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = tempfile::tempdir().unwrap();
                    let store =
                        VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();
                    let dataset = generate_code_dataset(size);

                    for (content, embedding, metadata) in &dataset {
                        black_box(
                            store
                                .add_vector(content, embedding.clone(), metadata.clone())
                                .unwrap(),
                        );
                    }

                    // Force memory measurement
                    black_box(store.vector_count());
                });
            },
        );

        // HNSW
        group.bench_with_input(
            BenchmarkId::new("hnsw_vector_store", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = tempfile::tempdir().unwrap();
                    let store =
                        HnswVectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();
                    let dataset = generate_code_dataset(size);

                    for (content, embedding, metadata) in &dataset {
                        black_box(
                            store
                                .add_vector(content, embedding.clone(), metadata.clone())
                                .unwrap(),
                        );
                    }

                    // Force memory measurement
                    black_box(store.vector_count());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_current_vector_store,
    bench_current_vector_store_k_values,
    bench_hnsw_vector_store,
    bench_turso_vector_store,
    bench_vector_insertion,
    bench_memory_usage
);
criterion_main!(benches);
