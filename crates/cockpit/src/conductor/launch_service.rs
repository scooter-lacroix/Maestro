//! Launch service for executing orchestrate commands
//!
//! Provides safe command execution with verification and timeout handling.
//! Supports the canonical `AgentSessionLaunchSpec` pipeline with provider-aware
//! suppression and overlap-matrix diagnostics.

use super::pane::CommandArgs;
use leindex_core::provider_boundary::{
    CapabilityOverlapMatrix, ProviderStatus, RuntimeDiagnostics, ToolSuppressionPolicy,
};
use std::collections::BTreeMap;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

/// Provider-aware context attached to a launch for suppression and diagnostics.
#[derive(Debug, Clone)]
pub struct LaunchProviderContext {
    /// Suppression policy derived from the overlap matrix at launch time.
    pub suppression_policy: ToolSuppressionPolicy,
    /// Overlap matrix snapshot used to derive the suppression policy.
    pub overlap_matrix: CapabilityOverlapMatrix,
    /// Runtime diagnostics captured at launch time.
    pub runtime_diagnostics: RuntimeDiagnostics,
    /// Environment overrides to inject into the child process.
    pub environment_overrides: BTreeMap<String, String>,
}

impl LaunchProviderContext {
    /// Build a provider context from a suppression policy and overlap matrix.
    pub fn from_policy_and_matrix(
        suppression_policy: ToolSuppressionPolicy,
        overlap_matrix: CapabilityOverlapMatrix,
    ) -> Self {
        let runtime_diagnostics = RuntimeDiagnostics {
            captured_at: chrono::Utc::now(),
            aggregate_status: ProviderStatus::Healthy,
            suppressed_count: suppression_policy.suppressed_tools.len(),
            analysis_preferred_count: suppression_policy.analysis_preferred_tools.len(),
            memory_preferred_count: suppression_policy.memory_preferred_tools.len(),
            retained_count: suppression_policy.retained_maestro_tools.len(),
            overlap_entry_count: overlap_matrix.entries.len(),
            provider_details: vec![],
        };

        let mut environment_overrides = BTreeMap::new();
        if let Ok(json) = suppression_policy.to_json_string() {
            environment_overrides.insert("MAESTRO_TOOL_SUPPRESSION_POLICY".into(), json);
        }

        Self {
            suppression_policy,
            overlap_matrix,
            runtime_diagnostics,
            environment_overrides,
        }
    }

    /// Human-readable diagnostics summary line.
    pub fn diagnostics_summary(&self) -> String {
        self.runtime_diagnostics.summary_line()
    }
}

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
pub struct LaunchRequest {
    track_id: String,
    command: CommandArgs,
    /// Optional provider-aware context for suppression and diagnostics.
    provider_context: Option<LaunchProviderContext>,
}

impl LaunchRequest {
    pub fn new(track_id: impl Into<String>, command: CommandArgs) -> Self {
        Self {
            track_id: track_id.into(),
            command,
            provider_context: None,
        }
    }

    /// Attach a provider context to this launch request.
    pub fn with_provider_context(mut self, ctx: LaunchProviderContext) -> Self {
        self.provider_context = Some(ctx);
        self
    }

    /// Get a reference to the provider context, if any.
    pub fn provider_context(&self) -> Option<&LaunchProviderContext> {
        self.provider_context.as_ref()
    }
}

/// Service for launching orchestrate commands with timeout support
pub struct LaunchService {
    timeout_secs: u64,
}

impl LaunchService {
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
                        if let Err(e) = child.kill() {
                            eprintln!(
                                "Warning: Failed to kill timed-out process {}: {}",
                                request.track_id, e
                            );
                        }
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

    /// Wait for a child process with a timeout using efficient polling
    fn wait_with_timeout(&self, child: &mut Child, timeout: Duration) -> Result<(), ()> {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(500); // Check every 500ms instead of 100ms

        loop {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()), // Process exited
                Ok(None) => {
                    // Process still running, check timeout
                    let elapsed = start.elapsed();
                    if elapsed >= timeout {
                        return Err(()); // Timeout
                    }
                    // Sleep for the check interval or remaining time, whichever is shorter
                    let remaining = timeout - elapsed;
                    let sleep_duration = if remaining < check_interval {
                        remaining
                    } else {
                        check_interval
                    };
                    thread::sleep(sleep_duration);
                }
                Err(_) => return Err(()), // Error checking status
            }
        }
    }

    /// Launch a command asynchronously (non-blocking) with timeout monitoring
    pub fn launch_async(&self, request: LaunchRequest) -> Result<LaunchHandle, String> {
        let mut cmd = Command::new(request.command.program());
        cmd.args(request.command.args());

        // Inject provider context environment overrides if present
        if let Some(ref ctx) = request.provider_context {
            for (key, value) in &ctx.environment_overrides {
                cmd.env(key, value);
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        Ok(LaunchHandle {
            track_id: request.track_id,
            child,
            timeout: self.timeout(),
            started_at: std::time::Instant::now(),
            provider_context: request.provider_context,
        })
    }

    /// Launch with a full provider-aware context, returning both the handle
    /// and a diagnostics summary suitable for toast/display.
    pub fn launch_with_diagnostics(
        &self,
        request: LaunchRequest,
    ) -> (Result<LaunchHandle, String>, Option<String>) {
        let diagnostics = request
            .provider_context
            .as_ref()
            .map(|ctx| ctx.diagnostics_summary());
        (self.launch_async(request), diagnostics)
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
    /// Provider context captured at launch time for diagnostics.
    provider_context: Option<LaunchProviderContext>,
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

    /// Get the provider context, if any was attached at launch.
    pub fn provider_context(&self) -> Option<&LaunchProviderContext> {
        self.provider_context.as_ref()
    }

    /// Get a diagnostics summary line, or None if no provider context.
    pub fn diagnostics_summary(&self) -> Option<String> {
        self.provider_context
            .as_ref()
            .map(|ctx| ctx.diagnostics_summary())
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
