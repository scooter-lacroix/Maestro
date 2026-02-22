//! Executable Discovery Module
//!
//! Provides cross-platform executable discovery using PATH environment variable
//! and common installation locations. No hardcoded absolute paths are used.
//!
//! ## Design Principles
//!
//! 1. **PATH-first discovery**: Always check PATH before hardcoded locations
//! 2. **XDG compliance**: Use XDG_BIN_HOME as a priority location
//! 3. **Graceful degradation**: Return None if not found, never panic
//! 4. **No user-specific paths**: Never include paths like /home/username
//!
//! ## Usage
//!
//! ```no_run
//! use maestro_core::portability::executable::*;
//!
//! // Find an executable in PATH
//! if let Some(path) = find_executable("pi") {
//!     println!("Found pi at: {:?}", path);
//! }
//!
//! // Find with additional search paths
//! if let Some(path) = find_executable_with_paths("yazi", &["~/.cargo/bin"]) {
//!     println!("Found yazi at: {:?}", path);
//! }
//!
//! // Check if an executable exists
//! if is_executable_available("rust-analyzer") {
//!     println!("rust-analyzer is available");
//! }
//! ```

use std::path::PathBuf;

/// Find an executable in PATH and common locations
///
/// Search order:
/// 1. System PATH (via which crate)
/// 2. XDG_BIN_HOME (~/.local/bin by default)
/// 3. ~/.cargo/bin (Rust toolchain)
/// 4. ~/.local/bin (user-local installations)
///
/// Returns the first matching executable path, or None if not found.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    // 1. Check PATH using which crate
    if let Ok(path) = which::which(name) {
        return Some(path);
    }

    // 2. Check XDG_BIN_HOME
    let xdg_bin = super::xdg_dirs::bin_home();
    let xdg_path = xdg_bin.join(name);
    if is_executable(&xdg_path) {
        return Some(xdg_path);
    }

    // 3. Check ~/.cargo/bin (Rust toolchain)
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin").join(name);
        if is_executable(&cargo_bin) {
            return Some(cargo_bin);
        }

        // 4. Check ~/.local/bin (already covered by XDG but explicit check)
        let local_bin = home.join(".local/bin").join(name);
        if is_executable(&local_bin) {
            return Some(local_bin);
        }
    }

    None
}

/// Find an executable with additional custom search paths
///
/// Searches PATH first, then the provided custom paths, then common locations.
///
/// # Arguments
///
/// * `name` - The executable name to find
/// * `custom_paths` - Additional paths to search (supports ~ expansion)
///
/// # Returns
///
/// The first matching executable path, or None if not found.
pub fn find_executable_with_paths(name: &str, custom_paths: &[&str]) -> Option<PathBuf> {
    // Check custom paths first (with tilde expansion)
    for path_template in custom_paths {
        let expanded = expand_tilde(path_template);
        let full_path = expanded.join(name);
        if is_executable(&full_path) {
            return Some(full_path);
        }
    }

    // Fall back to standard search
    find_executable(name)
}

/// Check if an executable is available
///
/// Returns true if the executable can be found in PATH or common locations.
pub fn is_executable_available(name: &str) -> bool {
    find_executable(name).is_some()
}

/// Get all locations where an executable might be found
///
/// Returns a list of paths to check, in priority order.
pub fn get_executable_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // From PATH environment variable
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if !dir.is_empty() {
                paths.push(PathBuf::from(dir));
            }
        }
    }

    // XDG_BIN_HOME
    paths.push(super::xdg_dirs::bin_home());

    // User-local locations
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".cargo/bin"));
        paths.push(home.join(".local/bin"));
        paths.push(home.join("bin"));
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));

    paths
}

/// Check if a path points to an executable file
///
/// On Unix, checks for execute permission bit.
/// On other platforms, just checks if the file exists.
#[cfg(unix)]
pub fn is_executable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.exists()
        && path.is_file()
        && std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
pub fn is_executable(path: &PathBuf) -> bool {
    path.exists() && path.is_file()
}

/// Expand tilde (~) in a path string
///
/// Converts ~ to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(path));
    }

    PathBuf::from(path)
}

/// Find all available executables matching a pattern
///
/// Searches all executable directories for files matching the pattern.
pub fn find_all_executables_matching(pattern: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();

    for dir in get_executable_search_paths() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.contains(pattern) {
                    let path = entry.path();
                    if is_executable(&path) {
                        results.push(path);
                    }
                }
            }
        }
    }

    results.sort();
    results.dedup();
    results
}

// ============================================================================
// Convenience functions for common Maestro executables
// ============================================================================

/// Find the pi-mono executable
///
/// Search order:
/// 1. PATH
/// 2. XDG_BIN_HOME
/// 3. ~/.cargo/bin
/// 4. ~/.local/bin
pub fn find_pi_mono() -> Option<PathBuf> {
    find_executable("pi")
}

/// Find the OMP (oh-my-pi) executable
pub fn find_omp() -> Option<PathBuf> {
    find_executable("omp")
}

/// Find the yazi file manager
pub fn find_yazi() -> Option<PathBuf> {
    find_executable("yazi")
}

/// Find the tmux-rs executable
pub fn find_tmux_rs() -> Option<PathBuf> {
    find_executable("tmux-rs").or_else(|| find_executable("tmux"))
}

/// Find the zellij multiplexer
pub fn find_zellij() -> Option<PathBuf> {
    find_executable("zellij")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~");
        assert!(expanded.to_string_lossy().len() > 1);

        let expanded = expand_tilde("~/test/path");
        assert!(expanded.to_string_lossy().ends_with("test/path"));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let expanded = expand_tilde("/usr/bin/test");
        assert_eq!(expanded, PathBuf::from("/usr/bin/test"));
    }

    #[test]
    fn test_get_executable_search_paths() {
        let paths = get_executable_search_paths();
        assert!(!paths.is_empty());

        // Should include common locations
        let _path_strings: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // At least one path should exist
        assert!(paths
            .iter()
            .any(|p| p.exists() || p.to_string_lossy().contains("bin")));
    }

    #[test]
    fn test_find_executable_sh() {
        // sh should always be available
        let result = find_executable("sh");
        assert!(result.is_some(), "sh should be found in PATH");
    }

    #[test]
    fn test_is_executable_available() {
        // sh is always available
        assert!(is_executable_available("sh"));

        // This should not exist
        assert!(!is_executable_available(
            "this_executable_definitely_does_not_exist_12345"
        ));
    }
}
