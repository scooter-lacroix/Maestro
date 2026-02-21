//! Shared types for Nexus Memory integration
//!
//! This module provides common types used across the memory subsystem:
//! - Memory suggestions for hot cache
//! - Vector search results with graph boosting
//! - Embedding metadata and configuration
//! - Compression result types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default embedding dimension (all-MiniLM-L6-v2)
pub const EMBEDDING_DIMENSION: usize = 384;

/// Default model name for embeddings
pub const DEFAULT_EMBEDDING_MODEL: &str = "all-MiniLM-L6-v2";

/// Memory suggestion for hot cache display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySuggestion {
    /// Unique memory identifier
    pub memory_id: i64,

    /// Namespace ID for filtering
    pub namespace_id: i64,

    /// Preview of memory content (truncated)
    pub content_preview: String,

    /// Full content length in characters
    pub content_length: usize,

    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f32,

    /// Base similarity from vector search
    pub similarity_score: f32,

    /// Graph-boosted score
    pub boosted_score: f32,

    /// Memory category
    pub category: MemoryCategory,

    /// Optional memory lane type
    pub lane_type: Option<MemoryLaneType>,

    /// Priority level (1=high, 2=medium, 3=low)
    pub priority: u8,

    /// Flash intensity for UI animation (0.0 to 1.0)
    pub flash_intensity: f32,

    /// When this suggestion was generated
    pub generated_at: DateTime<Utc>,

    /// Whether this is a new suggestion (not yet viewed)
    pub is_new: bool,
}

impl MemorySuggestion {
    /// Create a new suggestion with calculated flash intensity
    pub fn new(
        memory_id: i64,
        namespace_id: i64,
        content: &str,
        relevance_score: f32,
        similarity_score: f32,
        boosted_score: f32,
        category: MemoryCategory,
    ) -> Self {
        // Calculate flash intensity based on relevance
        let flash_intensity = (relevance_score * 0.7 + boosted_score * 0.3).min(1.0);

        // Create preview (max 100 chars)
        let content_preview = if content.len() > 100 {
            format!("{}...", &content[..97])
        } else {
            content.to_string()
        };

        Self {
            memory_id,
            namespace_id,
            content_preview,
            content_length: content.len(),
            relevance_score,
            similarity_score,
            boosted_score,
            category,
            lane_type: None,
            priority: 3,
            flash_intensity,
            generated_at: Utc::now(),
            is_new: true,
        }
    }

    /// Set the memory lane type
    pub fn with_lane_type(mut self, lane_type: MemoryLaneType) -> Self {
        self.lane_type = Some(lane_type);
        self
    }

    /// Set the priority level
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 3);
        // Recalculate flash intensity with priority boost
        let priority_boost = match self.priority {
            1 => 0.2, // High priority
            2 => 0.1, // Medium priority
            _ => 0.0, // Low priority
        };
        self.flash_intensity = (self.flash_intensity + priority_boost).min(1.0);
        self
    }

    /// Mark suggestion as viewed
    pub fn mark_viewed(&mut self) {
        self.is_new = false;
    }
}

/// Memory category types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum MemoryCategory {
    /// General information
    #[default]
    General,
    /// Factual knowledge
    Facts,
    /// User preferences
    Preferences,
    /// Technical specifications
    Specifications,
    /// Pattern recognition
    Patterns,
    /// Design decisions
    Decisions,
    /// Contextual information
    Context,
    /// Temporary working memory
    Temporary,
    /// Code observations
    Observations,
}

impl MemoryCategory {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Facts => "facts",
            Self::Preferences => "preferences",
            Self::Specifications => "specifications",
            Self::Patterns => "patterns",
            Self::Decisions => "decisions",
            Self::Context => "context",
            Self::Temporary => "temporary",
            Self::Observations => "observations",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "general" => Some(Self::General),
            "facts" | "fact" => Some(Self::Facts),
            "preferences" | "preference" => Some(Self::Preferences),
            "specifications" | "specification" | "specs" | "spec" => Some(Self::Specifications),
            "patterns" | "pattern" => Some(Self::Patterns),
            "decisions" | "decision" => Some(Self::Decisions),
            "context" => Some(Self::Context),
            "temporary" => Some(Self::Temporary),
            "observations" | "observation" => Some(Self::Observations),
            _ => None,
        }
    }

    /// Get default priority for this category
    pub fn default_priority(&self) -> u8 {
        match self {
            Self::Decisions => 1,       // High priority
            Self::Specifications => 1,  // High priority
            Self::Patterns => 2,        // Medium priority
            Self::Facts => 2,           // Medium priority
            Self::Preferences => 2,     // Medium priority
            Self::Context => 3,         // Low priority
            Self::Observations => 3,    // Low priority
            Self::General => 3,         // Low priority
            Self::Temporary => 3,       // Low priority
        }
    }
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


/// Memory lane type for specialized retrieval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLaneType {
    /// Correction to previous memory
    Correction,
    /// Pattern seed for learning
    PatternSeed,
    /// Key insight
    Insight,
    /// Context anchor
    Anchor,
    /// Reference material
    Reference,
}

impl MemoryLaneType {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Correction => "correction",
            Self::PatternSeed => "pattern_seed",
            Self::Insight => "insight",
            Self::Anchor => "anchor",
            Self::Reference => "reference",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "correction" | "correct" => Some(Self::Correction),
            "pattern_seed" | "patternseed" | "pattern" => Some(Self::PatternSeed),
            "insight" => Some(Self::Insight),
            "anchor" => Some(Self::Anchor),
            "reference" | "ref" => Some(Self::Reference),
            _ => None,
        }
    }

    /// Get boost factor for this lane type
    pub fn boost_factor(&self) -> f32 {
        match self {
            Self::Correction => 1.3,  // Corrections are important
            Self::PatternSeed => 1.2, // Patterns are valuable
            Self::Insight => 1.25,    // Insights are valuable
            Self::Anchor => 1.1,      // Anchors are moderately important
            Self::Reference => 1.0,   // References are baseline
        }
    }
}

impl std::fmt::Display for MemoryLaneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Vector search result with scoring breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// Memory ID
    pub id: i64,

    /// Namespace ID
    pub namespace_id: i64,

    /// Base cosine similarity (0.0 to 1.0)
    pub similarity: f32,

    /// Graph-boosted score
    pub boosted_score: f32,

    /// Priority weight applied
    pub priority_weight: f32,

    /// Depth in graph tree
    pub depth: u32,

    /// Category
    pub category: MemoryCategory,

    /// Lane type (if any)
    pub lane_type: Option<MemoryLaneType>,
}

impl VectorSearchResult {
    /// Create a new search result
    pub fn new(id: i64, namespace_id: i64, similarity: f32) -> Self {
        Self {
            id,
            namespace_id,
            similarity,
            boosted_score: similarity,
            priority_weight: 1.0,
            depth: 0,
            category: MemoryCategory::General,
            lane_type: None,
        }
    }

    /// Apply graph tree boosting
    pub fn apply_boost(&mut self, weight: f32, depth: u32, lane_boost: Option<f32>) {
        self.priority_weight = weight;
        self.depth = depth;

        // Depth penalty (slight reduction for deeper nodes)
        let depth_factor = 1.0 - (depth as f32 * 0.02);

        // Apply all factors
        self.boosted_score = self.similarity * weight * depth_factor.max(0.8);

        // Apply lane type boost if present
        if let Some(boost) = lane_boost {
            self.boosted_score *= boost;
        }
    }
}

/// Embedding metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    /// Memory ID this embedding belongs to
    pub memory_id: i64,

    /// Namespace ID
    pub namespace_id: i64,

    /// Embedding model used
    pub model: String,

    /// Embedding dimension
    pub dimension: usize,

    /// When the embedding was created
    pub created_at: DateTime<Utc>,

    /// Whether embedding is up-to-date with content
    pub is_current: bool,

    /// Content hash for staleness detection
    pub content_hash: u64,
}

impl EmbeddingMetadata {
    /// Create new embedding metadata
    pub fn new(memory_id: i64, namespace_id: i64, model: &str, dimension: usize) -> Self {
        Self {
            memory_id,
            namespace_id,
            model: model.to_string(),
            dimension,
            created_at: Utc::now(),
            is_current: true,
            content_hash: 0,
        }
    }

    /// Set content hash
    pub fn with_content_hash(mut self, hash: u64) -> Self {
        self.content_hash = hash;
        self
    }
}

/// Result of memory compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Original content
    pub original_content: String,

    /// Compressed/summarized content
    pub compressed_content: String,

    /// Original token count (estimated)
    pub original_tokens: usize,

    /// Compressed token count (estimated)
    pub compressed_tokens: usize,

    /// Compression ratio (0.0 to 1.0, lower is better)
    pub compression_ratio: f32,

    /// Key concepts extracted
    pub key_concepts: Vec<String>,

    /// Whether compression was lossy
    pub is_lossy: bool,
}

impl CompressionResult {
    /// Create a new compression result
    pub fn new(original: &str, compressed: &str, concepts: Vec<String>) -> Self {
        // Simple token estimation (4 chars per token average)
        let original_tokens = original.len() / 4;
        let compressed_tokens = compressed.len() / 4;

        let compression_ratio = if original_tokens > 0 {
            compressed_tokens as f32 / original_tokens as f32
        } else {
            1.0
        };

        Self {
            original_content: original.to_string(),
            compressed_content: compressed.to_string(),
            original_tokens,
            compressed_tokens,
            compression_ratio,
            key_concepts: concepts,
            is_lossy: compressed_tokens < original_tokens,
        }
    }

    /// Check if compression meets quality threshold
    pub fn meets_quality_threshold(&self, max_ratio: f32) -> bool {
        self.compression_ratio <= max_ratio
    }
}

/// Hot cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotCacheConfig {
    /// Maximum number of entries in cache
    pub capacity: usize,

    /// Time-to-live in seconds
    pub ttl_secs: u64,

    /// Minimum relevance score to include in cache
    pub min_relevance: f32,

    /// Maximum suggestions to return per query
    pub max_suggestions: usize,

    /// Background embedding refresh interval in seconds
    pub refresh_interval_secs: u64,

    /// Whether to enable flash animations
    pub enable_flash: bool,
}

impl Default for HotCacheConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            ttl_secs: 300, // 5 minutes
            min_relevance: 0.3,
            max_suggestions: 5,
            refresh_interval_secs: 60,
            enable_flash: true,
        }
    }
}

/// Vector store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Embedding dimension
    pub dimension: usize,

    /// HNSW M parameter (connections per node)
    pub hnsw_m: usize,

    /// HNSW ef_construction parameter
    pub hnsw_ef_construction: usize,

    /// HNSW ef_search parameter
    pub hnsw_ef_search: usize,

    /// Maximum vectors to store in memory
    pub max_in_memory: usize,

    /// Whether to persist to disk
    pub persist: bool,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            dimension: EMBEDDING_DIMENSION,
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            hnsw_ef_search: 50,
            max_in_memory: 100_000,
            persist: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_suggestion_creation() {
        let suggestion = MemorySuggestion::new(
            1,
            1,
            "This is a test memory content that should be truncated if too long",
            0.8,
            0.75,
            0.82,
            MemoryCategory::Facts,
        );

        assert_eq!(suggestion.memory_id, 1);
        assert!(!suggestion.content_preview.is_empty());
        assert!(suggestion.relevance_score > 0.0);
        assert!(suggestion.is_new);
    }

    #[test]
    fn test_memory_suggestion_priority_boost() {
        let base = MemorySuggestion::new(
            1,
            1,
            "test",
            0.5,
            0.5,
            0.5,
            MemoryCategory::General,
        );

        let high_priority = MemorySuggestion::new(
            1,
            1,
            "test",
            0.5,
            0.5,
            0.5,
            MemoryCategory::General,
        )
        .with_priority(1);

        assert!(high_priority.flash_intensity > base.flash_intensity);
    }

    #[test]
    fn test_memory_category_parsing() {
        assert_eq!(MemoryCategory::from_str("facts"), Some(MemoryCategory::Facts));
        assert_eq!(MemoryCategory::from_str("FACT"), Some(MemoryCategory::Facts));
        assert_eq!(MemoryCategory::from_str("unknown"), None);
    }

    #[test]
    fn test_memory_lane_type_boost() {
        assert!(MemoryLaneType::Correction.boost_factor() > MemoryLaneType::Reference.boost_factor());
    }

    #[test]
    fn test_vector_search_result_boosting() {
        let mut result = VectorSearchResult::new(1, 1, 0.8);
        result.apply_boost(1.5, 2, Some(1.2));

        assert!(result.boosted_score > result.similarity);
        assert_eq!(result.priority_weight, 1.5);
        assert_eq!(result.depth, 2);
    }

    #[test]
    fn test_compression_result() {
        let result = CompressionResult::new(
            "This is the original content that is quite long",
            "Short summary",
            vec!["key".to_string()],
        );

        assert!(result.compression_ratio < 1.0);
        assert!(result.is_lossy);
        assert!(!result.key_concepts.is_empty());
    }

    #[test]
    fn test_default_configs() {
        let hot_config = HotCacheConfig::default();
        assert_eq!(hot_config.capacity, 1000);
        assert_eq!(hot_config.ttl_secs, 300);

        let vector_config = VectorStoreConfig::default();
        assert_eq!(vector_config.dimension, EMBEDDING_DIMENSION);
    }
}
