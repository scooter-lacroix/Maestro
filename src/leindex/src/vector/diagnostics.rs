//! Vector Store Diagnostics
//!
//! Diagnostic helpers for debugging vector store issues.
//! Provides structured debugging output for:
//! - Store health and statistics
//! - Embedding analysis
//! - Cache performance
//! - Index integrity

use serde::{Deserialize, Serialize};
use std::fmt;

/// Diagnostic health status of a vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "HEALTHY"),
            HealthStatus::Degraded => write!(f, "DEGRADED"),
            HealthStatus::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Diagnostic report for a vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub store_type: String,
    pub health: HealthStatus,
    pub vector_count: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

impl fmt::Display for DiagnosticReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Vector Store Diagnostic Report ===")?;
        writeln!(f, "Store Type: {}", self.store_type)?;
        writeln!(f, "Health: {}", self.health)?;
        writeln!(f, "Vector Count: {}", self.vector_count)?;
        writeln!(f, "Cache Hit Rate: {:.2}%", self.cache_hit_rate * 100.0)?;
        writeln!(f, "Cache Hits: {}", self.cache_hits)?;
        writeln!(f, "Cache Misses: {}", self.cache_misses)?;

        if !self.issues.is_empty() {
            writeln!(f, "\nIssues:")?;
            for issue in &self.issues {
                writeln!(f, "  - {}", issue)?;
            }
        }

        if !self.warnings.is_empty() {
            writeln!(f, "\nWarnings:")?;
            for warning in &self.warnings {
                writeln!(f, "  - {}", warning)?;
            }
        }

        Ok(())
    }
}

/// Analyze embedding for potential issues
pub fn analyze_embedding(embedding: &[f32]) -> Vec<String> {
    let mut issues = Vec::new();

    if embedding.is_empty() {
        issues.push("Embedding is empty".to_string());
        return issues;
    }

    // Check for NaN values
    let nan_count = embedding.iter().filter(|v| v.is_nan()).count();
    if nan_count > 0 {
        issues.push(format!("Found {} NaN values in embedding", nan_count));
    }

    // Check for infinite values
    let inf_count = embedding.iter().filter(|v| v.is_infinite()).count();
    if inf_count > 0 {
        issues.push(format!("Found {} infinite values in embedding", inf_count));
    }

    // Check for all zeros
    let all_zero = embedding.iter().all(|v| *v == 0.0);
    if all_zero {
        issues.push("Embedding is all zeros (no information content)".to_string());
    }

    // Check for very large magnitude values
    let max_mag = embedding.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    if max_mag > 1000.0 {
        issues.push(format!(
            "Embedding has very large magnitude values (max: {:.2})",
            max_mag
        ));
    }

    // Check dimension constraints
    if embedding.is_empty() {
        issues.push("Embedding dimension is too small (< 1)".to_string());
    }
    if embedding.len() > 4096 {
        issues.push(format!(
            "Embedding dimension is very large ({} dims, max 4096)",
            embedding.len()
        ));
    }

    issues
}

/// Analyze multiple embeddings for consistency
pub fn analyze_embeddings_batch(embeddings: &[Vec<f32>]) -> Vec<String> {
    let mut issues = Vec::new();

    if embeddings.is_empty() {
        issues.push("Embeddings batch is empty".to_string());
        return issues;
    }

    // Check dimension consistency
    let first_dim = embeddings[0].len();
    let inconsistent: Vec<_> = embeddings
        .iter()
        .enumerate()
        .filter(|(_, e)| e.len() != first_dim)
        .map(|(i, e)| (i, e.len()))
        .collect();

    if !inconsistent.is_empty() {
        issues.push(format!(
            "Inconsistent dimensions: expected {} but found {:?} at indices {:?}",
            first_dim,
            inconsistent.iter().map(|(_, d)| d).collect::<Vec<_>>(),
            inconsistent.iter().map(|(i, _)| i).collect::<Vec<_>>()
        ));
    }

    // Check for duplicate embeddings
    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            if embeddings[i] == embeddings[j] {
                issues.push(format!(
                    "Duplicate embeddings found at indices {} and {}",
                    i, j
                ));
            }
        }
    }

    issues
}

/// Validate cosine similarity score
pub fn validate_similarity_score(score: f32) -> Vec<String> {
    let mut issues = Vec::new();

    if score.is_nan() {
        issues.push("Similarity score is NaN".to_string());
    }

    if !(-1.0..=1.0).contains(&score) {
        issues.push(format!(
            "Similarity score out of valid range [-1.0, 1.0]: {:.4}",
            score
        ));
    }

    issues
}

/// Format vector count with thousands separator
pub fn format_count(count: usize) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        format!("{}", count)
    }
}

/// Format duration in human-readable format
pub fn format_duration(duration_secs: f64) -> String {
    if duration_secs >= 3600.0 {
        format!("{:.1}h", duration_secs / 3600.0)
    } else if duration_secs >= 60.0 {
        format!("{:.1}m", duration_secs / 60.0)
    } else if duration_secs >= 1.0 {
        format!("{:.1}s", duration_secs)
    } else {
        format!("{:.0}ms", duration_secs * 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_embedding() {
        let valid_embedding = vec![0.1; 768];
        let issues = analyze_embedding(&valid_embedding);
        assert!(issues.is_empty(), "Valid embedding should have no issues");

        // Test NaN detection
        let mut nan_embedding = vec![0.1; 768];
        nan_embedding[0] = f32::NAN;
        let issues = analyze_embedding(&nan_embedding);
        assert!(issues.iter().any(|i| i.contains("NaN")));

        // Test zero embedding
        let zero_embedding = vec![0.0; 768];
        let issues = analyze_embedding(&zero_embedding);
        assert!(issues.iter().any(|i| i.contains("all zeros")));
    }

    #[test]
    fn test_analyze_embeddings_batch() {
        let embeddings = vec![vec![0.1; 768], vec![0.2; 768]];
        let issues = analyze_embeddings_batch(&embeddings);
        assert!(issues.is_empty());

        // Test inconsistent dimensions
        let inconsistent = vec![vec![0.1; 768], vec![0.2; 384]];
        let issues = analyze_embeddings_batch(&inconsistent);
        assert!(issues.iter().any(|i| i.contains("Inconsistent")));
    }

    #[test]
    fn test_validate_similarity_score() {
        let issues = validate_similarity_score(0.5);
        assert!(issues.is_empty());

        let issues = validate_similarity_score(f32::NAN);
        assert!(issues.iter().any(|i| i.contains("NaN")));

        let issues = validate_similarity_score(1.5);
        assert!(issues.iter().any(|i| i.contains("out of valid range")));
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(100), "100");
        assert_eq!(format_count(1_500), "1.5K");
        assert_eq!(format_count(1_500_000), "1.5M");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.5), "500ms");
        assert_eq!(format_duration(5.0), "5.0s");
        assert_eq!(format_duration(90.0), "1.5m");
        assert_eq!(format_duration(7200.0), "2.0h");
    }

    #[test]
    fn test_diagnostic_report_display() {
        let report = DiagnosticReport {
            store_type: "Linear".to_string(),
            health: HealthStatus::Healthy,
            vector_count: 1000,
            cache_hits: 800,
            cache_misses: 200,
            cache_hit_rate: 0.8,
            issues: vec![],
            warnings: vec!["Low cache hit rate".to_string()],
        };

        let display = format!("{}", report);
        assert!(display.contains("HEALTHY"));
        assert!(display.contains("1000"));
        assert!(display.contains("80.00%"));
    }
}
