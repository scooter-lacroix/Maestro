use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::AsyncReadExt;
use serde_json::json;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::provider_boundary::{MemoryLifecycleEventKind, SessionProviderProfile};

use super::StandaloneNexusProvider;

#[derive(Debug, Clone)]
pub struct NexusRuntimeBridge {
    provider: StandaloneNexusProvider,
}

impl NexusRuntimeBridge {
    pub fn new(provider: StandaloneNexusProvider) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> &StandaloneNexusProvider {
        &self.provider
    }

    pub async fn session_start(&self, profile: &SessionProviderProfile) -> Result<()> {
        self.run_session_command(
            profile,
            &[
                "session",
                "start",
                "--agent",
                profile.selected_cli.as_str(),
                "--session-key",
                profile.session_id.as_str(),
                "--cwd",
                profile.project_root.to_string_lossy().as_ref(),
            ],
            "session start",
        )
        .await
    }

    pub async fn session_event(
        &self,
        profile: &SessionProviderProfile,
        event_kind: MemoryLifecycleEventKind,
        reason: &str,
    ) -> Result<()> {
        self.run_session_command(
            profile,
            &[
                "session",
                "event",
                "--agent",
                profile.selected_cli.as_str(),
                "--session-key",
                profile.session_id.as_str(),
                "--cwd",
                profile.project_root.to_string_lossy().as_ref(),
                "--kind",
                event_kind.as_ref(),
            ],
            reason,
        )
        .await
    }

    pub async fn session_end(&self, profile: &SessionProviderProfile, reason: &str) -> Result<()> {
        self.run_session_command(
            profile,
            &[
                "session",
                "end",
                "--agent",
                profile.selected_cli.as_str(),
                "--session-key",
                profile.session_id.as_str(),
                "--cwd",
                profile.project_root.to_string_lossy().as_ref(),
                "--reason",
                reason,
            ],
            reason,
        )
        .await
    }

    async fn run_session_command(
        &self,
        profile: &SessionProviderProfile,
        args: &[&str],
        reason: &str,
    ) -> Result<()> {
        const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

        let mut command = Command::new(self.provider.installation().executable.clone());
        command.args(args);
        if let Some(root) = &self.provider.state_root {
            command.env("NEXUS_HOME", root);
        }
        command.envs(self.provider.lifecycle_env(profile));
        command.env(
            "MAESTRO_NEXUS_EVENT_PAYLOAD",
            self.event_payload(profile, reason).to_string(),
        );

        // Spawn the process with piped stderr for diagnostics; stdout not consumed so use null
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to invoke nexus bridge")?;

        // Wait for completion with timeout
        let result = timeout(COMMAND_TIMEOUT, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                if !status.success() {
                    // Read captured stderr for diagnostics
                    let mut stderr_buf = Vec::new();
                    if let Some(mut stderr) = child.stderr.take() {
                        let _ = stderr.read_to_end(&mut stderr_buf).await;
                    }
                    let stderr_text = String::from_utf8_lossy(&stderr_buf);
                    anyhow::bail!(
                        "nexus bridge command failed with exit status: {}\nstderr: {}",
                        status,
                        stderr_text.trim()
                    );
                }
            }
            Ok(Err(e)) => {
                anyhow::bail!("failed to wait for nexus bridge command: {}", e);
            }
            Err(_) => {
                // Timeout occurred - kill the spawned process
                let _ = child.kill().await;
                // Wait for the kill to take effect so stderr reaches EOF
                let _ = child.wait().await;
                // Read any stderr produced before the kill
                let mut stderr_buf = Vec::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let _ = stderr.read_to_end(&mut stderr_buf).await;
                }
                let stderr_text = String::from_utf8_lossy(&stderr_buf).trim().to_string();
                let detail = if stderr_text.is_empty() {
                    String::new()
                } else {
                    format!("\nstderr: {}", stderr_text)
                };
                anyhow::bail!(
                    "nexus bridge command timed out after {:?}{}",
                    COMMAND_TIMEOUT,
                    detail
                );
            }
        }

        Ok(())
    }

    fn event_payload(&self, profile: &SessionProviderProfile, reason: &str) -> serde_json::Value {
        json!({
            "managed_session": true,
            "provider_profile": profile.profile_name,
            "session_id": profile.session_id,
            "project_root": profile.project_root,
            "track_id": profile.track_id,
            "launch_origin": profile.launch_origin,
            "selected_cli": profile.selected_cli,
            "event_reason": reason,
        })
    }
}

impl MemoryLifecycleEventKind {
    pub fn as_ref(self) -> &'static str {
        match self {
            MemoryLifecycleEventKind::Compact => "compact",
            MemoryLifecycleEventKind::Checkpoint => "checkpoint",
            MemoryLifecycleEventKind::Completion => "completion",
            MemoryLifecycleEventKind::Review => "review",
        }
    }
}
