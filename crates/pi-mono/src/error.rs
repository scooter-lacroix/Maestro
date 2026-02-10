//! # Error types for maestro-pi-mono
//!
//! This module defines error types used throughout the maestro-pi-mono crate.
//!
//! ## Example
//!
//! ```rust
//! use maestro_pi_mono::error::{ExecutionError, Error, Result};
//!
//! fn execute_command() -> Result<()> {
//!     Err(Error::Execution(ExecutionError::Timeout {
//!         command: "status".to_string(),
//!         timeout_secs: 300,
//!     }))
//! }
//! ```
//!
//! ## Error Categories
//!
//! - `ExecutionError`: Command execution failures (timeout, retry exhausted, non-zero exit, validation)
//! - `ConfigError`: Configuration loading and validation errors (invalid path, load failed, missing field)
//! - `DetectionError`: CLI detection and discovery errors (not found, version parse failed, execution failed)
//! - `SerializationError`: JSON/YAML parsing errors (via serde_json::Error, serde_yaml::Error)
//! - `IoError`: Standard I/O errors (via std::io::Error)
//!
//! All errors implement `std::error::Error` and provide context for debugging.

use thiserror::Error;

/// Result type alias for maestro-pi-mono operations.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::error::Result;
///
/// fn do_something() -> Result<String> {
///     Ok("success".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

/// Core error type for maestro-pi-mono.
///
/// This enum represents all possible errors that can occur when using
/// the maestro-pi-mono crate.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::error::{Error, ExecutionError};
///
/// let error = Error::Execution(ExecutionError::Timeout {
///     command: "run-task".to_string(),
///     timeout_secs: 300,
/// });
///
/// assert!(error.is_execution());
/// assert!(error.to_string().contains("execution error"));
/// assert!(error.to_string().contains("run-task"));
/// ```
#[derive(Error, Debug)]
pub enum Error {
    /// Errors that occur during command execution
    #[error("execution error: {0}")]
    Execution(#[from] ExecutionError),

    /// Errors that occur during configuration loading
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Generic I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML serialization/deserialization errors
    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Other errors that don't fit into specific categories
    #[error("unknown error: {0}")]
    Other(String),

    /// Errors that occur during CLI detection
    #[error("detection error: {0}")]
    Detection(#[from] DetectionError),
}

impl Error {
    /// Returns `true` if this error is an execution error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::{Error, ExecutionError};
    ///
    /// let error = Error::Execution(ExecutionError::Timeout {
    ///     command: "test".to_string(),
    ///     timeout_secs: 60,
    /// });
    ///
    /// assert!(error.is_execution());
    /// ```
    pub fn is_execution(&self) -> bool {
        matches!(self, Error::Execution(_))
    }

    /// Returns `true` if this error is a configuration error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::{Error, ConfigError};
    ///
    /// let error = Error::Config(ConfigError::InvalidPath {
    ///     path: "/invalid/path".to_string(),
    /// });
    ///
    /// assert!(error.is_config());
    /// ```
    pub fn is_config(&self) -> bool {
        matches!(self, Error::Config(_))
    }

    /// Returns `true` if this error is a detection error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::{Error, DetectionError};
    ///
    /// let error = Error::Detection(DetectionError::NotFound);
    ///
    /// assert!(error.is_detection());
    /// ```
    pub fn is_detection(&self) -> bool {
        matches!(self, Error::Detection(_))
    }
}

/// Errors that can occur during Pi-Mono CLI detection.
///
/// These errors cover failures when discovering the pi-mono executable,
/// determining its version, and checking available capabilities.
#[derive(Error, Debug)]
pub enum DetectionError {
    /// The pi-mono executable was not found in any of the standard locations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::DetectionError;
    ///
    /// let error = DetectionError::NotFound;
    ///
    /// assert!(error.to_string().contains("not found"));
    /// ```
    #[error("pi-mono executable not found in any standard location")]
    NotFound,

    /// Failed to parse the version string from pi-mono.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::DetectionError;
    ///
    /// let error = DetectionError::VersionParseFailed {
    ///     output: "invalid version".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("invalid version"));
    /// ```
    #[error("failed to parse version from output: '{output}'")]
    VersionParseFailed {
        /// The raw output that could not be parsed
        output: String,
    },

    /// Command execution failed during detection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::DetectionError;
    ///
    /// let error = DetectionError::ExecutionFailed {
    ///     command: "pi --version".to_string(),
    ///     reason: "Permission denied".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("pi --version"));
    /// assert!(error.to_string().contains("Permission denied"));
    /// ```
    #[error("command execution failed: '{command}' - {reason}")]
    ExecutionFailed {
        /// The command that failed
        command: String,
        /// Reason for the failure
        reason: String,
    },
}

/// Errors that can occur during Pi-Mono command execution.
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// Command execution exceeded the configured timeout.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ExecutionError;
    ///
    /// let error = ExecutionError::Timeout {
    ///     command: "long-running-task".to_string(),
    ///     timeout_secs: 300,
    /// };
    ///
    /// assert!(error.to_string().contains("long-running-task"));
    /// assert!(error.to_string().contains("300"));
    /// ```
    #[error("command '{command}' timed out after {timeout_secs} seconds")]
    Timeout {
        /// The command that timed out
        command: String,
        /// The timeout duration in seconds
        timeout_secs: u64,
    },

    /// Command execution failed after all retries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ExecutionError;
    ///
    /// let error = ExecutionError::RetryExhausted {
    ///     command: "failing-task".to_string(),
    ///     attempts: 3,
    ///     last_error: "Connection refused".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("failing-task"));
    /// assert!(error.to_string().contains("3"));
    /// ```
    #[error("command '{command}' failed after {attempts} attempts: {last_error}")]
    RetryExhausted {
        /// The command that failed
        command: String,
        /// Number of retry attempts made
        attempts: usize,
        /// The error from the last attempt
        last_error: String,
    },

    /// The command returned a non-zero exit code.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ExecutionError;
    ///
    /// let error = ExecutionError::NonZeroExit {
    ///     command: "broken-task".to_string(),
    ///     exit_code: 1,
    ///     stderr: Some("Error: invalid input".to_string()),
    /// };
    ///
    /// assert!(error.to_string().contains("broken-task"));
    /// assert!(error.to_string().contains("exited with code 1"));
    /// ```
    #[error("command '{command}' exited with code {exit_code}")]
    NonZeroExit {
        /// The command that failed
        command: String,
        /// The exit code returned
        exit_code: i32,
        /// Standard error output, if available
        stderr: Option<String>,
    },

    /// Input validation failed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ExecutionError;
    ///
    /// let error = ExecutionError::Validation {
    ///     field: "task".to_string(),
    ///     message: "Task contains invalid characters".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("task"));
    /// assert!(error.to_string().contains("invalid characters"));
    /// ```
    #[error("validation failed for field '{field}': {message}")]
    Validation {
        /// The field that failed validation
        field: String,
        /// Validation error message
        message: String,
    },
}

/// Errors that can occur during configuration management.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// The specified configuration path is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ConfigError;
    ///
    /// let error = ConfigError::InvalidPath {
    ///     path: "/nonexistent/path".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("/nonexistent/path"));
    /// ```
    #[error("invalid configuration path: '{path}'")]
    InvalidPath {
        /// The invalid path
        path: String,
    },

    /// Failed to load configuration from the specified source.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ConfigError;
    ///
    /// let error = ConfigError::LoadFailed {
    ///     location: "config.json".to_string(),
    ///     reason: "permission denied".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("config.json"));
    /// assert!(error.to_string().contains("permission denied"));
    /// ```
    #[error("failed to load configuration from '{location}': {reason}")]
    LoadFailed {
        /// The configuration source that failed to load
        location: String,
        /// Human-readable reason for the failure
        reason: String,
    },

    /// The configuration is missing required fields.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::error::ConfigError;
    ///
    /// let error = ConfigError::MissingField {
    ///     field: "executable_path".to_string(),
    /// };
    ///
    /// assert!(error.to_string().contains("executable_path"));
    /// ```
    #[error("missing required configuration field: '{field}'")]
    MissingField {
        /// The missing field name
        field: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_execution() {
        let error = Error::Execution(ExecutionError::Timeout {
            command: "test".to_string(),
            timeout_secs: 60,
        });
        assert!(error.is_execution());
        assert!(!error.is_config());
    }

    #[test]
    fn test_error_is_config() {
        let error = Error::Config(ConfigError::InvalidPath {
            path: "/invalid".to_string(),
        });
        assert!(error.is_config());
        assert!(!error.is_execution());
    }

    #[test]
    fn test_execution_error_timeout_display() {
        let error = ExecutionError::Timeout {
            command: "my-command".to_string(),
            timeout_secs: 120,
        };
        let msg = error.to_string();
        assert!(msg.contains("my-command"));
        assert!(msg.contains("120"));
    }

    #[test]
    fn test_execution_error_retry_exhausted_display() {
        let error = ExecutionError::RetryExhausted {
            command: "failing-cmd".to_string(),
            attempts: 5,
            last_error: "Connection refused".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("failing-cmd"));
        assert!(msg.contains("5"));
        assert!(msg.contains("Connection refused"));
    }

    #[test]
    fn test_execution_error_non_zero_exit_display() {
        let error = ExecutionError::NonZeroExit {
            command: "broken".to_string(),
            exit_code: 1,
            stderr: Some("error details".to_string()),
        };
        let msg = error.to_string();
        assert!(msg.contains("broken"));
        assert!(msg.contains("1"));
    }

    #[test]
    fn test_config_error_invalid_path_display() {
        let error = ConfigError::InvalidPath {
            path: "/bad/path".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("/bad/path"));
    }

    #[test]
    fn test_config_error_load_failed_display() {
        let error = ConfigError::LoadFailed {
            location: "config.yaml".to_string(),
            reason: "file not found".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("config.yaml"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_config_error_missing_field_display() {
        let error = ConfigError::MissingField {
            field: "api_key".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("api_key"));
    }

    // DetectionError tests
    #[test]
    fn test_detection_error_not_found_display() {
        let error = DetectionError::NotFound;
        let msg = error.to_string();
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_detection_error_version_parse_failed_display() {
        let error = DetectionError::VersionParseFailed {
            output: "invalid version".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("invalid version"));
    }

    #[test]
    fn test_detection_error_execution_failed_display() {
        let error = DetectionError::ExecutionFailed {
            command: "pi --version".to_string(),
            reason: "Permission denied".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("pi --version"));
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn test_error_is_detection() {
        let error = Error::Detection(DetectionError::NotFound);
        assert!(error.is_detection());
        assert!(!error.is_execution());
        assert!(!error.is_config());
    }

    #[test]
    fn test_error_detection_conversion() {
        let detection_error = DetectionError::NotFound;
        let error: Error = detection_error.into();
        assert!(error.is_detection());
    }

    #[test]
    fn test_error_detection_from_execution_failed() {
        let detection_error = DetectionError::ExecutionFailed {
            command: "pi --version".to_string(),
            reason: "Command not found".to_string(),
        };
        let error: Error = detection_error.into();
        assert!(error.is_detection());
        assert!(error.to_string().contains("pi --version"));
    }
}
