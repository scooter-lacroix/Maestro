//! Hybrid Ranking - Combines text (BM25) and vector similarity scores
//!
//! This module provides score fusion for hybrid search:
//! - Normalize text scores (BM25 can vary widely)
//! - Combine with vector scores using configurable weights
//! - Return ranked, deduplicated results

use std::collections::HashMap;

/// A result from hybrid ranking with scores from both sources
#[derive(Debug, Clone, PartialEq)]
pub struct RankedResult {
    /// Document identifier
    pub id: String,
    /// Text/BM25 score (normalized to 0-1)
    pub text_score: Option<f32>,
    /// Vector similarity score (typically 0-1 from cosine)
    pub vector_score: Option<f32>,
    /// Final combined score
    pub final_score: f32,
}

/// Hybrid ranker that combines text and vector search results
#[derive(Debug, Clone)]
pub struct HybridRanker {
    /// Weight for vector similarity scores (0.0 to 1.0)
    vector_weight: f32,
    /// Weight for text/BM25 scores (0.0 to 1.0)
    text_weight: f32,
}

impl HybridRanker {
    /// Create a new hybrid ranker with specified weights
    ///
    /// # Arguments
    /// * `vector_weight` - Weight for vector similarity scores
    /// * `text_weight` - Weight for text/BM25 scores
    ///
    /// # Note
    /// Weights are used as-is; they don't need to sum to 1.0
    pub fn new(vector_weight: f32, text_weight: f32) -> Self {
        Self {
            vector_weight,
            text_weight,
        }
    }

    /// Create a ranker with equal weights (0.5, 0.5)
    pub fn balanced() -> Self {
        Self::new(0.5, 0.5)
    }

    /// Create a ranker favoring vector similarity
    pub fn vector_focused() -> Self {
        Self::new(0.7, 0.3)
    }

    /// Create a ranker favoring text search
    pub fn text_focused() -> Self {
        Self::new(0.3, 0.7)
    }

    /// Merge text and vector search results into ranked results
    ///
    /// # Arguments
    /// * `text_results` - Results from text search (id, score pairs)
    /// * `vector_results` - Results from vector search (id, score pairs)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Ranked and deduplicated results sorted by final score descending
    pub fn merge(
        &self,
        text_results: &[(String, f32)],
        vector_results: &[(String, f32)],
        limit: usize,
    ) -> Vec<RankedResult> {
        if limit == 0 {
            return Vec::new();
        }

        let mut merged: HashMap<String, RankedResult> = HashMap::new();

        // Process text results (normalize scores)
        let max_text = text_results.iter().map(|(_, s)| *s).fold(0.0_f32, f32::max);
        let text_norm = if max_text > 0.0 { max_text } else { 1.0 };

        for (id, score) in text_results {
            let normalized = score / text_norm;
            merged
                .entry(id.clone())
                .and_modify(|r| r.text_score = Some(normalized))
                .or_insert(RankedResult {
                    id: id.clone(),
                    text_score: Some(normalized),
                    vector_score: None,
                    final_score: 0.0,
                });
        }

        // Process vector results (already typically 0-1)
        let max_vec = vector_results
            .iter()
            .map(|(_, s)| *s)
            .fold(0.0_f32, f32::max);
        let vec_norm = if max_vec > 0.0 { max_vec } else { 1.0 };

        for (id, score) in vector_results {
            let normalized = score / vec_norm;
            merged
                .entry(id.clone())
                .and_modify(|r| r.vector_score = Some(normalized))
                .or_insert(RankedResult {
                    id: id.clone(),
                    text_score: None,
                    vector_score: Some(normalized),
                    final_score: 0.0,
                });
        }

        // Compute final scores
        let mut results: Vec<RankedResult> = merged
            .into_values()
            .map(|mut r| {
                let vec_s = r.vector_score.unwrap_or(0.0);
                let txt_s = r.text_score.unwrap_or(0.0);
                r.final_score = self.vector_weight * vec_s + self.text_weight * txt_s;
                r
            })
            .collect();

        // Sort by final score descending
        results.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        results
    }
}

impl Default for HybridRanker {
    fn default() -> Self {
        Self::balanced()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranker_new_sets_weights() {
        let r = HybridRanker::new(0.7, 0.3);
        assert!((r.vector_weight - 0.7).abs() < 0.001);
        assert!((r.text_weight - 0.3).abs() < 0.001);
    }

    #[test]
    fn ranker_balanced() {
        let r = HybridRanker::balanced();
        assert!((r.vector_weight - 0.5).abs() < 0.001);
        assert!((r.text_weight - 0.5).abs() < 0.001);
    }

    #[test]
    fn ranker_vector_focused() {
        let r = HybridRanker::vector_focused();
        assert!(r.vector_weight > r.text_weight);
    }

    #[test]
    fn ranker_text_focused() {
        let r = HybridRanker::text_focused();
        assert!(r.text_weight > r.vector_weight);
    }

    #[test]
    fn merge_empty_inputs() {
        let r = HybridRanker::balanced();
        let result = r.merge(&[], &[], 10);
        assert!(result.is_empty());
    }

    #[test]
    fn merge_text_only() {
        let r = HybridRanker::new(0.5, 0.5);
        let text = vec![("doc1".to_string(), 1.0)];
        let result = r.merge(&text, &[], 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "doc1");
        assert!(result[0].text_score.is_some());
        assert!(result[0].vector_score.is_none());
    }

    #[test]
    fn merge_vector_only() {
        let r = HybridRanker::new(0.5, 0.5);
        let vector = vec![("doc1".to_string(), 0.8)];
        let result = r.merge(&[], &vector, 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "doc1");
        assert!(result[0].vector_score.is_some());
        assert!(result[0].text_score.is_none());
    }

    #[test]
    fn merge_combines_scores() {
        let r = HybridRanker::new(0.5, 0.5);
        let text = vec![("doc1".to_string(), 0.8)];
        let vector = vec![("doc1".to_string(), 0.6)];

        let result = r.merge(&text, &vector, 10);
        assert_eq!(result.len(), 1);

        // Both normalized to 1.0 (they're the max), then combined:
        // 0.5 * 1.0 + 0.5 * 1.0 = 1.0
        assert!((result[0].final_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn merge_respects_limit() {
        let r = HybridRanker::balanced();
        let text: Vec<(String, f32)> = (0..20).map(|i| (format!("doc{}", i), 0.5)).collect();

        let result = r.merge(&text, &[], 5);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn merge_limit_zero() {
        let r = HybridRanker::balanced();
        let text = vec![("doc1".to_string(), 1.0)];
        let result = r.merge(&text, &[], 0);
        assert!(result.is_empty());
    }

    #[test]
    fn merge_deduplicates() {
        let r = HybridRanker::balanced();
        let text = vec![
            ("doc1".to_string(), 0.9),
            ("doc1".to_string(), 0.3), // duplicate in same input
        ];

        let result = r.merge(&text, &[], 10);
        assert_eq!(result.len(), 1, "Should deduplicate by ID");
    }

    #[test]
    fn merge_sorts_by_score_descending() {
        let r = HybridRanker::balanced();
        let text = vec![
            ("low".to_string(), 0.3),
            ("high".to_string(), 0.9),
            ("mid".to_string(), 0.6),
        ];

        let result = r.merge(&text, &[], 10);
        assert!(result[0].final_score >= result[1].final_score);
        assert!(result[1].final_score >= result[2].final_score);
    }

    #[test]
    fn merge_normalizes_text_scores() {
        let r = HybridRanker::new(0.0, 1.0); // Pure text
        let text = vec![
            ("a".to_string(), 100.0), // Large BM25 score
            ("b".to_string(), 50.0),
        ];

        let result = r.merge(&text, &[], 10);
        // Max (100.0) should normalize to 1.0
        assert!((result[0].text_score.unwrap() - 1.0).abs() < 0.001);
        // 50.0 / 100.0 = 0.5
        assert!((result[1].text_score.unwrap() - 0.5).abs() < 0.001);
    }

    #[test]
    fn merge_normalizes_vector_scores() {
        let r = HybridRanker::new(1.0, 0.0); // Pure vector
        let vector = vec![("a".to_string(), 0.8), ("b".to_string(), 0.4)];

        let result = r.merge(&[], &vector, 10);
        // Already normalized but we normalize again for safety
        assert!((result[0].vector_score.unwrap() - 1.0).abs() < 0.001);
        assert!((result[1].vector_score.unwrap() - 0.5).abs() < 0.001);
    }

    #[test]
    fn merge_handles_negative_scores() {
        let r = HybridRanker::balanced();
        let text = vec![("a".to_string(), -5.0), ("b".to_string(), -1.0)];

        // Should not panic
        let result = r.merge(&text, &[], 10);
        assert_eq!(result.len(), 2);
        // All scores should be finite
        for r in &result {
            assert!(r.final_score.is_finite());
        }
    }

    #[test]
    fn merge_zero_weights() {
        let r = HybridRanker::new(0.0, 0.0);
        let text = vec![("a".to_string(), 1.0)];
        let vector = vec![("a".to_string(), 1.0)];

        let result = r.merge(&text, &vector, 10);
        // All scores should be 0
        for r in &result {
            assert!(r.final_score.abs() < 0.001);
        }
    }

    #[test]
    fn ranked_result_debug() {
        let r = RankedResult {
            id: "test".to_string(),
            text_score: Some(0.5),
            vector_score: Some(0.8),
            final_score: 0.65,
        };
        let debug = format!("{:?}", r);
        assert!(debug.contains("test"));
    }

    #[test]
    fn ranked_result_partial_eq() {
        let r1 = RankedResult {
            id: "test".to_string(),
            text_score: Some(0.5),
            vector_score: Some(0.8),
            final_score: 0.65,
        };
        let r2 = RankedResult {
            id: "test".to_string(),
            text_score: Some(0.5),
            vector_score: Some(0.8),
            final_score: 0.65,
        };
        assert_eq!(r1, r2);
    }

    #[test]
    fn default_ranker_is_balanced() {
        let r = HybridRanker::default();
        assert!((r.vector_weight - 0.5).abs() < 0.001);
        assert!((r.text_weight - 0.5).abs() < 0.001);
    }
}
