use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;

use crate::provider_boundary::{
    combine_status, discover_installation, run_provider_check, MemoryLifecycleEventKind,
    MemoryProvider, MemoryProviderKind, ProviderDiagnostic, ProviderDoctorReport,
    ProviderInstallation, ProviderStatus, SessionProviderProfile,
};

use super::NexusRuntimeBridge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NexusInstallMethod {
    GitClone,
    Cargo,
}

impl NexusInstallMethod {
    pub fn install_hint(self) -> &'static str {
        match self {
            Self::GitClone => {
                "git clone https://github.com/scooter-lacroix/Nexus-Memory-System.git && cd Nexus-Memory-System && cargo build --release -p nexus-memory && ./scripts/install.sh --binary ./target/release/nexus && nexus init"
            }
            Self::Cargo => "cargo install --force nexus-memory && nexus init",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StandaloneNexusProvider {
    installation: ProviderInstallation,
    pub state_root: Option<PathBuf>,
}

impl StandaloneNexusProvider {
    pub fn discover() -> Option<Self> {
        let installation = discover_installation("NEXUS_BIN", "nexus")?;
        let state_root = std::env::var_os("NEXUS_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".nexus")));
        Some(Self {
            installation,
            state_root,
        })
    }

    pub fn installation(&self) -> &ProviderInstallation {
        &self.installation
    }

    pub fn supported_install_methods() -> [NexusInstallMethod; 2] {
        [NexusInstallMethod::GitClone, NexusInstallMethod::Cargo]
    }

    pub fn lifecycle_env(&self, profile: &SessionProviderProfile) -> Vec<(String, String)> {
        let mut env = vec![
            (
                "MAESTRO_PROVIDER_PROFILE".to_string(),
                profile.profile_name.clone(),
            ),
            ("MAESTRO_SESSION_ID".to_string(), profile.session_id.clone()),
            (
                "MAESTRO_PROJECT_ROOT".to_string(),
                profile.project_root.display().to_string(),
            ),
            (
                "MAESTRO_SELECTED_CLI".to_string(),
                profile.selected_cli.clone(),
            ),
        ];
        if let Some(track_id) = &profile.track_id {
            env.push(("MAESTRO_TRACK_ID".to_string(), track_id.clone()));
        }
        env
    }

    pub fn bridge(&self) -> NexusRuntimeBridge {
        NexusRuntimeBridge::new(self.clone())
    }

    fn diagnostic(
        &self,
        status: ProviderStatus,
        detail: impl Into<String>,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
        version: Option<String>,
    ) -> ProviderDiagnostic {
        ProviderDiagnostic {
            provider_name: "nexus".to_string(),
            provider_kind: "memory".to_string(),
            status,
            executable: Some(self.executable_hint().to_string()),
            version,
            source: Some(self.installation.method.as_ref().to_string()),
            detail: detail.into(),
            capabilities: capabilities.into_iter().map(Into::into).collect(),
            checked_at: Utc::now(),
        }
    }

    pub fn health_report_sync(&self, _project_root: &Path) -> Result<ProviderDoctorReport> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.validate_health(Path::new(".")))
    }
}

#[async_trait]
impl MemoryProvider for StandaloneNexusProvider {
    fn kind(&self) -> MemoryProviderKind {
        MemoryProviderKind::StandaloneNexus
    }

    fn name(&self) -> &str {
        "nexus"
    }

    fn executable_hint(&self) -> &str {
        self.installation.executable.to_str().unwrap_or("nexus")
    }

    async fn validate_health(&self, _project_root: &Path) -> Result<ProviderDoctorReport> {
        let version = run_provider_check(&self.installation.executable, &["--version"]).await;
        let init = run_provider_check(&self.installation.executable, &["init", "--help"]).await;
        let store = run_provider_check(&self.installation.executable, &["store", "--help"]).await;
        let search = run_provider_check(&self.installation.executable, &["search", "--help"]).await;
        let session =
            run_provider_check(&self.installation.executable, &["session", "--help"]).await;
        let storage_status = match self.state_root.as_ref() {
            Some(root) if root.exists() => (
                ProviderStatus::Healthy,
                format!("State root exists at {}", root.display()),
            ),
            Some(root) => (
                ProviderStatus::Misconfigured,
                format!("State root missing at {}", root.display()),
            ),
            None => (
                ProviderStatus::Misconfigured,
                "State root could not be derived".to_string(),
            ),
        };

        let diagnostics = vec![
            self.diagnostic(
                version.0,
                "Version check",
                ["version"],
                Some(version.1.clone()),
            ),
            self.diagnostic(init.0, "Init command help check", ["init"], None),
            self.diagnostic(store.0, "Store command help check", ["store"], None),
            self.diagnostic(search.0, "Search command help check", ["search"], None),
            self.diagnostic(
                session.0,
                "Session lifecycle command help check",
                ["session", "runtime"],
                None,
            ),
            self.diagnostic(storage_status.0, storage_status.1, ["storage"], None),
        ];

        let status = combine_status(
            &diagnostics
                .iter()
                .map(|item| item.status)
                .collect::<Vec<_>>(),
        );

        let mut recommended_actions = vec!["Run `nexus init` to initialize storage".to_string()];
        recommended_actions.extend(
            Self::supported_install_methods()
                .into_iter()
                .map(|method| method.install_hint().to_string()),
        );

        Ok(ProviderDoctorReport {
            subject: "standalone_nexus".to_string(),
            status,
            diagnostics,
            warnings: if status == ProviderStatus::Healthy {
                Vec::new()
            } else {
                vec!["Standalone Nexus is not fully healthy for managed-session use".to_string()]
            },
            recommended_actions,
        })
    }

    async fn session_started(&self, profile: &SessionProviderProfile) -> Result<()> {
        self.bridge().session_start(profile).await
    }

    async fn session_event(
        &self,
        profile: &SessionProviderProfile,
        event_kind: MemoryLifecycleEventKind,
        reason: &str,
    ) -> Result<()> {
        self.bridge()
            .session_event(profile, event_kind, reason)
            .await
    }

    async fn session_stopped(&self, profile: &SessionProviderProfile) -> Result<()> {
        self.bridge()
            .session_end(profile, "managed-session-stop")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_boundary::ProviderInstallMethod;

    #[test]
    fn supported_install_methods_cover_required_paths() {
        let methods = StandaloneNexusProvider::supported_install_methods();
        assert_eq!(methods.len(), 2);
        assert!(methods
            .iter()
            .any(|method| *method == NexusInstallMethod::GitClone));
        assert!(methods
            .iter()
            .any(|method| *method == NexusInstallMethod::Cargo));
    }

    #[test]
    fn lifecycle_env_tracks_session_identity() {
        let provider = StandaloneNexusProvider {
            installation: ProviderInstallation {
                executable: PathBuf::from("/usr/bin/nexus"),
                method: ProviderInstallMethod::Path,
                detail: "test".to_string(),
            },
            state_root: Some(PathBuf::from("/tmp/.nexus")),
        };

        let profile = SessionProviderProfile {
            profile_name: "maestro_runtime".to_string(),
            launch_origin: crate::provider_boundary::LaunchOrigin::Sessions,
            selected_cli: "claude".to_string(),
            project_root: PathBuf::from("/tmp/project"),
            session_id: "session-123".to_string(),
            track_id: Some("track-1".to_string()),
            analysis_provider: crate::provider_boundary::AnalysisProviderKind::StandaloneLeindex,
            memory_provider: MemoryProviderKind::StandaloneNexus,
            pooled_shared_servers: Vec::new(),
            suppression_policy: crate::provider_boundary::ToolSuppressionPolicy::default(),
            overlap_matrix: crate::provider_boundary::CapabilityOverlapMatrix::default(),
            diagnostics: Vec::new(),
        };

        let env = provider.lifecycle_env(&profile);
        assert!(env
            .iter()
            .any(|(k, v)| k == "MAESTRO_SESSION_ID" && v == "session-123"));
        assert!(env
            .iter()
            .any(|(k, v)| k == "MAESTRO_TRACK_ID" && v == "track-1"));
    }
}
