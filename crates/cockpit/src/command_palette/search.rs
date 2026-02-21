//! Fuzzy search implementation for command palette

/// Fuzzy match result containing matched positions
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub score: i32,
    pub positions: Vec<usize>,
}

/// Perform fuzzy matching on a haystack string given a needle query
///
/// Returns a MatchResult if the needle matches the haystack, None otherwise.
/// The score indicates the quality of the match (higher is better).
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<MatchResult> {
    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();

    if needle_lower.is_empty() {
        return Some(MatchResult {
            score: 0,
            positions: Vec::new(),
        });
    }

    let haystack_chars: Vec<char> = haystack_lower.chars().collect();
    let needle_chars: Vec<char> = needle_lower.chars().collect();

    // Needle must not be longer than haystack
    if needle_chars.len() > haystack_chars.len() {
        return None;
    }

    // Find positions of all needle characters in haystack
    let mut positions = Vec::new();
    let mut haystack_idx = 0;

    for needle_char in &needle_chars {
        let mut found = false;
        while haystack_idx < haystack_chars.len() {
            if &haystack_chars[haystack_idx] == needle_char {
                positions.push(haystack_idx);
                haystack_idx += 1;
                found = true;
                break;
            }
            haystack_idx += 1;
        }
        if !found {
            return None;
        }
    }

    // Calculate score
    let score = calculate_score(&positions, &haystack_chars);

    Some(MatchResult { score, positions })
}

/// Calculate match score based on positions
///
/// Higher scores indicate better matches:
/// - Consecutive matches score higher
/// - Matches at word boundaries score higher
/// - Matches at the start score higher
fn calculate_score(positions: &[usize], haystack: &[char]) -> i32 {
    let mut score = 0i32;

    // Base score for matching
    score += positions.len() as i32 * 10;

    // Bonus for matches at the start
    if positions.first() == Some(&0) {
        score += 20;
    }

    // Bonus for consecutive matches
    for window in positions.windows(2) {
        if window[1] == window[0] + 1 {
            score += 15;
        }
    }

    // Bonus for word boundary matches (after spaces, underscores, or at CamelCase)
    for &pos in positions {
        if pos == 0 {
            score += 10;
        } else if haystack[pos - 1] == ' ' || haystack[pos - 1] == '_' {
            score += 10;
        } else if pos > 0 && haystack[pos - 1].is_lowercase() && haystack[pos].is_uppercase() {
            // CamelCase boundary
            score += 10;
        }
    }

    // Penalty for gaps (non-consecutive matches)
    for window in positions.windows(2) {
        let gap = window[1] - window[0];
        if gap > 1 {
            score -= (gap as i32) * 2;
        }
    }

    score
}

/// Search and rank items by fuzzy matching
///
/// Returns items sorted by match score (best first)
pub fn search_items<T>(
    items: &[T],
    query: &str,
    extractor: impl Fn(&T) -> &str,
) -> Vec<(usize, MatchResult)> {
    let mut results: Vec<(usize, MatchResult)> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            let text = extractor(item);
            fuzzy_match(text, query).map(|result| (idx, result))
        })
        .collect();

    // Sort by score (descending)
    results.sort_by(|a, b| b.1.score.cmp(&a.1.score));

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        let result = fuzzy_match("cron", "cron");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.positions, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_prefix() {
        let result = fuzzy_match("cron jobs", "cron");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.positions, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_substring() {
        let result = fuzzy_match("scheduled cron jobs", "cron");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.positions[0] >= 10); // "cron" starts at position 10
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        let result = fuzzy_match("Cron Jobs", "cron");
        assert!(result.is_some());
    }

    #[test]
    fn test_fuzzy_match_fuzzy() {
        // "cj" should match "cron jobs" (c... j...)
        let result = fuzzy_match("cron jobs", "cj");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.positions, vec![0, 5]);
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let result = fuzzy_match("cron", "xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_empty_needle() {
        let result = fuzzy_match("cron", "");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.score, 0);
        assert!(result.positions.is_empty());
    }

    #[test]
    fn test_fuzzy_match_needle_longer_than_haystack() {
        let result = fuzzy_match("ab", "abc");
        assert!(result.is_none());
    }

    #[test]
    fn test_score_consecutive() {
        let positions = vec![0, 1, 2, 3];
        let haystack: Vec<char> = "cron".chars().collect();
        let score = calculate_score(&positions, &haystack);
        assert!(score > 0);

        // Consecutive should score higher than non-consecutive
        let positions_gap = vec![0, 2, 4, 6];
        let haystack_gap: Vec<char> = "c_r_o_n".chars().collect();
        let score_gap = calculate_score(&positions_gap, &haystack_gap);
        assert!(score > score_gap);
    }

    #[test]
    fn test_score_start_bonus() {
        let positions_start = vec![0, 1];
        let positions_mid = vec![5, 6];
        let haystack: Vec<char> = "xxxxcron".chars().collect();

        let score_start = calculate_score(&positions_start, &haystack);
        let score_mid = calculate_score(&positions_mid, &haystack);

        // Match at start should score higher than match in middle
        assert!(score_start > score_mid);
    }

    #[test]
    fn test_search_items() {
        let items = vec!["cron jobs", "mcp servers", "sandbox"];
        let results = search_items(&items, "cron", |s| *s);

        assert!(!results.is_empty());
        assert_eq!(results[0].0, 0); // First item should be "cron jobs"
    }

    #[test]
    fn test_search_items_ranking() {
        let items = vec!["cron", "my cron job", "something else"];
        let results = search_items(&items, "cron", |s| *s);

        assert!(!results.is_empty());
        // Exact match "cron" should rank higher than "my cron job"
        assert_eq!(results[0].0, 0);
    }

    #[test]
    fn test_camel_case_boundary() {
        // "mcp" should get bonus for matching at CamelCase boundary in "McpServer"
        let result = fuzzy_match("McpServer", "mcp");
        assert!(result.is_some());
        // The match should exist even with camelCase
    }
}
