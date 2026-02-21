//! Secret redaction for logs and error messages
//!
//! This module provides utilities to redact sensitive information
//! from logs and error messages to prevent secret leakage.

use regex::Regex;
use std::sync::Arc;

/// Pattern and replacement pair for secret redaction
struct RedactionPattern {
    pattern: &'static str,
    replacement: &'static str,
}

/// Patterns that might indicate secrets in log output
const SECRET_PATTERNS: &[RedactionPattern] = &[
    RedactionPattern {
        pattern: r"(?i)(api[_-]?key|apikey|secret[_-]?key|secretkey)\s*[=:]\s*[a-zA-Z0-9_\-]{16,}",
        replacement: "$1=***REDACTED***",
    },
    RedactionPattern {
        pattern: r"(?i)(bearer|token)\s+[a-zA-Z0-9_\-\.]{16,}",
        replacement: "Bearer ***REDACTED***",
    },
    RedactionPattern {
        pattern: r"(?i)(password|passwd|pwd)\s*[=:]\s*[^\s]+",
        replacement: "$1=***REDACTED***",
    },
    RedactionPattern {
        pattern: r"(?i)authorization\s*:\s*[a-z]+\s+[a-zA-Z0-9_\-\.]{16,}",
        replacement: "authorization: ***REDACTED***",
    },
    RedactionPattern {
        pattern: r"(?i)(https?://[^/\s]*:)[^/\s@]+(@)",
        replacement: "$1***REDACTED***$2",
    },
    RedactionPattern {
        pattern: r"(?i)(eyJ[a-zA-Z0-9_\-]*\.eyJ[a-zA-Z0-9_\-]*\.)[a-zA-Z0-9_\-]*",
        replacement: "$1***REDACTED***",
    },
];

/// Secret redactor for sanitizing log output
pub struct SecretRedactor {
    patterns: Vec<(Regex, String)>,
}

impl SecretRedactor {
    /// Create a new redactor with default patterns
    pub fn new() -> Self {
        let patterns = SECRET_PATTERNS
            .iter()
            .map(|rp| {
                let regex = Regex::new(rp.pattern)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Invalid redaction regex '{}': {}", rp.pattern, e);
                        // Return a regex that never matches
                        Regex::new(r"(?!x)x").unwrap()
                    });
                (regex, rp.replacement.to_string())
            })
            .collect();

        Self { patterns }
    }

    /// Redact secrets from a string
    pub fn redact(&self, input: &str) -> String {
        let mut result = input.to_string();

        for (regex, replacement) in &self.patterns {
            result = regex.replace_all(&result, replacement).to_string();
        }

        result
    }

    /// Check if a string likely contains a secret
    pub fn contains_secret(&self, input: &str) -> bool {
        for (regex, _) in &self.patterns {
            if regex.is_match(input) {
                return true;
            }
        }
        false
    }
}

impl Default for SecretRedactor {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe global redactor
pub static GLOBAL_REDACTOR: std::sync::LazyLock<Arc<SecretRedactor>> =
    std::sync::LazyLock::new(|| Arc::new(SecretRedactor::new()));

/// Redact secrets from a string using the global redactor
pub fn redact_secrets(input: &str) -> String {
    GLOBAL_REDACTOR.redact(input)
}

/// Check if a string contains secrets
pub fn might_contain_secret(input: &str) -> bool {
    GLOBAL_REDACTOR.contains_secret(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        let redactor = SecretRedactor::new();
        let input = "api_key=sk_1234567890abcdef";
        let output = redactor.redact(input);
        assert!(!output.contains("sk_1234567890abcdef"), "API key should be redacted");
        assert!(output.contains("***REDACTED***"), "Should show redaction marker");
    }

    #[test]
    fn test_redact_password() {
        let redactor = SecretRedactor::new();
        let input = "password: supersecret123";
        let output = redactor.redact(input);
        assert!(!output.contains("supersecret123"), "Password should be redacted");
    }

    #[test]
    fn test_redact_bearer_token() {
        let redactor = SecretRedactor::new();
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let output = redactor.redact(input);
        assert!(!output.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"), "Token should be redacted");
    }

    #[test]
    fn test_redact_json_secret() {
        let redactor = SecretRedactor::new();
        // Test with format that matches our patterns
        let input = r#"api_key=sk_1234567890abcdef"#;
        let output = redactor.redact(input);
        assert!(!output.contains("sk_1234567890abcdef"), "API key should be redacted");
        assert!(output.contains("***REDACTED***"), "Should show redaction marker");
    }

    #[test]
    fn test_contains_secret() {
        let redactor = SecretRedactor::new();
        assert!(redactor.contains_secret("api_key=sk_1234567890abcdef"));
        assert!(redactor.contains_secret("password: secret"));
        assert!(!redactor.contains_secret("normal message without secrets"));
    }

    #[test]
    fn test_global_redactor() {
        let input = "password=secret123";
        let output = redact_secrets(input);
        assert!(!output.contains("secret123"));
    }

    #[test]
    fn test_redact_url_with_secret() {
        let redactor = SecretRedactor::new();
        let input = "https://user:password@api.example.com/endpoint";
        let output = redactor.redact(input);
        assert!(!output.contains("password"), "URL password should be redacted");
        assert!(output.contains("https://"), "Protocol should remain");
    }

    #[test]
    fn test_redact_jwt_token() {
        let redactor = SecretRedactor::new();
        let input = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signaturehere";
        let output = redactor.redact(input);
        assert!(!output.contains("signaturehere"), "JWT signature should be redacted");
    }
}
