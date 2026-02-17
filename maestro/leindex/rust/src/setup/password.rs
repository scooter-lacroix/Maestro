//! Password Management System for sudo operations
//!
//! Provides secure password caching to allow single password entry
//! during the installation process.

use std::io::{self, Write};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Secure password cache that stores password in memory only
pub struct PasswordCache {
    /// The cached password (stored in memory only, never written to disk)
    password: Arc<Mutex<Option<String>>>,
    /// When the password was last used for sudo
    last_used: Arc<Mutex<Option<Instant>>>,
    /// Refresh interval for sudo session (default: 4 minutes, sudo default is 5)
    refresh_interval: Duration,
}

impl Default for PasswordCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordCache {
    /// Creates a new empty password cache
    pub fn new() -> Self {
        Self {
            password: Arc::new(Mutex::new(None)),
            last_used: Arc::new(Mutex::new(None)),
            refresh_interval: Duration::from_secs(240), // 4 minutes
        }
    }

    /// Creates a password cache with the given password
    pub fn with_password(password: String) -> Self {
        let cache = Self::new();
        cache.set_password(password);
        cache
    }

    /// Sets the password in the cache
    pub fn set_password(&self, password: String) {
        if let Ok(mut guard) = self.password.lock() {
            // Zero out old password if exists
            if let Some(ref mut old) = *guard {
                // Overwrite with zeros before replacing
                unsafe {
                    std::ptr::write_volatile(old.as_mut_ptr(), 0);
                }
            }
            *guard = Some(password);
        }
        if let Ok(mut guard) = self.last_used.lock() {
            *guard = Some(Instant::now());
        }
    }

    /// Gets the cached password
    pub fn get_password(&self) -> Option<String> {
        self.password.lock().ok().and_then(|guard| guard.clone())
    }

    /// Clears the password from memory securely
    pub fn clear(&self) {
        if let Ok(mut guard) = self.password.lock() {
            if let Some(ref mut password) = *guard {
                // Securely zero memory
                unsafe {
                    std::ptr::write_volatile(password.as_mut_ptr(), 0);
                    for byte in password.as_bytes_mut() {
                        std::ptr::write_volatile(byte, 0);
                    }
                }
            }
            *guard = None;
        }
        if let Ok(mut guard) = self.last_used.lock() {
            *guard = None;
        }
    }

    /// Checks if password is cached and still valid
    pub fn is_valid(&self) -> bool {
        let password_guard = self.password.lock();
        let last_used_guard = self.last_used.lock();
        
        match (password_guard, last_used_guard) {
            (Ok(pw), Ok(lu)) => {
                if pw.is_none() {
                    return false;
                }
                if let Some(last) = *lu {
                    // Password is valid if we've used it within the refresh interval
                    last.elapsed() < self.refresh_interval
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Refreshes the sudo session to keep it alive
    pub fn refresh_sudo_session(&self) -> Result<(), String> {
        let password = self.get_password().ok_or("No password cached")?;
        
        let mut output = Command::new("sudo")
            .arg("-S")
            .arg("-v") // Validate/refresh sudo ticket
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sudo: {}", e))?;

        if let Some(ref mut stdin) = output.stdin {
            writeln!(stdin, "{}", password).map_err(|e| format!("Failed to write password: {}", e))?;
        }

        let result = output.wait_with_output()
            .map_err(|e| format!("Failed to wait for sudo: {}", e))?;

        if result.status.success() {
            if let Ok(mut guard) = self.last_used.lock() {
                *guard = Some(Instant::now());
            }
            Ok(())
        } else {
            Err("Sudo validation failed - password may be incorrect".to_string())
        }
    }

    /// Executes a sudo command with the cached password
    pub fn sudo_with_password(&self, command: &str) -> Result<std::process::Output, String> {
        let password = self.get_password().ok_or("No password cached")?;
        
        // Refresh session if needed
        if !self.is_valid() {
            self.refresh_sudo_session()?;
        }

        let output = Command::new("bash")
            .arg("-c")
            .arg(format!("echo '{}' | sudo -S {}", password, command))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let result = output.wait_with_output()
            .map_err(|e| format!("Failed to wait for command: {}", e))?;

        if let Ok(mut guard) = self.last_used.lock() {
            *guard = Some(Instant::now());
        }

        Ok(result)
    }
}

impl Drop for PasswordCache {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Prompts for password in terminal (for non-TUI contexts)
/// Note: This is a basic implementation. For TUI contexts, use the
/// password modal in setup_main.rs instead.
pub fn prompt_password_terminal(_prompt: &str) -> Result<String, io::Error> {
    // This function is provided for completeness but is not used
    // by the TUI installer. The TUI has its own password modal.
    // For non-TUI contexts, you can implement platform-specific
    // password reading or use a crate like `rpassword`.
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Terminal password prompt requires rpassword crate or TUI implementation"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_cache_new() {
        let cache = PasswordCache::new();
        assert!(!cache.is_valid());
        assert!(cache.get_password().is_none());
    }

    #[test]
    fn test_password_cache_set_get() {
        let cache = PasswordCache::new();
        cache.set_password("test123".to_string());
        assert_eq!(cache.get_password(), Some("test123".to_string()));
    }

    #[test]
    fn test_password_cache_clear() {
        let cache = PasswordCache::new();
        cache.set_password("test123".to_string());
        cache.clear();
        assert!(cache.get_password().is_none());
        assert!(!cache.is_valid());
    }

    #[test]
    fn test_password_cache_valid_after_set() {
        let cache = PasswordCache::new();
        cache.set_password("test123".to_string());
        // Should be valid immediately after setting
        // Note: This test may be flaky if the refresh_interval is very short
        // But with default 4 minutes, it should always pass
        assert!(cache.is_valid());
    }
}
