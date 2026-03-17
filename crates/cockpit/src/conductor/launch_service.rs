//! Launch service for executing orchestrate commands
//!
//! Provides safe command execution with verification and timeout handling.

use std::process::{Command, Child};
use std::thread;
use std::time::Duration;
use super::pane::CommandArgs;

/// Result of a launch operation
#[derive(Debug, Clone)]
pub enum LaunchResult {
    Success { track_id: String, pid: u32 },
    SpawnFailed { track_id: String, error: String },
    VerificationFailed { track_id: String, reason: String },
    Timeout { track_id: String, timeout_secs: u64 },
}

/// Request to launch an orchestrate command
#[derive(Debug, Clone)]
pub struct LaunchRequest{
    track_id: String,
    command: CommandArgs,
}

impl LaunchRequest{
    pub fn new(track_id: impl Into<String>, command: CommandArgs) -> Self{
        Self{
            track_id: track_id.into(),
            command,
        }
    }
}

/// Service for launching orchestrate commands with timeout support
pub struct LaunchService{
    timeout_secs: u64,
}

impl LaunchService{
    /// Create a new LaunchService with default 30-second timeout
    pub fn new() -> Result<Self, String> {
        Ok(Self { timeout_secs: 30 })
    }

    /// Create a new LaunchService with a custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }

    /// Get the configured timeout
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// Launch a command with timeout handling
    pub fn launch(&self, request: LaunchRequest) -> LaunchResult {
        let mut cmd = Command::new(request.command.program());
        cmd.args(request.command.args());

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                let timeout = self.timeout();

                // Wait for the process with timeout
                match self.wait_with_timeout(&mut child, timeout) {
                    Ok(_) => LaunchResult::Success {
                        track_id: request.track_id,
                        pid,
                    },
                    Err(_) => {
                        // Kill the process if it timed out
                        let _ = child.kill();
                        LaunchResult::Timeout {
                            track_id: request.track_id,
                            timeout_secs: self.timeout_secs,
                        }
                    }
                }
            }
            Err(e) => LaunchResult::SpawnFailed {
                track_id: request.track_id,
                error: e.to_string(),
            },
        }
    }

    /// Wait for a child process with a timeout
    fn wait_with_timeout(&self, child: &mut Child, timeout: Duration) -> Result<(), ()> {
        let start = std::time::Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()), // Process exited
                Ok(None) => {
                    // Process still running, check timeout
                    if start.elapsed() >= timeout {
                        return Err(()); // Timeout
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(_) => return Err(()), // Error checking status
            }
        }
    }

    /// Launch a command asynchronously (non-blocking) with timeout monitoring
    pub fn launch_async(&self, request: LaunchRequest) -> Result<LaunchHandle, String> {
        let mut cmd = Command::new(request.command.program());
        cmd.args(request.command.args());

        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        Ok(LaunchHandle {
            track_id: request.track_id,
            child,
            timeout: self.timeout(),
            started_at: std::time::Instant::now(),
        })
    }
}

impl Default for LaunchService {
    fn default() -> Self {
        Self::new().expect("Failed to create LaunchService")
    }
}

/// Handle for an asynchronous launch operation
pub struct LaunchHandle {
    track_id: String,
    child: Child,
    timeout: Duration,
    started_at: std::time::Instant,
}

impl LaunchHandle {
    /// Get the track ID
    pub fn track_id(&self) -> &str {
        &self.track_id
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Check if the process has exceeded its timeout
    pub fn is_timed_out(&self) -> bool {
        self.started_at.elapsed() >= self.timeout
    }

    /// Check if the process has completed
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        self.child.try_wait()
    }

    /// Wait for the process to complete (blocking)
    pub fn wait(&mut self) -> Result<std::process::ExitStatus, std::io::Error> {
        self.child.wait()
    }

    /// Kill the process
    pub fn kill(&mut self) -> Result<(), std::io::Error> {
        self.child.kill()
    }

    /// Get elapsed time since launch
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get remaining time before timeout
    pub fn time_remaining(&self) -> Duration {
        let elapsed = self.elapsed();
        if elapsed >= self.timeout {
            Duration::from_secs(0)
        } else {
            self.timeout - elapsed
        }
    }
}
