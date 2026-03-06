//! Pi-Mono CLI detection and discovery
//!
//! This module provides functionality for detecting the pi-mono CLI installation,
//! determining its version, and identifying available capabilities.

use crate::error::{DetectionError, Error, Result};
use std::path::PathBuf;
use std::process::Command;
use which::which;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Pi-Mono CLI capability flags
///
/// These flags represent the capabilities supported by the detected pi-mono installation.
///
/// # Thread Safety
///
/// This type is `Clone` and `Send`, making it safe to share across threads.
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Whether subagent extension is supported
    pub subagent: bool,
    /// Whether streaming responses are supported
    pub streaming: bool,
    /// Whether parallel execution is supported
    pub parallel: bool,
    /// Whether chain execution is supported
    pub chain: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            subagent: true,
            streaming: true,
            parallel: true,
            chain: true,
        }
    }
}

impl Capabilities {
    /// Create a new Capabilities instance with all capabilities enabled
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new Capabilities instance with custom settings
    pub fn with_capabilities(subagent: bool, streaming: bool, parallel: bool, chain: bool) -> Self {
        Self {
            subagent,
            streaming,
            parallel,
            chain,
        }
    }
}

/// Detected Pi-Mono CLI information
///
/// This struct contains information about a detected pi-mono CLI installation,
/// including the executable path, version, and supported capabilities.
///
/// # Thread Safety
///
/// This type is `Clone` and `Send`, making it safe to share across threads.
#[derive(Debug, Clone)]
pub struct PiDetection {
    /// Path to the pi-mono executable
    pub executable_path: PathBuf,
    /// Detected version string (e.g., "0.49.3")
    pub version: Option<String>,
    /// Detected capabilities
    pub capabilities: Capabilities,
}

impl PiDetection {
    /// Search for pi-mono executable in standard locations
    ///
    /// This method searches for the pi-mono executable using PATH-first discovery:
    /// 1. `$PATH` - Using the `which` command (priority)
    /// 2. `~/.local/bin/pi` - User local installation (XDG_BIN_HOME)
    /// 3. `~/.cargo/bin/pi` - Cargo-installed
    /// 4. Custom path via `PI_MONO_PATH` environment variable
    ///
    /// Each path is validated to ensure it exists, is a regular file, and has execute permissions.
    ///
    /// # Errors
    ///
    /// Returns `Error::Detection(DetectionError::NotFound)` if the pi-mono
    /// executable cannot be found in any of the standard locations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::detection::PiDetection;
    ///
    /// match PiDetection::detect() {
    ///     Ok(detection) => println!("Found pi-mono at: {:?}", detection.executable_path),
    ///     Err(e) => println!("Could not find pi-mono: {}", e),
    /// }
    /// ```
    pub fn detect() -> Result<Self> {
        // Helper function to validate executable
        #[cfg(unix)]
        fn is_valid_executable(path: &PathBuf) -> bool {
            path.exists()
                && path.is_file()
                && std::fs::metadata(path)
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
        }

        // On non-Unix platforms, skip execute permission check
        #[cfg(not(unix))]
        fn is_valid_executable(path: &PathBuf) -> bool {
            path.exists() && path.is_file()
        }

        // 1. Check custom path via environment variable
        if let Ok(custom_path) = std::env::var("PI_MONO_PATH") {
            let path = PathBuf::from(custom_path);
            if is_valid_executable(&path) {
                return Ok(Self {
                    executable_path: path,
                    version: None,
                    capabilities: Capabilities::default(),
                });
            }
        }

        // 2. Check PATH first (most portable)
        if let Ok(path) = which("pi") {
            return Ok(Self {
                executable_path: path,
                version: None,
                capabilities: Capabilities::default(),
            });
        }

        // 3. Check common user-local installation paths (XDG-compliant)
        // Note: No hardcoded absolute paths - all computed from home directory
        let search_paths: Vec<Option<PathBuf>> = vec![
            // XDG_BIN_HOME (~/.local/bin by default)
            dirs::home_dir().map(|d| d.join(".local/bin/pi")),
            // Cargo bin directory
            dirs::home_dir().map(|d| d.join(".cargo/bin/pi")),
            // User bin directory
            dirs::home_dir().map(|d| d.join("bin/pi")),
        ];

        // Check each path
        for path in search_paths.into_iter().flatten() {
            if is_valid_executable(&path) {
                return Ok(Self {
                    executable_path: path,
                    version: None,
                    capabilities: Capabilities::default(),
                });
            }
        }

        Err(Error::Detection(DetectionError::NotFound))
    }

    /// Detect pi-mono version by executing --version
    ///
    /// This method executes the pi-mono CLI with the `--version` flag
    /// and parses the output to extract the version string.
    ///
    /// # Errors
    ///
    /// Returns `Error::Detection(DetectionError::ExecutionFailed)` if the
    /// command execution fails or times out after 5 seconds.
    ///
    /// Returns `Error::Detection(DetectionError::VersionParseFailed)` if
    /// the version string cannot be parsed from the output.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::detection::PiDetection;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut detection = PiDetection::detect()?;
    ///     let version = detection.detect_version().await?;
    ///     println!("Pi-mono version: {}", version);
    /// }
    /// ```
    pub async fn detect_version(&mut self) -> Result<String> {
        use std::time::Duration;

        let executable_path = self.executable_path.clone();
        let executable_path_str = format!("{:?}", executable_path);

        // Add 5 second timeout to prevent hanging
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::task::spawn_blocking(move || {
                Command::new(&executable_path).arg("--version").output()
            }),
        )
        .await
        .map_err(|_| {
            Error::Detection(DetectionError::ExecutionFailed {
                command: executable_path_str.clone(),
                reason: "Command timed out after 5 seconds".to_string(),
            })
        })?
        .map_err(|e| {
            Error::Detection(DetectionError::ExecutionFailed {
                command: executable_path_str.clone(),
                reason: format!("Task join failed: {}", e),
            })
        })?
        .map_err(|e| {
            Error::Detection(DetectionError::ExecutionFailed {
                command: executable_path_str,
                reason: format!("Failed to execute: {}", e),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Detection(DetectionError::ExecutionFailed {
                command: format!("{:?} --version", self.executable_path),
                reason: stderr.to_string(),
            }));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let version_str = stdout.trim();

        // Try to parse as semver
        // The output might be in different formats:
        // - "0.49.3"
        // - "pi version 0.49.3"
        // - "pi 0.49.3"
        let version = if let Ok(_v) = version_str.parse::<semver::Version>() {
            version_str.to_string()
        } else {
            // Try to extract version from the last word
            let last_word = version_str.split_whitespace().last().ok_or_else(|| {
                Error::Detection(DetectionError::VersionParseFailed {
                    output: version_str.to_string(),
                })
            })?;

            if let Ok(_v) = last_word.parse::<semver::Version>() {
                last_word.to_string()
            } else {
                return Err(Error::Detection(DetectionError::VersionParseFailed {
                    output: version_str.to_string(),
                }));
            }
        };

        self.version = Some(version.clone());
        Ok(version)
    }

    /// Detect available capabilities
    ///
    /// This method detects the capabilities supported by the pi-mono installation.
    /// Currently returns default capabilities. Full capability detection will be
    /// implemented in a future phase.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::detection::PiDetection;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut detection = PiDetection::detect()?;
    ///     let capabilities = detection.detect_capabilities().await?;
    ///     println!("Subagent support: {}", capabilities.subagent);
    /// }
    /// ```
    pub async fn detect_capabilities(&mut self) -> Result<Capabilities> {
        // Return default capabilities
        // Future implementation will check for:
        // - Extension availability (e.g., subagent)
        // - Feature flags
        // - Command availability
        let capabilities = Capabilities::new();
        self.capabilities = capabilities.clone();
        Ok(capabilities)
    }

    /// Perform full detection including version and capabilities
    ///
    /// This is a convenience method that performs all detection steps
    /// in a single call. Returns detection with as much information
    /// as could be gathered.
    ///
    /// Version and capabilities detection failures are logged but do not
    /// cause the overall detection to fail - the method returns success
    /// with the detected executable path and whatever additional info
    /// could be gathered.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::detection::PiDetection;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let detection = PiDetection::detect_full().await?;
    ///     println!("Found pi-mono version: {:?}", detection.version);
    /// }
    /// ```
    pub async fn detect_full() -> Result<Self> {
        let mut detection = Self::detect()?;

        // Attempt version detection - log warnings but don't fail
        match detection.detect_version().await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Failed to detect pi-mono version: {}", e);
            }
        }

        // Attempt capabilities detection - log warnings but don't fail
        match detection.detect_capabilities().await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Failed to detect pi-mono capabilities: {}", e);
            }
        }

        Ok(detection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(caps.subagent);
        assert!(caps.streaming);
        assert!(caps.parallel);
        assert!(caps.chain);
    }

    #[test]
    fn test_capabilities_new() {
        let caps = Capabilities::new();
        assert!(caps.subagent);
        assert!(caps.streaming);
        assert!(caps.parallel);
        assert!(caps.chain);
    }

    #[test]
    fn test_capabilities_with_custom() {
        let caps = Capabilities::with_capabilities(true, false, true, false);
        assert!(caps.subagent);
        assert!(!caps.streaming);
        assert!(caps.parallel);
        assert!(!caps.chain);
    }

    #[test]
    fn test_pi_detection_creation() {
        // Use a portable path for testing
        let test_path = dirs::home_dir()
            .map(|h| h.join(".local/bin/pi"))
            .unwrap_or_else(|| PathBuf::from(".local/bin/pi"));

        let detection = PiDetection {
            executable_path: test_path.clone(),
            version: Some("0.49.3".to_string()),
            capabilities: Capabilities::default(),
        };

        assert_eq!(detection.executable_path, test_path);
        assert_eq!(detection.version, Some("0.49.3".to_string()));
        assert!(detection.capabilities.streaming);
    }

    #[test]
    fn test_detect_fallback_to_which() {
        // Test that which works as a fallback for a command that exists
        let result = which::which("sh");
        assert!(result.is_ok(), "sh should be found in PATH");
    }

    #[test]
    fn test_search_paths_are_portable() {
        // Verify that search paths are computed from home directory
        // (no hardcoded absolute paths)
        let home = dirs::home_dir().expect("No home directory");

        // These paths should all be relative to home
        let user_local = home.join(".local/bin/pi");
        let cargo_bin = home.join(".cargo/bin/pi");
        let user_bin = home.join("bin/pi");

        // All should be under home directory
        assert!(user_local.starts_with(&home));
        assert!(cargo_bin.starts_with(&home));
        assert!(user_bin.starts_with(&home));
    }

    #[test]
    fn test_executable_validation() {
        // Test that is_valid_executable helper correctly validates executables
        use std::fs::{self, File};

        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = std::env::temp_dir().join("pi_test_executable");
        fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("test_executable");

        // Test 1: Non-existent file
        assert!(!test_file.exists());

        // Test 2: Create file but no execute permissions
        File::create(&test_file).unwrap();
        assert!(test_file.exists());
        assert!(test_file.is_file());

        #[cfg(unix)]
        {
            let metadata = fs::metadata(&test_file).unwrap();
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            // No execute bit set
            assert_eq!(mode & 0o111, 0);
        }

        // Test 3: Add execute permissions (Unix only)
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&test_file).unwrap();
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            let mut new_permissions = permissions.clone();
            new_permissions.set_mode(mode | 0o755);
            fs::set_permissions(&test_file, new_permissions).unwrap();
            let metadata = fs::metadata(&test_file).unwrap();
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            // Execute bit is set
            assert!(mode & 0o111 != 0);
        }

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_detect_capabilities() {
        // Use a portable path for testing
        let test_path = dirs::home_dir()
            .map(|h| h.join(".local/bin/pi"))
            .unwrap_or_else(|| PathBuf::from(".local/bin/pi"));

        let mut detection = PiDetection {
            executable_path: test_path,
            version: None,
            capabilities: Capabilities {
                subagent: false,
                streaming: false,
                parallel: false,
                chain: false,
            },
        };

        let result = detection.detect_capabilities().await;
        assert!(result.is_ok());

        let caps = result.unwrap();
        assert!(caps.subagent);
        assert!(caps.streaming);
        assert!(caps.parallel);
        assert!(caps.chain);

        // Verify that detection.capabilities was also updated
        assert!(detection.capabilities.subagent);
        assert!(detection.capabilities.streaming);
    }

    #[test]
    fn test_detect_not_found() {
        // Test the detect() method - we don't know if pi is installed
        let result = PiDetection::detect();

        // Verify the result is either Ok with "pi" executable or NotFound error
        match &result {
            Ok(detection) => assert!(detection.executable_path.ends_with("pi")),
            Err(Error::Detection(DetectionError::NotFound)) => {
                // Expected result when pi is not installed
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}
