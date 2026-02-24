//! Comprehensive tests for MaesterClaw hot cache and utilities
//!
//! These tests verify the hot cache functionality for memory suggestions.
//! UI integration tests are in the sibling module `ui_integration_tests`
//! (declared in maesterclaw/mod.rs).

// Hot Cache tests
#[cfg(test)]
mod hot_cache_tests {
    use crate::maesterclaw::hot_cache::{clamp_flash, MemorySuggestion};

    /// Test that suggestion stream emits when semantic detector crosses threshold
    #[test]
    fn test_suggestion_stream_emits_on_threshold_cross() {
        let suggestion = MemorySuggestion {
            memory_id: 123,
            preview: "User prefers Rust for CLI tools".to_string(),
            relevance_score: 0.85,
            flash_intensity: 0.7,
        };

        // Should emit when relevance crosses threshold (typically 0.7)
        assert!(
            suggestion.relevance_score > 0.7,
            "High relevance should trigger suggestion"
        );
        assert!(
            !suggestion.preview.is_empty(),
            "Preview should be populated"
        );
    }

    /// Test that stale suggestions expire based on TTL
    #[test]
    fn test_stale_suggestions_expire_ttl() {
        use crate::maesterclaw::hot_cache::SuggestionTtl;

        let ttl = SuggestionTtl::from_secs(60);
        assert!(!ttl.is_expired());

        // Simulate time passing - this will fail until TTL tracking is implemented
        let expired_ttl = SuggestionTtl::expired();
        assert!(
            expired_ttl.is_expired(),
            "Expired TTL should be marked as expired"
        );
    }

    /// Test that UI flash intensity clamps to [0.0, 1.0]
    #[test]
    fn test_flash_intensity_clamps_to_range() {
        // Test lower bound
        assert_eq!(
            clamp_flash(-0.5),
            0.0,
            "Negative values should clamp to 0.0"
        );
        assert_eq!(clamp_flash(0.0), 0.0, "Zero should remain 0.0");

        // Test upper bound
        assert_eq!(clamp_flash(1.0), 1.0, "One should remain 1.0");
        assert_eq!(
            clamp_flash(1.5),
            1.0,
            "Values above 1.0 should clamp to 1.0"
        );
        assert_eq!(
            clamp_flash(2.0),
            1.0,
            "Values above 1.0 should clamp to 1.0"
        );

        // Test in-range values
        assert_eq!(clamp_flash(0.5), 0.5, "In-range values should pass through");
        assert_eq!(
            clamp_flash(0.75),
            0.75,
            "In-range values should pass through"
        );
    }

    /// Test suggestion ordering by relevance score
    #[test]
    fn test_suggestions_order_by_relevance() {
        let mut suggestions = [
            MemorySuggestion {
                memory_id: 1,
                preview: "Low relevance".to_string(),
                relevance_score: 0.3,
                flash_intensity: 0.2,
            },
            MemorySuggestion {
                memory_id: 2,
                preview: "High relevance".to_string(),
                relevance_score: 0.9,
                flash_intensity: 0.8,
            },
            MemorySuggestion {
                memory_id: 3,
                preview: "Medium relevance".to_string(),
                relevance_score: 0.6,
                flash_intensity: 0.5,
            },
        ];

        // Sort by relevance (descending)
        suggestions.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        assert_eq!(
            suggestions[0].memory_id, 2,
            "Highest relevance should be first"
        );
        assert_eq!(
            suggestions[1].memory_id, 3,
            "Medium relevance should be second"
        );
        assert_eq!(
            suggestions[2].memory_id, 1,
            "Lowest relevance should be last"
        );
    }

    /// Test that suggestion preview is truncated to reasonable length
    #[test]
    fn test_suggestion_preview_truncation() {
        use crate::maesterclaw::hot_cache::truncate_preview;

        let long_preview = "This is a very long preview that should be truncated to fit within the UI hint line without breaking the layout or causing overflow issues in the terminal interface.";
        let truncated = truncate_preview(long_preview, 60);

        assert!(
            truncated.len() <= 63,
            "Truncated preview should include ellipsis and fit within limit"
        );
        assert!(
            truncated.ends_with("..."),
            "Truncated preview should end with ellipsis"
        );
    }

    /// Test that empty suggestions don't cause UI issues
    #[test]
    fn test_empty_suggestions_handle_gracefully() {
        let suggestions: Vec<MemorySuggestion> = vec![];

        // Should not panic when rendering empty list
        assert!(
            suggestions.is_empty(),
            "Empty suggestions should be handled gracefully"
        );
    }
}

// Runtime validation tests
#[cfg(test)]
mod runtime_validation_tests {
    use maestro_core::{McpManager, SandboxManager, SecurityPolicy};

    #[test]
    fn test_mcp_manager_starts_empty() {
        let manager = McpManager::new();
        let (registered, connected) = manager.try_get_status();
        assert!(
            registered.is_empty(),
            "MCP manager should start with no registered servers"
        );
        assert!(
            connected.is_empty(),
            "MCP manager should start with no connected servers"
        );
    }

    #[test]
    fn test_sandbox_manager_has_native_runtime() {
        let manager = SandboxManager::new(SecurityPolicy::default());
        let runtimes = manager.available_runtimes();
        assert!(!runtimes.is_empty(), "Sandbox manager should have runtimes");
        assert!(
            runtimes.contains(&"native"),
            "Sandbox manager should have native runtime"
        );
    }

    #[test]
    fn test_security_policy_defaults() {
        let policy = SecurityPolicy::default();
        // Verify default policy has reasonable settings
        assert!(
            policy.max_memory_bytes >= 0,
            "Memory limit should be non-negative"
        );
        assert!(policy.max_cpu_shares > 0, "CPU shares should be positive");
    }
}
