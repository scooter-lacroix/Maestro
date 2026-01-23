//! # Configuration module for Pi-Mono integration
//!
//! This module handles configuration loading and management for Pi-Mono agents.
//!
//! ## Example
//!
//! ```rust
//! use maestro_pi_mono::config::PiMonoConfig;
//! use std::collections::HashMap;
//!
//! // Create a default configuration
//! let config = PiMonoConfig::default();
//!
//! // Or create with custom settings using the builder pattern
//! let mut config = PiMonoConfig::default();
//! config.executable_path = Some("/path/to/pi-mono".to_string());
//! config.work_dir = Some("/workspace".to_string());
//! ```

use serde::{Deserialize, Serialize};

/// Pi-Mono configuration structure.
///
/// # Examples
///
/// Creating a default configuration:
///
/// ```rust
/// use maestro_pi_mono::config::PiMonoConfig;
///
/// let config = PiMonoConfig::default();
/// assert!(config.executable_path.is_none());
/// assert!(config.work_dir.is_none());
/// ```
///
/// Creating a configuration with custom settings:
///
/// ```rust
/// use maestro_pi_mono::config::PiMonoConfig;
/// use std::collections::HashMap;
///
/// let mut config = PiMonoConfig::default();
/// config.executable_path = Some("/usr/bin/pi-mono".to_string());
/// config.work_dir = Some("/tmp/workspace".to_string());
/// config.env_vars.insert("DEBUG".to_string(), "1".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiMonoConfig {
    /// Path to the pi-mono executable
    pub executable_path: Option<String>,

    /// Working directory for pi-mono operations
    pub work_dir: Option<String>,

    /// Additional environment variables
    pub env_vars: std::collections::HashMap<String, String>,
}

impl Default for PiMonoConfig {
    /// Creates a default `PiMonoConfig` with:
    /// - No executable path set
    /// - No working directory set
    /// - Empty environment variables map
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::config::PiMonoConfig;
    ///
    /// let config = PiMonoConfig::default();
    /// assert!(config.executable_path.is_none());
    /// assert!(config.work_dir.is_none());
    /// assert!(config.env_vars.is_empty());
    /// ```
    fn default() -> Self {
        Self {
            executable_path: None,
            work_dir: None,
            env_vars: std::collections::HashMap::new(),
        }
    }
}
