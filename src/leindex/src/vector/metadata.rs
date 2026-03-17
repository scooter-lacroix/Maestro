//! Vector Metadata
//!
//! Metadata structures for vectors and index.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default embedding dimension (CodeRankEmbed)
pub const DEFAULT_EMBEDDING_DIM: usize = 768;

/// Supported embedding models
pub const SUPPORTED_MODELS: &[(&str, usize)] = &[
    ("nomic-ai/CodeRankEmbed", 768),
    ("all-MiniLM-L6-v2", 384),
    ("BAAI/bge-small-en-v1.5", 384),
];

/// Supported backends
pub const SUPPORTED_BACKENDS: &[&str] = &["hnsw", "diskann"];

/// Security limits
pub const MAX_VECTORS: usize = 10_000_000;
pub const MAX_QUERY_LENGTH: usize = 8192;
pub const MAX_TOP_K: usize = 1000;
pub const MAX_CONTENT_SIZE: usize = 50 * 1024 * 1024; // 50MB

/// Metadata for a vector in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub file_path: String,
    pub chunk_index: i32,
    pub start_line: Option<i32>,
    pub end_line: Option<i32>,
    pub chunk_type: ChunkType,
    pub parent_context: Option<String>,
    pub embedding_model: String,
    pub created_at: DateTime<Utc>,
}

impl VectorMetadata {
    pub fn new(file_path: &str, chunk_index: i32) -> Self {
        Self {
            file_path: file_path.to_string(),
            chunk_index,
            start_line: None,
            end_line: None,
            chunk_type: ChunkType::Text,
            parent_context: None,
            embedding_model: "nomic-ai/CodeRankEmbed".to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn with_lines(mut self, start: i32, end: i32) -> Self {
        self.start_line = Some(start);
        self.end_line = Some(end);
        self
    }

    pub fn with_type(mut self, chunk_type: ChunkType) -> Self {
        self.chunk_type = chunk_type;
        self
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.parent_context = Some(context.to_string());
        self
    }
}

/// Types of code chunks
/// NOTE: Explicit discriminants ensure stable integer values for database storage
/// Never change the numeric values - they're persisted in Turso database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ChunkType {
    Function = 0,
    Class = 1,
    Module = 2,
    Import = 3,
    Comment = 4,
    #[default]
    Text = 5,
    Other = 6, // Must remain 6 for backward compatibility
}

impl ChunkType {
    /// Convert to integer for database storage (prevents SQL injection)
    pub fn to_i32(self) -> i32 {
        self as i32
    }

    /// Convert from integer (database storage) to ChunkType
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Function,
            1 => Self::Class,
            2 => Self::Module,
            3 => Self::Import,
            4 => Self::Comment,
            5 => Self::Text,
            _ => Self::Other, // 6 or unknown -> Other
        }
    }
}

/// Index-level metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub version: String,
    pub backend: String,
    pub model: String,
    pub dimension: usize,
    pub vector_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            backend: "hnsw".to_string(),
            model: "nomic-ai/CodeRankEmbed".to_string(),
            dimension: DEFAULT_EMBEDDING_DIM,
            vector_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Search result from vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub vector_id: String,
    pub score: f32,
    pub metadata: VectorMetadata,
    pub content: Option<String>,
}

/// Configuration for HNSW backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    pub graph_degree: usize,
    pub build_complexity: usize,
    pub search_complexity: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            graph_degree: 32,
            build_complexity: 64,
            search_complexity: 32,
        }
    }
}

/// Retry configuration with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub enabled: bool,
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a retry attempt
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay =
            (self.initial_delay_ms as f64 * self.exponential_base.powi(attempt as i32)) as u64;
        let delay = delay.min(self.max_delay_ms);

        if self.jitter {
            let jitter = (delay as f64 * 0.25 * rand::random()) as u64;
            delay.saturating_add(jitter)
        } else {
            delay
        }
    }
}

// Use a simple random for jitter (avoiding extra dependency)
mod rand {
    pub fn random() -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // Hash the nanos to get a pseudo-random distribution
        let mut hasher = DefaultHasher::new();
        hasher.write_u128(nanos);
        let h = hasher.finish();

        (h as f64) / (u64::MAX as f64)
    }
}

// Validation constants (Task 7.6.31-33, 7.7.1)
pub const MIN_EMBEDDING_DIM: usize = 1;
pub const MAX_EMBEDDING_DIM: usize = 4096;
pub const MAX_CHUNK_INDEX: i32 = 10_000_000;
pub const MAX_FILE_PATH_LENGTH: usize = 4096;

/// Validate embedding dimensions (Task 7.6.31)
pub fn validate_embedding_dim(embedding: &[f32]) -> Result<()> {
    let dim = embedding.len();
    if !(MIN_EMBEDDING_DIM..=MAX_EMBEDDING_DIM).contains(&dim) {
        return Err(anyhow::anyhow!(
            "Invalid embedding dimension: {} (must be between {} and {})",
            dim,
            MIN_EMBEDDING_DIM,
            MAX_EMBEDDING_DIM
        ));
    }
    Ok(())
}

/// Validate vector_id format (Task 7.6.32)
pub fn validate_vector_id(vector_id: &str) -> Result<()> {
    if !vector_id.starts_with("vec_") {
        return Err(anyhow::anyhow!(
            "Invalid vector_id format: must start with 'vec_', got: {}",
            vector_id
        ));
    }
    if vector_id.len() < 5 {
        // "vec_" + at least 1 char
        return Err(anyhow::anyhow!(
            "Invalid vector_id: too short, got: {}",
            vector_id
        ));
    }
    Ok(())
}

/// Validate chunk_index bounds (Task 7.6.33)
pub fn validate_chunk_index(chunk_index: i32) -> Result<()> {
    if !(0..=MAX_CHUNK_INDEX).contains(&chunk_index) {
        return Err(anyhow::anyhow!(
            "Invalid chunk_index: {} (must be between 0 and {})",
            chunk_index,
            MAX_CHUNK_INDEX
        ));
    }
    Ok(())
}

/// Validate file_path is not empty (Task 7.7.1)
pub fn validate_file_path(file_path: &str) -> Result<()> {
    if file_path.trim().is_empty() {
        return Err(anyhow::anyhow!("Invalid file_path: cannot be empty"));
    }
    if file_path.len() > MAX_FILE_PATH_LENGTH {
        return Err(anyhow::anyhow!(
            "Invalid file_path: too long ({} chars, max {})",
            file_path.len(),
            MAX_FILE_PATH_LENGTH
        ));
    }
    Ok(())
}
