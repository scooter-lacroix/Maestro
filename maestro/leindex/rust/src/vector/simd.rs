//! SIMD-accelerated vector operations
//!
//! Provides optimized cosine similarity computation using SIMD instructions
//! via the `wide` crate for portable SIMD operations.

use wide::f32x8;

/// SIMD-accelerated cosine similarity
///
/// Processes 8 floating point numbers at a time using SIMD instructions,
/// providing ~2-3x speedup over scalar implementation.
#[inline(always)]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let len = a.len();
    let chunks = len / 8;
    let _remainder = len % 8;

    // Use SIMD for chunks of 8
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..chunks {
        let offset = i * 8;
        let a_arr: [f32; 8] = a[offset..offset + 8].try_into().unwrap();
        let b_arr: [f32; 8] = b[offset..offset + 8].try_into().unwrap();
        let a_vec = f32x8::new(a_arr);
        let b_vec = f32x8::new(b_arr);

        let mult = a_vec * b_vec;
        dot += mult.reduce_add();
        norm_a += (a_vec * a_vec).reduce_add();
        norm_b += (b_vec * b_vec).reduce_add();
    }

    // Handle remainder
    for i in (chunks * 8)..len {
        let ai = a[i];
        let bi = b[i];
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let norm_a = norm_a.sqrt();
    let norm_b = norm_b.sqrt();

    const EPSILON: f32 = 1e-10;
    if norm_a < EPSILON || norm_b < EPSILON {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_accuracy() {
        // Test identical vectors
        let a = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let result = cosine_similarity(&a, &b);
        assert!((result - 1.0).abs() < 0.001);

        // Test orthogonal vectors
        let c = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let result2 = cosine_similarity(&a, &c);
        assert!(result2.abs() < 0.001);

        // Test 768-dimensional vectors (real use case)
        let a_768: Vec<f32> = (0..768).map(|i| (i as f32 / 768.0).cos()).collect();
        let b_768: Vec<f32> = (0..768).map(|i| (i as f32 / 768.0).sin()).collect();
        let result3 = cosine_similarity(&a_768, &b_768);
        assert!(result3 >= -1.0 && result3 <= 1.0);
    }

    #[test]
    fn test_simd_performance() {
        use std::time::Instant;

        let iterations = 10000;
        let a: Vec<f32> = (0..768).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..768).map(|i| (i as f32).cos()).collect();

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = cosine_similarity(&a, &b);
        }
        let elapsed = start.elapsed();

        println!(
            "SIMD cosine_similarity: {} iterations in {:?}",
            iterations, elapsed
        );
        println!(
            "Average: {:.2} µs per call",
            elapsed.as_micros() as f64 / iterations as f64
        );
    }
}
