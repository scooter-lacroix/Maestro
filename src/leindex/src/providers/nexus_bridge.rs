use anyhow::{Context, Result};
use serde_json::json;
use tokio::process::Command;

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
            "session end",
        )
        .await
    }

    async fn run_session_command(
        &self,
        profile: &SessionProviderProfile,
        args: &[&str],
        reason: &str,
    ) -> Result<()> {
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

        let output = command
            .output()
            .await
            .context("failed to invoke nexus bridge")?;
        if !output.status.success() {
            anyhow::bail!(
                "nexus bridge command failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
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
