//! Hot Cache - Memory suggestion integration for Cockpit UX
//!
//! This module provides non-intrusive memory suggestions that appear
//! during active sessions based on semantic relevance from the memory stack.
//!
//! ## Architecture
//!
//! ```text
//! Memory Stack (Core)
//!   └── Suggestion Stream (semantic search)
//!         ├── Hot Cache (buffer + TTL)
//!         └── Cockpit UI (hint rendering)
//! ```

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Default TTL for suggestions (60 seconds)
const DEFAULT_TTL_SECS: i64 = 60;

/// Maximum preview length for UI hint line
const MAX_PREVIEW_LENGTH: usize = 80;

/// Relevance threshold for showing suggestions
const RELEVANCE_THRESHOLD: f32 = 0.7;

/// Maximum number of suggestions to show at once
const MAX_SUGGESTIONS: usize = 3;

/// Memory suggestion from the semantic memory system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySuggestion {
    /// Memory ID
    pub memory_id: i64,
    /// Preview text (truncated content)
    pub preview: String,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f32,
    /// Flash intensity for UI animation (0.0 to 1.0)
    pub flash_intensity: f32,
}

impl MemorySuggestion {
    /// Create a new memory suggestion
    pub fn new(memory_id: i64, content: String, relevance_score: f32) -> Self {
        let preview = truncate_preview(&content, MAX_PREVIEW_LENGTH);
        let flash_intensity = calculate_flash_intensity(relevance_score);

        Self {
            memory_id,
            preview,
            relevance_score,
            flash_intensity,
        }
    }

    /// Check if this suggestion should be shown based on relevance
    pub fn should_show(&self) -> bool {
        self.relevance_score >= RELEVANCE_THRESHOLD
    }

    /// Get display priority (higher is more important)
    pub fn priority(&self) -> f32 {
        self.relevance_score * self.flash_intensity
    }
}

/// Time-to-live for a suggestion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionTtl {
    /// When this suggestion expires
    pub expires_at: DateTime<Utc>,
}

impl SuggestionTtl {
    /// Create a new TTL from seconds
    pub fn from_secs(secs: i64) -> Self {
        Self {
            expires_at: Utc::now() + Duration::seconds(secs),
        }
    }

    /// Create an already-expired TTL for testing
    pub fn expired() -> Self {
        Self {
            expires_at: Utc::now() - Duration::seconds(1),
        }
    }

    /// Check if this TTL has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Get remaining seconds
    pub fn remaining_secs(&self) -> i64 {
        let duration = self.expires_at - Utc::now();
        duration.num_seconds().max(0)
    }
}

/// Buffered suggestion with TTL tracking
#[derive(Debug, Clone)]
pub struct BufferedSuggestion {
    /// The underlying suggestion
    pub suggestion: MemorySuggestion,
    /// When this suggestion expires
    pub ttl: SuggestionTtl,
    /// When this suggestion was first seen
    pub created_at: DateTime<Utc>,
    /// Number of times this suggestion has been shown
    pub show_count: usize,
}

impl BufferedSuggestion {
    /// Create a new buffered suggestion
    pub fn new(suggestion: MemorySuggestion) -> Self {
        Self {
            ttl: SuggestionTtl::from_secs(DEFAULT_TTL_SECS),
            created_at: Utc::now(),
            show_count: 0,
            suggestion,
        }
    }

    /// Create with custom TTL
    pub fn with_ttl(suggestion: MemorySuggestion, ttl_secs: i64) -> Self {
        Self {
            ttl: SuggestionTtl::from_secs(ttl_secs),
            created_at: Utc::now(),
            show_count: 0,
            suggestion,
        }
    }

    /// Check if this suggestion has expired
    pub fn is_expired(&self) -> bool {
        self.ttl.is_expired()
    }

    /// Increment show count and return current count
    pub fn mark_shown(&mut self) -> usize {
        self.show_count += 1;
        self.show_count
    }

    /// Get age in seconds
    pub fn age_secs(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }
}

/// Hot cache for memory suggestions
#[derive(Debug, Clone, Default)]
pub struct HotCache {
    /// Buffered suggestions
    suggestions: Vec<BufferedSuggestion>,
    /// Maximum suggestions to keep
    max_suggestions: usize,
}

impl HotCache {
    /// Create a new hot cache
    pub fn new() -> Self {
        Self {
            suggestions: Vec::new(),
            max_suggestions: MAX_SUGGESTIONS,
        }
    }

    /// Create with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            suggestions: Vec::new(),
            max_suggestions: capacity,
        }
    }

    /// Add a new suggestion to the cache
    pub fn insert(&mut self, suggestion: MemorySuggestion) {
        // Check if we already have this memory
        if let Some(existing) = self
            .suggestions
            .iter_mut()
            .find(|s| s.suggestion.memory_id == suggestion.memory_id)
        {
            // Update existing suggestion with new relevance
            existing.suggestion = suggestion;
            existing.ttl = SuggestionTtl::from_secs(DEFAULT_TTL_SECS);
            return;
        }

        // Add new suggestion
        let buffered = BufferedSuggestion::new(suggestion);
        self.suggestions.push(buffered);

        // Trim to capacity
        self.trim();
    }

    /// Remove expired suggestions
    pub fn prune_expired(&mut self) {
        self.suggestions.retain(|s| !s.is_expired());
    }

    /// Get active (non-expired) suggestions, sorted by priority
    pub fn active_suggestions(&mut self) -> Vec<&mut MemorySuggestion> {
        self.prune_expired();

        // Sort by priority
        self.suggestions.sort_by(|a, b| {
            b.suggestion
                .priority()
                .partial_cmp(&a.suggestion.priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Mark as shown and collect references
        self.suggestions
            .iter_mut()
            .filter(|s| s.suggestion.should_show())
            .map(|s| {
                s.mark_shown();
                &mut s.suggestion
            })
            .collect()
    }

    /// Get the number of active suggestions
    pub fn len(&self) -> usize {
        self.suggestions.iter().filter(|s| !s.is_expired()).count()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.suggestions.is_empty() || self.suggestions.iter().all(|s| s.is_expired())
    }

    /// Clear all suggestions
    pub fn clear(&mut self) {
        self.suggestions.clear();
    }

    /// Trim cache to max capacity
    fn trim(&mut self) {
        if self.suggestions.len() > self.max_suggestions {
            // Sort by relevance and keep top N
            self.suggestions.sort_by(|a, b| {
                b.suggestion
                    .relevance_score
                    .partial_cmp(&a.suggestion.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.suggestions.truncate(self.max_suggestions);
        }
    }
}

/// Clamp flash intensity to [0.0, 1.0]
pub fn clamp_flash(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

/// Calculate flash intensity based on relevance score
fn calculate_flash_intensity(relevance: f32) -> f32 {
    clamp_flash(relevance)
}

/// Truncate preview text to fit within max length
pub fn truncate_preview(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    // Truncate and add ellipsis
    let safe_len = max_len.saturating_sub(3);
    format!("{}...", &text[..safe_len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_suggestion_new() {
        let suggestion = MemorySuggestion::new(123, "User prefers Vim editor".to_string(), 0.85);

        assert_eq!(suggestion.memory_id, 123);
        assert!(!suggestion.preview.is_empty());
        assert_eq!(suggestion.relevance_score, 0.85);
        assert!(suggestion.should_show());
    }

    #[test]
    fn test_memory_suggestion_should_show_threshold() {
        let high_relevance = MemorySuggestion::new(1, "test".to_string(), 0.8);
        assert!(high_relevance.should_show());

        let low_relevance = MemorySuggestion::new(2, "test".to_string(), 0.5);
        assert!(!low_relevance.should_show());

        let border = MemorySuggestion::new(3, "test".to_string(), 0.7);
        assert!(border.should_show());
    }

    #[test]
    fn test_ttl_expiration() {
        let ttl = SuggestionTtl::from_secs(1);
        assert!(!ttl.is_expired());

        let expired = SuggestionTtl::expired();
        assert!(expired.is_expired());
    }

    #[test]
    fn test_ttl_remaining_secs() {
        let ttl = SuggestionTtl::from_secs(60);
        assert!(ttl.remaining_secs() > 0);
        assert!(ttl.remaining_secs() <= 60);
    }

    #[test]
    fn test_buffered_suggestion_lifecycle() {
        let suggestion = MemorySuggestion::new(1, "test".to_string(), 0.8);
        let mut buffered = BufferedSuggestion::new(suggestion);

        assert!(!buffered.is_expired());
        assert_eq!(buffered.show_count, 0);

        buffered.mark_shown();
        assert_eq!(buffered.show_count, 1);

        assert!(buffered.age_secs() >= 0);
    }

    #[test]
    fn test_hot_cache_insert() {
        let mut cache = HotCache::new();

        let suggestion = MemorySuggestion::new(1, "test".to_string(), 0.8);
        cache.insert(suggestion);

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_hot_cache_prune_expired() {
        let mut cache = HotCache::new();

        let suggestion = MemorySuggestion::new(1, "test".to_string(), 0.8);
        let mut buffered = BufferedSuggestion::new(suggestion);
        buffered.ttl = SuggestionTtl::expired();
        cache.suggestions.push(buffered);

        cache.prune_expired();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_hot_cache_capacity() {
        let mut cache = HotCache::with_capacity(2);

        cache.insert(MemorySuggestion::new(1, "low".to_string(), 0.5));
        cache.insert(MemorySuggestion::new(2, "medium".to_string(), 0.7));
        cache.insert(MemorySuggestion::new(3, "high".to_string(), 0.9));

        // Should keep only top 2 by relevance
        assert!(cache.len() <= 2);
    }

    #[test]
    fn test_hot_cache_active_suggestions() {
        let mut cache = HotCache::new();

        cache.insert(MemorySuggestion::new(1, "low".to_string(), 0.5));
        cache.insert(MemorySuggestion::new(2, "high".to_string(), 0.9));

        let active = cache.active_suggestions();
        // Only high relevance should be shown
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_clamp_flash() {
        assert_eq!(clamp_flash(-0.5), 0.0);
        assert_eq!(clamp_flash(0.0), 0.0);
        assert_eq!(clamp_flash(0.5), 0.5);
        assert_eq!(clamp_flash(1.0), 1.0);
        assert_eq!(clamp_flash(1.5), 1.0);
    }

    #[test]
    fn test_truncate_preview() {
        let short = "short";
        assert_eq!(truncate_preview(short, 10), "short");

        let long = "This is a very long string that should be truncated";
        let truncated = truncate_preview(long, 20);
        assert!(truncated.len() <= 23); // 20 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_calculate_flash_intensity() {
        assert_eq!(calculate_flash_intensity(0.0), 0.0);
        assert_eq!(calculate_flash_intensity(0.5), 0.5);
        assert_eq!(calculate_flash_intensity(1.0), 1.0);
        assert_eq!(calculate_flash_intensity(1.5), 1.0);
    }

    #[test]
    fn test_memory_suggestion_priority() {
        let suggestion = MemorySuggestion::new(1, "test".to_string(), 0.8);
        let priority = suggestion.priority();
        // Use approximate comparison for floating point (0.8 * 0.8 = 0.64)
        assert!(
            (priority - 0.64).abs() < 0.001,
            "Priority {} should be close to 0.64",
            priority
        );
    }

    #[test]
    fn test_hot_cache_update_existing() {
        let mut cache = HotCache::new();

        cache.insert(MemorySuggestion::new(1, "original".to_string(), 0.7));
        cache.insert(MemorySuggestion::new(1, "updated".to_string(), 0.9));

        // Should update existing, not add duplicate
        assert_eq!(cache.len(), 1);

        let active = cache.active_suggestions();
        assert_eq!(active[0].preview, "updated");
        assert_eq!(active[0].relevance_score, 0.9);
    }
}
