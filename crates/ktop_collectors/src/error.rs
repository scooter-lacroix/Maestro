//! Error types for ktop collectors
//!
//! This module defines all error types used across the collector modules.

use std::fmt;

/// Result type alias for collector operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for ktop collectors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Failed to read CPU metrics
    CpuReadFailed(String),

    /// Failed to read memory metrics
    MemoryReadFailed(String),

    /// Failed to read process information
    ProcessReadFailed(String),

    /// Failed to read network statistics
    NetworkReadFailed(String),

    /// Failed to read disk information
    DiskReadFailed(String),

    /// Failed to read Maestro-specific metrics
    MaestroMetricsFailed(String),

    /// Invalid collector configuration
    InvalidConfig(String),

    /// Collector not initialized
    NotInitialized(String),

    /// Operation timed out
    Timeout(String),

    /// Internal error
    Internal(String),

    /// Data collection failed
    CollectionFailed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CpuReadFailed(msg) => write!(f, "Failed to read CPU metrics: {}", msg),
            Error::MemoryReadFailed(msg) => write!(f, "Failed to read memory metrics: {}", msg),
            Error::ProcessReadFailed(msg) => write!(f, "Failed to read process info: {}", msg),
            Error::NetworkReadFailed(msg) => write!(f, "Failed to read network stats: {}", msg),
            Error::DiskReadFailed(msg) => write!(f, "Failed to read disk info: {}", msg),
            Error::MaestroMetricsFailed(msg) => {
                write!(f, "Failed to read Maestro metrics: {}", msg)
            }
            Error::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Error::NotInitialized(msg) => write!(f, "Collector not initialized: {}", msg),
            Error::Timeout(msg) => write!(f, "Operation timed out: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
            Error::CollectionFailed(msg) => write!(f, "Collection failed: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

// Note: sysinfo 0.32 removed SysError type
// If we need to convert from sysinfo errors in the future, we'll use String error messages

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", Error::CpuReadFailed("test".to_string())),
            "Failed to read CPU metrics: test"
        );
        assert_eq!(
            format!("{}", Error::MemoryReadFailed("test".to_string())),
            "Failed to read memory metrics: test"
        );
    }

    #[test]
    fn test_error_equality() {
        let err1 = Error::CpuReadFailed("test".to_string());
        let err2 = Error::CpuReadFailed("test".to_string());
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_error_clone() {
        let err1 = Error::NetworkReadFailed("network".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
