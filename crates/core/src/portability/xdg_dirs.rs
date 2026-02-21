//! XDG Base Directory Specification Implementation
//!
//! Provides XDG-compliant path resolution for user data, config, cache, and
//! runtime directories. Falls back gracefully when XDG environment variables
//! are not set.
//!
//! ## XDG Specification
//!
//! - XDG_DATA_HOME: ~/.local/share (default)
//! - XDG_CONFIG_HOME: ~/.config (default)
//! - XDG_CACHE_HOME: ~/.cache (default)
//! - XDG_STATE_HOME: ~/.local/state (default)
//! - XDG_RUNTIME_DIR: /run/user/$UID (typically set by pam_systemd)
//!
//! ## Usage
//!
//! ```no_run
//! use maestro_core::portability::xdg_dirs::*;
//!
//! // Get Maestro-specific directories
//! let maestro_data = maestro_data_dir();
//! let maestro_config = maestro_config_dir();
//! let maestro_cache = maestro_cache_dir();
//!
//! println!("Data: {:?}", maestro_data);
//! println!("Config: {:?}", maestro_config);
//! println!("Cache: {:?}", maestro_cache);
//! ```

use std::path::PathBuf;

/// Get XDG_DATA_HOME or default
///
/// Returns $XDG_DATA_HOME if set, otherwise ~/.local/share
pub fn data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/share"))
                .unwrap_or_else(|| PathBuf::from(".local/share"))
        })
}

/// Get XDG_CONFIG_HOME or default
///
/// Returns $XDG_CONFIG_HOME if set, otherwise ~/.config
pub fn config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".config"))
                .unwrap_or_else(|| PathBuf::from(".config"))
        })
}

/// Get XDG_CACHE_HOME or default
///
/// Returns $XDG_CACHE_HOME if set, otherwise ~/.cache
pub fn cache_home() -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".cache"))
                .unwrap_or_else(|| PathBuf::from(".cache"))
        })
}

/// Get XDG_STATE_HOME or default
///
/// Returns $XDG_STATE_HOME if set, otherwise ~/.local/state
pub fn state_home() -> PathBuf {
    std::env::var("XDG_STATE_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/state"))
                .unwrap_or_else(|| PathBuf::from(".local/state"))
        })
}

/// Get XDG_RUNTIME_DIR or fallback
///
/// Returns $XDG_RUNTIME_DIR if set, otherwise uses a temporary directory.
/// The fallback is less secure but ensures the application works.
pub fn runtime_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Fallback to /tmp/user/$UID or just /tmp
            let uid = unsafe { libc::getuid() };
            let user_runtime = PathBuf::from(format!("/tmp/user/{}", uid));
            if user_runtime.exists() {
                user_runtime
            } else {
                std::env::temp_dir()
            }
        })
}

/// Get XDG_BIN_HOME or default
///
/// Returns $XDG_BIN_HOME if set, otherwise ~/.local/bin
/// This is a newer XDG specification addition.
pub fn bin_home() -> PathBuf {
    std::env::var("XDG_BIN_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/bin"))
                .unwrap_or_else(|| PathBuf::from(".local/bin"))
        })
}

// ============================================================================
// Maestro-specific directory helpers
// ============================================================================

/// Get Maestro data directory
///
/// Returns $XDG_DATA_HOME/maestro or ~/.local/share/maestro
pub fn maestro_data_dir() -> PathBuf {
    data_home().join("maestro")
}

/// Get Maestro config directory
///
/// Returns $XDG_CONFIG_HOME/maestro or ~/.config/maestro
pub fn maestro_config_dir() -> PathBuf {
    config_home().join("maestro")
}

/// Get Maestro cache directory
///
/// Returns $XDG_CACHE_HOME/maestro or ~/.cache/maestro
pub fn maestro_cache_dir() -> PathBuf {
    cache_home().join("maestro")
}

/// Get Maestro state directory
///
/// Returns $XDG_STATE_HOME/maestro or ~/.local/state/maestro
pub fn maestro_state_dir() -> PathBuf {
    state_home().join("maestro")
}

/// Get LeIndex data directory
///
/// Returns $XDG_DATA_HOME/leindex or ~/.local/share/leindex
pub fn leindex_data_dir() -> PathBuf {
    data_home().join("leindex")
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir(dir: &PathBuf) -> std::io::Result<()> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_home_fallback() {
        // When XDG_DATA_HOME is not set, should use ~/.local/share
        let path = data_home();
        assert!(path.ends_with(".local/share") || path.to_string_lossy().contains("XDG_DATA_HOME"));
    }

    #[test]
    fn test_config_home_fallback() {
        let path = config_home();
        assert!(path.ends_with(".config") || path.to_string_lossy().contains("XDG_CONFIG_HOME"));
    }

    #[test]
    fn test_cache_home_fallback() {
        let path = cache_home();
        assert!(path.ends_with(".cache") || path.to_string_lossy().contains("XDG_CACHE_HOME"));
    }

    #[test]
    fn test_maestro_data_dir() {
        let path = maestro_data_dir();
        assert!(path.ends_with("maestro"));
        assert!(path.to_string_lossy().contains("share") || path.to_string_lossy().contains("XDG_DATA_HOME"));
    }

    #[test]
    fn test_bin_home() {
        let path = bin_home();
        assert!(path.ends_with(".local/bin") || path.to_string_lossy().contains("XDG_BIN_HOME"));
    }
}
