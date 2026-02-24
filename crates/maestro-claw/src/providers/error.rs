//! Provider error types

use thiserror::Error;

/// Error type for provider operations
#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    /// API key is missing or invalid
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {0} seconds")]
    RateLimitExceeded(u64),

    /// Request timeout
    #[error("Request timed out after {0} seconds")]
    Timeout(u64),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Model not found or not available
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Content filtered by provider
    #[error("Content filtered: {0}")]
    ContentFiltered(String),

    /// Provider returned an error
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Response parsing failed
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Tool call error
    #[error("Tool call error: {0}")]
    ToolCallError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Provider unavailable
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
}

impl ProviderError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded(_)
                | Self::Timeout(_)
                | Self::NetworkError(_)
                | Self::Unavailable(_)
        )
    }

    /// Check if this is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Self::AuthenticationFailed(_))
    }

    /// Check if this is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimitExceeded(_))
    }

    /// Get retry-after seconds if available
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Self::RateLimitExceeded(secs) => Some(*secs),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error() {
        let err = ProviderError::AuthenticationFailed("invalid key".to_string());
        assert!(err.is_auth_error());
        assert!(!err.is_rate_limit());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_rate_limit_error() {
        let err = ProviderError::RateLimitExceeded(60);
        assert!(err.is_rate_limit());
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(60));
    }

    #[test]
    fn test_timeout_error() {
        let err = ProviderError::Timeout(30);
        assert!(err.is_retryable());
        assert!(!err.is_rate_limit());
    }

    #[test]
    fn test_network_error() {
        let err = ProviderError::NetworkError("connection reset".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_model_not_found() {
        let err = ProviderError::ModelNotFound("gpt-5".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_parse_error() {
        let err = ProviderError::ParseError("invalid JSON".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = ProviderError::RateLimitExceeded(30);
        let msg = format!("{}", err);
        assert!(msg.contains("30"));
        assert!(msg.contains("Rate limit"));
    }
}
