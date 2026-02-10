//! Advanced rate-limit detection (Ralph TUI parity)
//!
//! Mirrors the logic from `ralph-tui/src/engine/rate-limit-detector.ts`:
//! - Regex-based detection for common LLM APIs (Anthropic/Claude, OpenAI/Azure)
//! - Looks at stderr first (avoids stdout false positives), falls back to stdout
//! - Parses Retry-After style hints when present

use regex::Regex;
use std::collections::HashMap;

/// Detection input captured from an agent execution.
#[derive(Debug, Clone)]
pub struct RateLimitDetectionInput {
    pub stderr: String,
    pub stdout: String,
    pub exit_code: Option<i32>,
    pub agent_id: Option<String>,
}

/// Detection result (mirrors Ralph’s shape).
#[derive(Debug, Clone)]
pub struct RateLimitDetectionResult {
    pub is_rate_limit: bool,
    pub message: Option<String>,
    pub retry_after: Option<u64>,
}

impl RateLimitDetectionResult {
    pub fn not_limited() -> Self {
        Self {
            is_rate_limit: false,
            message: None,
            retry_after: None,
        }
    }
}

#[derive(Clone)]
struct RateLimitPattern {
    pattern: Regex,
    retry_after_pattern: Option<Regex>,
}

/// Regex-driven detector.
pub struct RateLimitDetector {
    common_patterns: Vec<RateLimitPattern>,
    agent_specific: HashMap<String, Vec<RateLimitPattern>>,
    loose_patterns: Vec<Regex>,
}

impl RateLimitDetector {
    pub fn new() -> Self {
        Self {
            common_patterns: vec![
                RateLimitPattern {
                    pattern: Regex::new(r"(?:HTTP|status|error|code|response)[\s:]*429|429\s*(?:too many|rate limit|error)")
                        .expect("valid regex"),
                    retry_after_pattern: Some(
                        Regex::new(r"retry[- ]?after[:\s]+(\d+)\s*s?").expect("valid regex"),
                    ),
                },
                RateLimitPattern {
                    pattern: Regex::new(r"rate[- ]limit").expect("valid regex"),
                    retry_after_pattern: Some(
                        Regex::new(r"retry[- ]?after[:\s]+(\d+)\s*s?").expect("valid regex"),
                    ),
                },
                RateLimitPattern {
                    pattern: Regex::new(r"too many requests").expect("valid regex"),
                    retry_after_pattern: Some(Regex::new(r"(\d+)\s*seconds?").expect("valid regex")),
                },
                RateLimitPattern {
                    pattern: Regex::new(r"quota[- ]?exceeded").expect("valid regex"),
                    retry_after_pattern: Some(Regex::new(r"(\d+)\s*seconds?").expect("valid regex")),
                },
                RateLimitPattern {
                    pattern: Regex::new(r"\boverloaded\b").expect("valid regex"),
                    retry_after_pattern: Some(Regex::new(r"(\d+)\s*seconds?").expect("valid regex")),
                },
            ],
            agent_specific: {
                let mut map = HashMap::new();
                map.insert(
                    "claude".to_string(),
                    vec![
                        RateLimitPattern {
                            pattern: Regex::new(r"anthropic.*rate[- ]?limit").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"retry[- ]?after[:\s]+(\d+)\s*s?").expect("valid regex"),
                            ),
                        },
                        RateLimitPattern {
                            pattern: Regex::new(r"API rate limit exceeded").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"wait[:\s]+(\d+)\s*s?").expect("valid regex"),
                            ),
                        },
                        RateLimitPattern {
                            pattern: Regex::new(r"claude.*is currently overloaded")
                                .expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"(\d+)\s*seconds?").expect("valid regex"),
                            ),
                        },
                        RateLimitPattern {
                            pattern: Regex::new(r"api[- ]?error.*429").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"retry[- ]?after[:\s]+(\d+)").expect("valid regex"),
                            ),
                        },
                    ],
                );
                map.insert(
                    "opencode".to_string(),
                    vec![
                        RateLimitPattern {
                            pattern: Regex::new(r"openai.*rate[- ]?limit").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"retry[- ]?after[:\s]+(\d+)\s*s?").expect("valid regex"),
                            ),
                        },
                        RateLimitPattern {
                            pattern: Regex::new(r"tokens per minute").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"(\d+)\s*seconds?").expect("valid regex"),
                            ),
                        },
                        RateLimitPattern {
                            pattern: Regex::new(r"requests per minute").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"(\d+)\s*seconds?").expect("valid regex"),
                            ),
                        },
                        RateLimitPattern {
                            pattern: Regex::new(r"azure.*throttl").expect("valid regex"),
                            retry_after_pattern: Some(
                                Regex::new(r"(\d+)\s*seconds?").expect("valid regex"),
                            ),
                        },
                    ],
                );
                map
            },
            loose_patterns: vec![
                Regex::new(r"throttl").expect("valid regex"),
                Regex::new(r"limit.*exceeded").expect("valid regex"),
                Regex::new(r"exceeded.*limit").expect("valid regex"),
                Regex::new(r"capacity").expect("valid regex"),
                Regex::new(r"backoff").expect("valid regex"),
            ],
        }
    }

    /// Detect rate-limit signals from stderr/stdout + exit code.
    pub fn detect(&self, input: RateLimitDetectionInput) -> RateLimitDetectionResult {
        // Prefer stderr to avoid false positives from stdout (mirrors Ralph)
        let candidates = [
            input.stderr.as_str(),
            input.stdout.as_str(), // fallback
        ];

        let patterns = self.patterns_for_agent(input.agent_id.as_deref());

        for &text in candidates.iter() {
            if text.trim().is_empty() {
                continue;
            }

            for pat in &patterns {
                if pat.pattern.is_match(text) {
                    return RateLimitDetectionResult {
                        is_rate_limit: true,
                        message: Some(Self::extract_message(text, &pat.pattern)),
                        retry_after: Self::extract_retry_after(
                            text,
                            pat.retry_after_pattern.as_ref(),
                        )
                        .or_else(|| Self::extract_retry_after_header(text)),
                    };
                }
            }
        }

        // Exit-code assisted loose detection
        if let Some(code) = input.exit_code {
            if code != 0 && self.is_rate_limit_exit_code(code) {
                for &text in candidates.iter() {
                    if text.trim().is_empty() {
                        continue;
                    }
                    if let Some(msg) = self.loose_rate_limit_check(text) {
                        return RateLimitDetectionResult {
                            is_rate_limit: true,
                            message: Some(msg),
                            retry_after: Self::extract_retry_after_header(text),
                        };
                    }
                }
            }
        }

        RateLimitDetectionResult::not_limited()
    }

    fn patterns_for_agent(&self, agent_id: Option<&str>) -> Vec<RateLimitPattern> {
        let mut patterns = self.common_patterns.clone();
        if let Some(agent) = agent_id {
            if let Some(extra) = self.agent_specific.get(agent) {
                patterns.extend(extra.clone());
            }
        }
        patterns
    }

    fn extract_message(text: &str, pattern: &Regex) -> String {
        if let Some(mat) = pattern.find(text) {
            let start = text[..mat.start()]
                .char_indices()
                .rev()
                .take(50)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end = text[mat.end()..]
                .char_indices()
                .take(100)
                .last()
                .map(|(i, _)| mat.end() + i)
                .unwrap_or_else(|| text.len());
            let mut snippet = text[start..end].trim().replace('\n', " ");
            if snippet.len() > 200 {
                snippet.truncate(200);
            }
            snippet
        } else {
            "Rate limit detected".to_string()
        }
    }

    fn extract_retry_after(text: &str, pattern: Option<&Regex>) -> Option<u64> {
        if let Some(pat) = pattern {
            if let Some(caps) = pat.captures(text) {
                if let Some(m) = caps.get(1) {
                    if let Ok(val) = m.as_str().trim().parse::<u64>() {
                        if (1..=3600).contains(&val) {
                            return Some(val);
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_retry_after_header(text: &str) -> Option<u64> {
        let header = Regex::new(r"retry[- ]?after[:\s]+(\d+)\s*s?").expect("valid regex");
        Self::extract_retry_after(text, Some(&header))
    }

    fn loose_rate_limit_check(&self, text: &str) -> Option<String> {
        for pat in &self.loose_patterns {
            if pat.is_match(text) {
                return Some(Self::extract_message(text, pat));
            }
        }
        None
    }

    fn is_rate_limit_exit_code(&self, code: i32) -> bool {
        matches!(code, 1 | 2 | 429)
    }
}
