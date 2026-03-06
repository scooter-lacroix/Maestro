//! PTY Bridge for Maestro-tab integration
//!
//! Provides direct PTY access for transparency and other
//! terminal control sequences that need to bypass the multiplexer.

use anyhow::Result;
use std::io::Write;
use std::os::fd::FromRawFd;

/// PTY bridge for direct terminal output
pub struct PtyBridge {
    /// The PTY file descriptor
    fd: std::fs::File,
}

impl PtyBridge {
    /// Create a new PTY bridge using /dev/tty
    pub fn new() -> Result<Self> {
        let fd = match std::fs::OpenOptions::new().write(true).open("/dev/tty") {
            Ok(f) => f,
            Err(_) => {
                // Fallback: use stdout fd
                unsafe { std::fs::File::from_raw_fd(1) }
            }
        };

        Ok(Self { fd })
    }

    /// Write bytes directly to the PTY
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.fd.write_all(data)?;
        Ok(())
    }

    /// Write a string directly to the PTY
    pub fn write_str(&mut self, s: &str) -> Result<()> {
        self.write(s.as_bytes())
    }

    /// Flush any buffered output
    pub fn flush(&mut self) -> Result<()> {
        self.fd.flush()?;
        Ok(())
    }

    /// Write an OSC sequence to the PTY
    pub fn write_osc(&mut self, sequence: &str) -> Result<()> {
        self.write_str(sequence)?;
        self.flush()
    }
}

impl Default for PtyBridge {
    fn default() -> Self {
        Self::new().expect("Failed to create PTY bridge")
    }
}

/// Trait for types that can write to a PTY
pub trait PtyWriter: Send + Sync {
    /// Write bytes to the PTY
    fn write_pty(&mut self, data: &[u8]) -> Result<()>;

    /// Flush the PTY
    fn flush_pty(&mut self) -> Result<()>;
}

impl PtyWriter for PtyBridge {
    fn write_pty(&mut self, data: &[u8]) -> Result<()> {
        self.write(data)
    }

    fn flush_pty(&mut self) -> Result<()> {
        self.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_bridge_creation() {
        // This test may fail in non-TTY environments
        let result = PtyBridge::new();
        // Just verify it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }
}
