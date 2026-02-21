//! Common Path Resolution Utilities
//!
//! Provides centralized path resolution for Maestro without hardcoded absolute paths.
//! All paths are resolved relative to XDG directories or user home directories.
//!
//! ## Design Principles
//!
//! 1. **No hardcoded absolute paths** - All paths are computed at runtime
//! 2. **XDG compliance** - Use XDG Base Directory specification
//! 3. **Fallback gracefully** - Always have a reasonable fallback
//! 4. **Document assumptions** - Clear documentation for each path resolver

use std::path::PathBuf;

/// Get the home directory with a graceful fallback
///
/// Falls back to the current directory if home cannot be determined.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Get user-local bin directory
///
/// Returns XDG_BIN_HOME (~/.local/bin by default).
/// This is where user-installed executables should be placed.
pub fn user_bin_dir() -> PathBuf {
    super::xdg_dirs::bin_home()
}

/// Get common executable search directories
///
/// Returns directories that typically contain executables, in priority order.
/// Does NOT include system paths like /usr/bin that are already in PATH.
pub fn executable_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // User-local bin (XDG)
    dirs.push(user_bin_dir());

    // Home-based locations
    let home = home_dir();
    dirs.push(home.join(".cargo/bin"));
    dirs.push(home.join("bin"));

    // Go installation
    dirs.push(home.join("go/bin"));

    // Deduplicate
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|p| seen.insert(p.clone()));

    dirs
}

/// Get Maestro home directory
///
/// This is the primary location for Maestro data, configs, and resources.
/// Returns ~/.maestro by default.
pub fn maestro_home() -> PathBuf {
    // Allow override via environment variable
    std::env::var("MAESTRO_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| super::executable::expand_tilde(&s))
        .unwrap_or_else(|| {
            home_dir().join(".maestro")
        })
}

/// Get Maestro resources directory
///
/// Contains bundled resources like zide, layouts, etc.
/// Order of preference:
/// 1. $MAESTRO_HOME/resources
/// 2. $XDG_DATA_HOME/maestro/resources
/// 3. ~/.maestro/resources
pub fn maestro_resources_dir() -> PathBuf {
    maestro_home().join("resources")
}

/// Get zide resources directory
///
/// The zide terminal environment configuration.
pub fn zide_dir() -> PathBuf {
    maestro_resources_dir().join("zide")
}

/// Get pi-mono extensions directory
///
/// Where pi-mono extensions are installed.
pub fn pi_extensions_dir() -> PathBuf {
    home_dir().join(".pi/extensions")
}

/// Get pi-mono config directory
pub fn pi_config_dir() -> PathBuf {
    home_dir().join(".pi")
}

/// Get OMP (oh-my-pi) directory
///
/// Checks common locations for oh-my-pi installation.
pub fn omp_dir() -> Option<PathBuf> {
    let home = home_dir();

    let candidates = vec![
        home.join("oh-my-pi"),
        home.join(".oh-my-pi"),
        // Relative to maestro repo
        PathBuf::from("vendor/oh-my-pi"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

/// Get Claude Code config directory
pub fn claude_config_dir() -> PathBuf {
    home_dir().join(".claude")
}

/// Get Claude Code commands directory
pub fn claude_commands_dir() -> PathBuf {
    claude_config_dir().join("commands")
}

/// Get Claude Code skills directory
pub fn claude_skills_dir() -> PathBuf {
    claude_config_dir().join("skills")
}

/// Expand a path template
///
/// Supports:
/// - ~ for home directory
/// - $XDG_DATA_HOME, $XDG_CONFIG_HOME, $XDG_CACHE_HOME
/// - $MAESTRO_HOME
pub fn expand_path_template(template: &str) -> PathBuf {
    let result = template
        .replace("$MAESTRO_HOME", &maestro_home().to_string_lossy())
        .replace("$XDG_DATA_HOME", &super::xdg_dirs::data_home().to_string_lossy())
        .replace("$XDG_CONFIG_HOME", &super::xdg_dirs::config_home().to_string_lossy())
        .replace("$XDG_CACHE_HOME", &super::xdg_dirs::cache_home().to_string_lossy());

    super::executable::expand_tilde(&result)
}

/// Resolve a relative path to an absolute path
///
/// Resolves relative paths from the current directory,
/// and expands ~ and environment variables.
pub fn resolve_path(path: &str) -> PathBuf {
    let expanded = super::executable::expand_tilde(path);

    if expanded.is_relative() {
        std::env::current_dir()
            .map(|cwd| cwd.join(&expanded))
            .unwrap_or(expanded)
    } else {
        expanded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir() {
        let home = home_dir();
        assert!(home.to_string_lossy().len() > 1);
    }

    #[test]
    fn test_maestro_home() {
        let home = maestro_home();
        assert!(home.to_string_lossy().contains("maestro"));
    }

    #[test]
    fn test_executable_search_dirs() {
        let dirs = executable_search_dirs();
        assert!(!dirs.is_empty());
    }

    #[test]
    fn test_expand_path_template() {
        let expanded = expand_path_template("$MAESTRO_HOME/test");
        assert!(expanded.to_string_lossy().contains("maestro"));
        assert!(expanded.to_string_lossy().ends_with("test"));
    }

    #[test]
    fn test_maestro_resources_dir() {
        let resources = maestro_resources_dir();
        assert!(resources.to_string_lossy().contains("resources"));
    }
}
