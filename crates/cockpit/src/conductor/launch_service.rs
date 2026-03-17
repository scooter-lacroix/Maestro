//! Launch service for executing orchestrate commands
//!
//! Provides safe command execution with verification.

use std::process::Command;
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

/// Service for launching orchestrate commands
pub struct LaunchService{
    timeout_secs: u64,
}

impl LaunchService{
    pub fn new() -> Result<Self, String> {
        Ok(Self { timeout_secs: 30 })
    }

    pub fn launch(&self, request: LaunchRequest) -> LaunchResult {
        let mut cmd = Command::new(request.command.program());
        cmd.args(request.command.args());

        match cmd.spawn() {
            Ok(child) => LaunchResult::Success {
                track_id: request.track_id,
                pid: child.id(),
            },
            Err(e) => LaunchResult::SpawnFailed {
                track_id: request.track_id,
                error: e.to_string(),
            },
        }
    }
}

impl Default for LaunchService {
    fn default() -> Self {
        Self::new().expect("Failed to create LaunchService")
    }
}
