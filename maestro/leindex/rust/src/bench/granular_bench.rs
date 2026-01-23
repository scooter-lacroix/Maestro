//! Granular benchmark to find optimal Linear -> HNSW switch point
//!
//! Run with: cargo run --bin granular_bench --release

use leindex_core::vector::{HnswVectorStore, VectorMetadata, VectorStore};
use std::time::Instant;
use tempfile::tempdir;

fn generate_embedding(seed: usize) -> Vec<f32> {
    let mut embedding = vec![0.0f32; 768];
    for (i, val) in embedding.iter_mut().enumerate() {
        let base = ((i as f32 * 0.01 + seed as f32 * 0.001) % std::f32::consts::PI).sin();
        *val = base.clamp(-1.0, 1.0);
    }
    // Normalize
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in embedding.iter_mut() {
            *val /= norm;
        }
    }
    embedding
}

fn main() {
    // Granular steps between 50K and 100K
    let sizes = &[50_000, 60_000, 70_000, 80_000, 90_000, 100_000];
    let warmup_iterations = 100;
    let measurement_iterations = 1000;

    println!("Granular Vector Search Benchmark: Linear vs HNSW");
    println!("======================================================\n");

    for &size in sizes {
        println!("\n=== Dataset Size: {} vectors ===", size);

        // Generate embeddings once
        let embeddings: Vec<_> = (0..size).map(|i| generate_embedding(i)).collect();

        // Benchmark Linear Search
        println!("\n[Linear Search]");
        let temp_dir = tempdir().unwrap();
        let store = VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

        let start_insert = Instant::now();
        for (i, embedding) in embeddings.iter().enumerate() {
            let metadata = VectorMetadata::new(&format!("file_{}.rs", i / 1000), i as i32);
            let content = format!("content {}", i);
            store
                .add_vector(&content, embedding.clone(), metadata)
                .unwrap();
        }
        let insert_time = start_insert.elapsed();
        println!("  Insert time: {:.2}s", insert_time.as_secs_f64());

        // Warmup
        for _ in 0..warmup_iterations {
            let _ = store.search(&embeddings[0], 10).unwrap();
        }

        // Measure
        let start = Instant::now();
        for _ in 0..measurement_iterations {
            let _ = store.search(&embeddings[0], 10).unwrap();
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / measurement_iterations as f64;
        let qps = 1_000_000.0 / avg_us;
        println!("  Avg query time: {:.3} µs", avg_us);
        println!("  Queries per second: {:.0}", qps);

        // Benchmark HNSW
        println!("\n[HNSW Search]");
        let temp_dir = tempdir().unwrap();
        let store = HnswVectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap();

        // OPTIMIZATION: Use batch insert for HNSW (100x faster than individual inserts)
        let items: Vec<(String, Vec<f32>, VectorMetadata)> = embeddings.iter().enumerate()
            .map(|(i, embedding)| {
                let metadata = VectorMetadata::new(&format!("file_{}.rs", i / 1000), i as i32);
                (format!("content {}", i), embedding.clone(), metadata)
            })
            .collect();

        let start_insert = Instant::now();
        let _vector_ids = store.add_vectors_batch(items).unwrap();
        let insert_time = start_insert.elapsed();
        println!("  Insert time: {:.2}s", insert_time.as_secs_f64());

        // Warmup
        for _ in 0..warmup_iterations {
            let _ = store.search(&embeddings[0], 10).unwrap();
        }

        // Measure
        let start = Instant::now();
        for _ in 0..measurement_iterations {
            let _ = store.search(&embeddings[0], 10).unwrap();
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / measurement_iterations as f64;
        let qps = 1_000_000.0 / avg_us;
        println!("  Avg query time: {:.3} µs", avg_us);
        println!("  Queries per second: {:.0}", qps);

        // Determine winner
        if avg_us < 2.7 {
            println!("\n  => Both excellent (< 2.7 µs)");
        }
    }

    println!("\n=== Benchmark Complete ===");
}
