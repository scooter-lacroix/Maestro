//! Vector Store Module
//!
//! Pure Rust implementation of LEANN vector search.
//! Supports multiple vector store backends:
//! - Linear cosine similarity (baseline)
//! - HNSW-based approximate nearest neighbor search
//! - Turso native vector search with DiskANN
//! - SIMD-accelerated cosine similarity

pub mod store;
pub mod metadata;
pub mod cache;
pub mod report;
pub mod hnsw_store;
pub mod turso_store;
pub mod simd;
pub mod adaptive;

#[cfg(test)]
mod benchmark_tests;
#[cfg(test)]
mod concurrency_tests;

pub use store::*;
pub use metadata::*;
pub use cache::*;
pub use report::*;
pub use hnsw_store::*;
pub use turso_store::*;
pub use simd::cosine_similarity;
pub use adaptive::*;
