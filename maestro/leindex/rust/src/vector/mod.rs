//! Vector Store Module
//!
//! Pure Rust implementation of LEANN vector search.
//! Supports multiple vector store backends:
//! - Linear cosine similarity (baseline)
//! - HNSW-based approximate nearest neighbor search
//! - Turso native vector search with DiskANN
//! - SIMD-accelerated cosine similarity

pub mod adaptive;
pub mod cache;
pub mod hnsw_store;
pub mod metadata;
pub mod migrations;
pub mod report;
pub mod simd;
pub mod store;
pub mod turso_store;

#[cfg(test)]
mod benchmark_tests;
#[cfg(test)]
mod concurrency_tests;

pub use adaptive::*;
pub use cache::*;
pub use hnsw_store::*;
pub use metadata::*;
pub use migrations::*;
pub use report::*;
pub use simd::cosine_similarity;
pub use store::*;
pub use turso_store::*;
