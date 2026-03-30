//! Provider boundary contracts for Maestro-managed agent sessions.
//!
//! These types define the stable boundary between:
//! - Maestro as the host/orchestrator/runtime shell
//! - Standalone LeIndex as the authoritative analysis provider
//! - Standalone Nexus as the authoritative memory/cognition provider
//! - The Maestro MCP pool as shared infrastructure for non-overlapping servers

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Runtime entry point that launched a managed session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchOrigin {
    MaestroClaw,
    Sessions,
    Conductor,
}

/// High-level ownership bucket for a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOwner {
    Maestro,
    StandaloneLeindex,
    StandaloneNexus,
    PooledSharedMcp,
    Delete,
}

/// Provider kind for analysis responsibilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisProviderKind {
    StandaloneLeindex,
}

/// Provider kind for memory and cognition responsibilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProviderKind {
    StandaloneNexus,
}

/// Provider status used in doctor reports and runtime diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Healthy,
    Degraded,
    Missing,
    Misconfigured,
}

/// Single capability ownership record in the provider matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityOwnership {
    pub capability: String,
    pub owner: CapabilityOwner,
    pub rationale: String,
    pub notes: Vec<String>,
}

/// Shared MCP server reference carried into a managed session profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PooledMcpServerRef {
    pub name: String,
    pub transport: String,
    pub source: String,
}

/// Machine-readable provider diagnostic snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderDiagnostic {
    pub provider_name: String,
    pub provider_kind: String,
    pub status: ProviderStatus,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub source: Option<String>,
    pub detail: String,
    pub capabilities: BTreeSet<String>,
    pub checked_at: DateTime<Utc>,
}

/// Doctor output for a provider or provider profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderDoctorReport {
    pub subject: String,
    pub status: ProviderStatus,
    pub diagnostics: Vec<ProviderDiagnostic>,
    pub warnings: Vec<String>,
    pub recommended_actions: Vec<String>,
}

/// How a capability overlap between providers should be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapResolution {
    /// Route to the analysis provider (e.g. LeIndex).
    PreferAnalysis,
    /// Route to the memory provider (e.g. Nexus).
    PreferMemory,
    /// Suppress the capability entirely in the managed session.
    Suppress,
    /// Keep the capability under Maestro's direct ownership.
    RetainMaestro,
}

/// A single capability claimed by multiple providers, with a chosen resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityOverlapEntry {
    /// The capability name (e.g. "symbol_search", "memory_query").
    pub capability: String,
    /// All providers that claim this capability.
    pub claimants: BTreeSet<String>,
    /// Chosen resolution for this overlap.
    pub resolution: OverlapResolution,
    /// Human-readable rationale.
    pub rationale: String,
}

/// Matrix of capability overlaps between providers, used to derive suppression policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityOverlapMatrix {
    /// Overlap entries keyed by capability name for O(1) lookup.
    pub entries: BTreeMap<String, CapabilityOverlapEntry>,
}

impl CapabilityOverlapEntry {
    /// Create a new overlap entry.
    pub fn new(
        capability: impl Into<String>,
        claimants: BTreeSet<String>,
        resolution: OverlapResolution,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            capability: capability.into(),
            claimants,
            resolution,
            rationale: rationale.into(),
        }
    }
}

impl CapabilityOverlapMatrix {
    /// Create an empty matrix.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an overlap entry.
    pub fn add(&mut self, entry: CapabilityOverlapEntry) {
        self.entries.insert(entry.capability.clone(), entry);
    }

    /// Derive a `ToolSuppressionPolicy` from this overlap matrix.
    ///
    /// Each entry's resolution determines which bucket the capability falls into:
    /// - `PreferAnalysis` → `analysis_preferred_tools`
    /// - `PreferMemory` → `memory_preferred_tools`
    /// - `Suppress` → `suppressed_tools`
    /// - `RetainMaestro` → `retained_maestro_tools`
    pub fn to_suppression_policy(&self) -> ToolSuppressionPolicy {
        let mut policy = ToolSuppressionPolicy::default();
        for entry in self.entries.values() {
            match entry.resolution {
                OverlapResolution::PreferAnalysis => {
                    policy
                        .analysis_preferred_tools
                        .insert(entry.capability.clone());
                }
                OverlapResolution::PreferMemory => {
                    policy
                        .memory_preferred_tools
                        .insert(entry.capability.clone());
                }
                OverlapResolution::Suppress => {
                    policy.suppressed_tools.insert(entry.capability.clone());
                }
                OverlapResolution::RetainMaestro => {
                    policy
                        .retained_maestro_tools
                        .insert(entry.capability.clone());
                }
            }
        }
        policy
    }
}

/// Explicit suppression/preference rules for a managed session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolSuppressionPolicy {
    /// Tool names that must not be exposed to a Maestro-managed session.
    pub suppressed_tools: BTreeSet<String>,
    /// Tool names that should be routed to the analysis provider.
    pub analysis_preferred_tools: BTreeSet<String>,
    /// Tool names that should be routed to the memory provider.
    pub memory_preferred_tools: BTreeSet<String>,
    /// Maestro-owned tools that remain explicitly allowed.
    pub retained_maestro_tools: BTreeSet<String>,
}

impl ToolSuppressionPolicy {
    /// Create a suppression policy from an overlap matrix.
    pub fn from_overlap_matrix(matrix: &CapabilityOverlapMatrix) -> Self {
        matrix.to_suppression_policy()
    }

    /// Check whether a tool name is suppressed.
    pub fn is_suppressed(&self, tool: &str) -> bool {
        self.suppressed_tools.contains(tool)
    }

    /// Check whether a tool should be routed to the analysis provider.
    pub fn is_analysis_preferred(&self, tool: &str) -> bool {
        self.analysis_preferred_tools.contains(tool)
    }

    /// Check whether a tool should be routed to the memory provider.
    pub fn is_memory_preferred(&self, tool: &str) -> bool {
        self.memory_preferred_tools.contains(tool)
    }

    /// Check whether a tool is explicitly retained under Maestro ownership.
    pub fn is_retained(&self, tool: &str) -> bool {
        self.retained_maestro_tools.contains(tool)
    }

    /// Return the set of all tools referenced across all buckets.
    pub fn all_referenced_tools(&self) -> BTreeSet<String> {
        let mut all = BTreeSet::new();
        all.extend(self.suppressed_tools.iter().cloned());
        all.extend(self.analysis_preferred_tools.iter().cloned());
        all.extend(self.memory_preferred_tools.iter().cloned());
        all.extend(self.retained_maestro_tools.iter().cloned());
        all
    }
}

impl ToolSuppressionPolicy {
    /// Render the policy as compact JSON for environment export and diagnostics.
    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Concrete per-CLI overlap matrix for Maestro-managed sessions.
///
/// This matrix keeps the policy boundary explicit:
/// - analysis work is routed to standalone LeIndex
/// - memory/cognition work is routed to standalone Nexus
/// - Maestro pool entries remain only for non-overlapping shared servers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedCliOverlapProfile {
    pub cli: String,
    pub launch_surface: String,
    pub analysis_preferred_tools: BTreeSet<String>,
    pub memory_preferred_tools: BTreeSet<String>,
    pub suppressed_pool_entries: BTreeSet<String>,
    pub retained_maestro_tools: BTreeSet<String>,
    pub notes: Vec<String>,
}

impl ManagedCliOverlapProfile {
    pub fn suppression_policy(&self) -> ToolSuppressionPolicy {
        ToolSuppressionPolicy {
            suppressed_tools: self.suppressed_pool_entries.clone(),
            analysis_preferred_tools: self.analysis_preferred_tools.clone(),
            memory_preferred_tools: self.memory_preferred_tools.clone(),
            retained_maestro_tools: self.retained_maestro_tools.clone(),
        }
    }
}

/// Return the canonical overlap profile for a supported CLI.
pub fn managed_cli_overlap_profile(cli: &str) -> ManagedCliOverlapProfile {
    let normalized = cli.trim().to_ascii_lowercase();
    let analysis_preferred_tools = BTreeSet::from([
        "project_map".to_string(),
        "deep_analyze".to_string(),
        "phase_analysis".to_string(),
    ]);
    let memory_preferred_tools = BTreeSet::from([
        "memory".to_string(),
        "working_set".to_string(),
        "memory_query".to_string(),
    ]);
    let retained_maestro_tools = BTreeSet::from([
        "shell".to_string(),
        "file".to_string(),
        "cron_add".to_string(),
        "cron_list".to_string(),
        "cron_remove".to_string(),
    ]);
    let suppressed_pool_entries =
        BTreeSet::from(["maestro-tool-search".to_string(), "leindex".to_string()]);

    let (launch_surface, notes) = match normalized.as_str() {
        "claude" => (
            "mcp-config".to_string(),
            vec![
                "Claude sessions receive a direct standalone LeIndex entry and a Nexus lifecycle bridge."
                    .to_string(),
                "Shared MCP pool entries remain available for non-overlapping servers only.".to_string(),
            ],
        ),
        "codex" => (
            "toml-overrides".to_string(),
            vec![
                "Codex uses command-line overrides, but the same provider profile and suppression policy still apply."
                    .to_string(),
            ],
        ),
        "opencode" => (
            "opencode-config".to_string(),
            vec![
                "OpenCode receives a generated config with direct providers plus pooled shared servers.".to_string(),
            ],
        ),
        "gemini" => (
            "system-settings".to_string(),
            vec![
                "Gemini is launched with exported settings files, not a pooled broker path.".to_string(),
            ],
        ),
        "qwen" => (
            "system-settings".to_string(),
            vec![
                "Qwen uses the same provider profile but a different config surface.".to_string(),
            ],
        ),
        "iflow" => (
            "system-settings".to_string(),
            vec![
                "Iflow is treated as a managed session with direct providers and filtered shared MCP entries."
                    .to_string(),
            ],
        ),
        "amp" => (
            "mcp-config".to_string(),
            vec![
                "Amp receives a generated MCP configuration with direct provider entries first.".to_string(),
            ],
        ),
        "droid" => (
            "home-overlay".to_string(),
            vec![
                "Droid is routed through a temporary HOME overlay, but the same provider policy still applies."
                    .to_string(),
            ],
        ),
        _ => (
            "mcp-config".to_string(),
            vec![
                "Unknown managed CLIs fall back to the conservative direct-provider profile.".to_string(),
            ],
        ),
    };

    ManagedCliOverlapProfile {
        cli: normalized,
        launch_surface,
        analysis_preferred_tools,
        memory_preferred_tools,
        suppressed_pool_entries,
        retained_maestro_tools,
        notes,
    }
}

/// Return the concrete matrix across all supported managed-session CLIs.
pub fn managed_cli_overlap_matrix() -> Vec<ManagedCliOverlapProfile> {
    vec![
        managed_cli_overlap_profile("claude"),
        managed_cli_overlap_profile("codex"),
        managed_cli_overlap_profile("opencode"),
        managed_cli_overlap_profile("gemini"),
        managed_cli_overlap_profile("qwen"),
        managed_cli_overlap_profile("iflow"),
        managed_cli_overlap_profile("amp"),
        managed_cli_overlap_profile("droid"),
    ]
}

/// Build a concrete capability overlap matrix for one managed CLI.
pub fn managed_cli_overlap_matrix_for(cli: &str) -> CapabilityOverlapMatrix {
    let profile = managed_cli_overlap_profile(cli);
    let mut matrix = CapabilityOverlapMatrix::new();

    for capability in &profile.analysis_preferred_tools {
        matrix.add(CapabilityOverlapEntry::new(
            capability.clone(),
            BTreeSet::from([profile.cli.clone(), "leindex".to_string()]),
            OverlapResolution::PreferAnalysis,
            format!(
                "{} routes {} to standalone LeIndex",
                profile.cli, capability
            ),
        ));
    }

    for capability in &profile.memory_preferred_tools {
        matrix.add(CapabilityOverlapEntry::new(
            capability.clone(),
            BTreeSet::from([profile.cli.clone(), "nexus".to_string()]),
            OverlapResolution::PreferMemory,
            format!("{} routes {} to standalone Nexus", profile.cli, capability),
        ));
    }

    for capability in &profile.suppressed_pool_entries {
        matrix.add(CapabilityOverlapEntry::new(
            capability.clone(),
            BTreeSet::from([profile.cli.clone(), "maestro_pool".to_string()]),
            OverlapResolution::Suppress,
            format!(
                "{} suppresses pooled {} in managed sessions",
                profile.cli, capability
            ),
        ));
    }

    for capability in &profile.retained_maestro_tools {
        matrix.add(CapabilityOverlapEntry::new(
            capability.clone(),
            BTreeSet::from([profile.cli.clone(), "maestro".to_string()]),
            OverlapResolution::RetainMaestro,
            format!(
                "{} retains {} under Maestro ownership",
                profile.cli, capability
            ),
        ));
    }

    matrix
}

/// Provider-specific MCP server config payloads for a managed session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderMcpConfig {
    pub direct_servers: BTreeMap<String, serde_json::Value>,
    pub pooled_servers: BTreeMap<String, serde_json::Value>,
}

/// Canonical provider profile carried by Maestro-managed sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionProviderProfile {
    pub profile_name: String,
    pub launch_origin: LaunchOrigin,
    pub selected_cli: String,
    pub project_root: PathBuf,
    pub session_id: String,
    pub track_id: Option<String>,
    pub analysis_provider: AnalysisProviderKind,
    pub memory_provider: MemoryProviderKind,
    pub pooled_shared_servers: Vec<PooledMcpServerRef>,
    pub suppression_policy: ToolSuppressionPolicy,
    /// Overlap matrix that was used (or could be used) to derive the suppression policy.
    pub overlap_matrix: CapabilityOverlapMatrix,
    pub diagnostics: Vec<ProviderDiagnostic>,
}

/// Canonical launch specification shared by MaestroClaw, Sessions, and Conductor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionLaunchSpec {
    pub selected_cli: String,
    pub launch_origin: LaunchOrigin,
    pub project_root: PathBuf,
    pub session_id: String,
    pub track_id: Option<String>,
    pub provider_profile: SessionProviderProfile,
    pub mcp_config_path: Option<PathBuf>,
    pub environment_overrides: BTreeMap<String, String>,
    pub hook_attachments: Vec<String>,
    pub telemetry_destination: Option<String>,
    /// Runtime diagnostics snapshot captured at launch time.
    pub runtime_diagnostics: RuntimeDiagnostics,
}

impl AgentSessionLaunchSpec {
    /// Canonical builder that derives suppression policy from the overlap matrix,
    /// captures diagnostics, and produces a ready-to-launch spec.
    pub fn build(
        selected_cli: impl Into<String>,
        launch_origin: LaunchOrigin,
        project_root: impl Into<PathBuf>,
        session_id: impl Into<String>,
        track_id: Option<String>,
        provider_profile: SessionProviderProfile,
        mcp_config_path: Option<PathBuf>,
        hook_attachments: Vec<String>,
        telemetry_destination: Option<String>,
    ) -> Self {
        let overlap_matrix = &provider_profile.overlap_matrix;
        let suppression = ToolSuppressionPolicy::from_overlap_matrix(overlap_matrix);
        let runtime_diagnostics = RuntimeDiagnostics::from_profile(&provider_profile, &suppression);
        let selected_cli = selected_cli.into();

        let mut environment_overrides = BTreeMap::new();
        if let Ok(json) = suppression.to_json_string() {
            environment_overrides.insert("MAESTRO_TOOL_SUPPRESSION_POLICY".to_string(), json);
        }
        environment_overrides.insert(
            "MAESTRO_PROVIDER_PROFILE".to_string(),
            provider_profile.profile_name.clone(),
        );
        environment_overrides.insert("MAESTRO_SELECTED_CLI".to_string(), selected_cli.clone());
        environment_overrides.insert(
            "MAESTRO_ANALYSIS_PROVIDER".to_string(),
            format!("{:?}", provider_profile.analysis_provider).to_ascii_lowercase(),
        );
        environment_overrides.insert(
            "MAESTRO_MEMORY_PROVIDER".to_string(),
            format!("{:?}", provider_profile.memory_provider).to_ascii_lowercase(),
        );

        Self {
            selected_cli,
            launch_origin,
            project_root: project_root.into(),
            session_id: session_id.into(),
            track_id,
            provider_profile,
            mcp_config_path,
            environment_overrides,
            hook_attachments,
            telemetry_destination,
            runtime_diagnostics,
        }
    }
}

/// Runtime diagnostics snapshot captured at launch time.
///
/// Provides a machine-readable record of provider health, suppression policy
/// summary, and overlap matrix statistics for observability and debugging.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDiagnostics {
    /// Timestamp when diagnostics were captured.
    pub captured_at: DateTime<Utc>,
    /// Aggregate health across all providers.
    pub aggregate_status: ProviderStatus,
    /// Count of suppressed tools.
    pub suppressed_count: usize,
    /// Count of analysis-preferred tools.
    pub analysis_preferred_count: usize,
    /// Count of memory-preferred tools.
    pub memory_preferred_count: usize,
    /// Count of retained Maestro tools.
    pub retained_count: usize,
    /// Total overlap entries in the matrix.
    pub overlap_entry_count: usize,
    /// Per-provider diagnostic details.
    pub provider_details: Vec<ProviderDiagnostic>,
}

impl RuntimeDiagnostics {
    /// Build diagnostics from a provider profile and its suppression policy.
    pub fn from_profile(profile: &SessionProviderProfile, policy: &ToolSuppressionPolicy) -> Self {
        let statuses: Vec<ProviderStatus> = profile.diagnostics.iter().map(|d| d.status).collect();
        let aggregate_status = if statuses.is_empty() {
            ProviderStatus::Missing
        } else {
            combine_status(&statuses)
        };

        Self {
            captured_at: Utc::now(),
            aggregate_status,
            suppressed_count: policy.suppressed_tools.len(),
            analysis_preferred_count: policy.analysis_preferred_tools.len(),
            memory_preferred_count: policy.memory_preferred_tools.len(),
            retained_count: policy.retained_maestro_tools.len(),
            overlap_entry_count: profile.overlap_matrix.entries.len(),
            provider_details: profile.diagnostics.clone(),
        }
    }

    /// Render a compact human-readable summary for logging/toast display.
    pub fn summary_line(&self) -> String {
        format!(
            "providers:{} | suppressed:{} analysis:{} memory:{} retained:{} overlaps:{}",
            match self.aggregate_status {
                ProviderStatus::Healthy => "ok",
                ProviderStatus::Degraded => "degraded",
                ProviderStatus::Missing => "missing",
                ProviderStatus::Misconfigured => "misconfigured",
            },
            self.suppressed_count,
            self.analysis_preferred_count,
            self.memory_preferred_count,
            self.retained_count,
            self.overlap_entry_count,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderInstallMethod {
    Cargo,
    InstallScript,
    PyPiBootstrap,
    GitClone,
    Path,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInstallation {
    pub executable: PathBuf,
    pub method: ProviderInstallMethod,
    pub detail: String,
}

#[async_trait]
pub trait AnalysisProvider: Send + Sync {
    fn kind(&self) -> AnalysisProviderKind;
    fn name(&self) -> &str;
    fn executable_hint(&self) -> &str;

    async fn validate_health(&self, project_root: &std::path::Path)
        -> Result<ProviderDoctorReport>;
    async fn build_mcp_config(
        &self,
        project_root: &std::path::Path,
        session_id: &str,
    ) -> Result<ProviderMcpConfig>;
}

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    fn kind(&self) -> MemoryProviderKind;
    fn name(&self) -> &str;
    fn executable_hint(&self) -> &str;

    async fn validate_health(&self, project_root: &std::path::Path)
        -> Result<ProviderDoctorReport>;
    async fn session_started(&self, profile: &SessionProviderProfile) -> Result<()>;
    async fn session_event(
        &self,
        profile: &SessionProviderProfile,
        event_kind: MemoryLifecycleEventKind,
        reason: &str,
    ) -> Result<()>;
    async fn session_stopped(&self, profile: &SessionProviderProfile) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLifecycleEventKind {
    Compact,
    Checkpoint,
    Completion,
    Review,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(provider_name: &str, provider_kind: &str) -> ProviderDiagnostic {
        ProviderDiagnostic {
            provider_name: provider_name.to_string(),
            provider_kind: provider_kind.to_string(),
            status: ProviderStatus::Healthy,
            executable: Some(provider_name.to_string()),
            version: Some("1.0.0".to_string()),
            source: Some("test".to_string()),
            detail: "ok".to_string(),
            capabilities: BTreeSet::from(["health".to_string(), "mcp".to_string()]),
            checked_at: Utc::now(),
        }
    }

    #[test]
    fn provider_profile_captures_managed_session_boundary() {
        let profile = SessionProviderProfile {
            profile_name: "maestro_runtime".to_string(),
            launch_origin: LaunchOrigin::MaestroClaw,
            selected_cli: "claude".to_string(),
            project_root: PathBuf::from("/tmp/project"),
            session_id: "session-1".to_string(),
            track_id: Some("track-42".to_string()),
            analysis_provider: AnalysisProviderKind::StandaloneLeindex,
            memory_provider: MemoryProviderKind::StandaloneNexus,
            pooled_shared_servers: vec![PooledMcpServerRef {
                name: "github".to_string(),
                transport: "stdio".to_string(),
                source: "maestro_pool".to_string(),
            }],
            suppression_policy: ToolSuppressionPolicy {
                suppressed_tools: BTreeSet::from(["grep".to_string(), "memory".to_string()]),
                analysis_preferred_tools: BTreeSet::from(["project_map".to_string()]),
                memory_preferred_tools: BTreeSet::from(["working_set".to_string()]),
                retained_maestro_tools: BTreeSet::from(["shell".to_string()]),
            },
            overlap_matrix: CapabilityOverlapMatrix::default(),
            diagnostics: vec![
                diagnostic("leindex", "analysis"),
                diagnostic("nexus", "memory"),
            ],
        };

        assert_eq!(profile.profile_name, "maestro_runtime");
        assert_eq!(profile.launch_origin, LaunchOrigin::MaestroClaw);
        assert_eq!(
            profile.analysis_provider,
            AnalysisProviderKind::StandaloneLeindex
        );
        assert_eq!(profile.memory_provider, MemoryProviderKind::StandaloneNexus);
        assert!(profile.suppression_policy.suppressed_tools.contains("grep"));
        assert_eq!(profile.pooled_shared_servers.len(), 1);
    }

    #[test]
    fn launch_spec_wraps_provider_profile_for_all_launch_surfaces() {
        let profile = SessionProviderProfile {
            profile_name: "maestro_runtime".to_string(),
            launch_origin: LaunchOrigin::Conductor,
            selected_cli: "codex".to_string(),
            project_root: PathBuf::from("/tmp/project"),
            session_id: "session-2".to_string(),
            track_id: None,
            analysis_provider: AnalysisProviderKind::StandaloneLeindex,
            memory_provider: MemoryProviderKind::StandaloneNexus,
            pooled_shared_servers: Vec::new(),
            suppression_policy: ToolSuppressionPolicy::default(),
            overlap_matrix: CapabilityOverlapMatrix::default(),
            diagnostics: vec![diagnostic("leindex", "analysis")],
        };

        let launch = AgentSessionLaunchSpec {
            selected_cli: "codex".to_string(),
            launch_origin: LaunchOrigin::Conductor,
            project_root: PathBuf::from("/tmp/project"),
            session_id: "session-2".to_string(),
            track_id: None,
            provider_profile: profile.clone(),
            mcp_config_path: Some(PathBuf::from("/tmp/managed-mcp.json")),
            environment_overrides: BTreeMap::from([(
                "MAESTRO_PROVIDER_PROFILE".to_string(),
                "maestro_runtime".to_string(),
            )]),
            hook_attachments: vec!["pre-compact".to_string(), "session-start".to_string()],
            telemetry_destination: Some("cockpit".to_string()),
            runtime_diagnostics: RuntimeDiagnostics::from_profile(
                &profile,
                &ToolSuppressionPolicy::default(),
            ),
        };

        assert_eq!(launch.provider_profile.profile_name, "maestro_runtime");
        assert_eq!(launch.launch_origin, LaunchOrigin::Conductor);
        assert_eq!(
            launch.environment_overrides.get("MAESTRO_PROVIDER_PROFILE"),
            Some(&"maestro_runtime".to_string())
        );
        assert_eq!(launch.hook_attachments.len(), 2);
    }

    #[test]
    fn managed_cli_overlap_matrix_covers_supported_managed_clis() {
        let matrix = managed_cli_overlap_matrix();
        let clis = matrix
            .iter()
            .map(|entry| entry.cli.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(matrix.len(), 8);
        for cli in [
            "claude", "codex", "opencode", "gemini", "qwen", "iflow", "amp", "droid",
        ] {
            assert!(clis.contains(cli), "missing overlap profile for {cli}");
        }
    }

    #[test]
    fn managed_cli_overlap_profile_derives_suppression_policy() {
        let profile = managed_cli_overlap_profile("claude");
        let policy = profile.suppression_policy();

        assert!(policy.suppressed_tools.contains("maestro-tool-search"));
        assert!(policy.analysis_preferred_tools.contains("project_map"));
        assert!(policy.memory_preferred_tools.contains("working_set"));
        assert!(policy.retained_maestro_tools.contains("shell"));
        assert_eq!(profile.launch_surface, "mcp-config");
    }

    #[test]
    fn discover_install_method_classifies_common_paths() {
        assert_eq!(
            classify_install_method(Path::new("/home/user/.cargo/bin/leindex")),
            ProviderInstallMethod::Cargo
        );
        assert_eq!(
            classify_install_method(Path::new("/tmp/venv/bin/leindex")),
            ProviderInstallMethod::PyPiBootstrap
        );
        assert_eq!(
            classify_install_method(Path::new("/opt/Nexus-Memory-System/target/release/nexus")),
            ProviderInstallMethod::GitClone
        );
    }

    #[test]
    fn overlap_matrix_derives_suppression_policy() {
        let mut matrix = CapabilityOverlapMatrix::new();
        matrix.add(CapabilityOverlapEntry::new(
            "symbol_search",
            BTreeSet::from(["maestro".to_string(), "leindex".to_string()]),
            OverlapResolution::PreferAnalysis,
            "LeIndex is the authoritative code search provider",
        ));
        matrix.add(CapabilityOverlapEntry::new(
            "memory_query",
            BTreeSet::from(["maestro".to_string(), "nexus".to_string()]),
            OverlapResolution::PreferMemory,
            "Nexus is the authoritative memory provider",
        ));
        matrix.add(CapabilityOverlapEntry::new(
            "builtin_grep",
            BTreeSet::from(["maestro".to_string(), "leindex".to_string()]),
            OverlapResolution::Suppress,
            "Redundant with leindex_text_search",
        ));
        matrix.add(CapabilityOverlapEntry::new(
            "shell",
            BTreeSet::from(["maestro".to_string()]),
            OverlapResolution::RetainMaestro,
            "Maestro-owned runtime tool",
        ));

        let policy = matrix.to_suppression_policy();

        assert!(policy.is_analysis_preferred("symbol_search"));
        assert!(policy.is_memory_preferred("memory_query"));
        assert!(policy.is_suppressed("builtin_grep"));
        assert!(policy.is_retained("shell"));

        // Verify round-trip via from_overlap_matrix
        let policy2 = ToolSuppressionPolicy::from_overlap_matrix(&matrix);
        assert_eq!(policy, policy2);

        // all_referenced_tools returns union
        let all = policy.all_referenced_tools();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn suppression_policy_helpers_work_independently() {
        let policy = ToolSuppressionPolicy {
            suppressed_tools: BTreeSet::from(["grep".to_string()]),
            analysis_preferred_tools: BTreeSet::from(["project_map".to_string()]),
            memory_preferred_tools: BTreeSet::from(["working_set".to_string()]),
            retained_maestro_tools: BTreeSet::from(["shell".to_string()]),
        };

        assert!(policy.is_suppressed("grep"));
        assert!(!policy.is_suppressed("shell"));
        assert!(policy.is_analysis_preferred("project_map"));
        assert!(policy.is_memory_preferred("working_set"));
        assert!(policy.is_retained("shell"));
        assert_eq!(policy.all_referenced_tools().len(), 4);
    }

    #[test]
    fn runtime_diagnostics_from_profile_captures_provider_health() {
        let profile = SessionProviderProfile {
            profile_name: "test".to_string(),
            launch_origin: LaunchOrigin::MaestroClaw,
            selected_cli: "claude".to_string(),
            project_root: PathBuf::from("/tmp/p"),
            session_id: "s1".to_string(),
            track_id: None,
            analysis_provider: AnalysisProviderKind::StandaloneLeindex,
            memory_provider: MemoryProviderKind::StandaloneNexus,
            pooled_shared_servers: vec![],
            suppression_policy: ToolSuppressionPolicy {
                suppressed_tools: BTreeSet::from(["grep".to_string()]),
                analysis_preferred_tools: BTreeSet::from(["project_map".to_string()]),
                memory_preferred_tools: BTreeSet::from(["memory".to_string()]),
                retained_maestro_tools: BTreeSet::from(["shell".to_string()]),
            },
            overlap_matrix: CapabilityOverlapMatrix::default(),
            diagnostics: vec![
                diagnostic("leindex", "analysis"),
                diagnostic("nexus", "memory"),
            ],
        };

        let diag = RuntimeDiagnostics::from_profile(&profile, &profile.suppression_policy);

        assert_eq!(diag.aggregate_status, ProviderStatus::Healthy);
        assert_eq!(diag.suppressed_count, 1);
        assert_eq!(diag.analysis_preferred_count, 1);
        assert_eq!(diag.memory_preferred_count, 1);
        assert_eq!(diag.retained_count, 1);
        assert_eq!(diag.provider_details.len(), 2);
        assert!(diag.summary_line().contains("providers:ok"));
    }

    #[test]
    fn launch_spec_build_derives_suppression_from_overlap_matrix() {
        let mut matrix = CapabilityOverlapMatrix::new();
        matrix.add(CapabilityOverlapEntry::new(
            "symbol_search",
            BTreeSet::from(["maestro".to_string(), "leindex".to_string()]),
            OverlapResolution::PreferAnalysis,
            "LeIndex is authoritative",
        ));
        matrix.add(CapabilityOverlapEntry::new(
            "builtin_grep",
            BTreeSet::from(["maestro".to_string(), "leindex".to_string()]),
            OverlapResolution::Suppress,
            "Redundant",
        ));

        let profile = SessionProviderProfile {
            profile_name: "test_build".to_string(),
            launch_origin: LaunchOrigin::Conductor,
            selected_cli: "codex".to_string(),
            project_root: PathBuf::from("/tmp/build-test"),
            session_id: "s-build".to_string(),
            track_id: Some("track-1".to_string()),
            analysis_provider: AnalysisProviderKind::StandaloneLeindex,
            memory_provider: MemoryProviderKind::StandaloneNexus,
            pooled_shared_servers: vec![],
            suppression_policy: ToolSuppressionPolicy::from_overlap_matrix(&matrix),
            overlap_matrix: matrix,
            diagnostics: vec![diagnostic("leindex", "analysis")],
        };

        let spec = AgentSessionLaunchSpec::build(
            "codex",
            LaunchOrigin::Conductor,
            PathBuf::from("/tmp/build-test"),
            "s-build",
            Some("track-1".to_string()),
            profile.clone(),
            Some(PathBuf::from("/tmp/mcp.json")),
            vec!["session-start".to_string()],
            Some("cockpit".to_string()),
        );

        assert_eq!(spec.selected_cli, "codex");
        assert_eq!(spec.track_id, Some("track-1".to_string()));
        assert!(spec
            .environment_overrides
            .contains_key("MAESTRO_TOOL_SUPPRESSION_POLICY"));
        assert!(spec
            .environment_overrides
            .contains_key("MAESTRO_PROVIDER_PROFILE"));
        assert_eq!(spec.runtime_diagnostics.overlap_entry_count, 2);
        assert_eq!(spec.runtime_diagnostics.suppressed_count, 1);
    }
}

pub(crate) fn classify_install_method(path: &Path) -> ProviderInstallMethod {
    let path_str = path.to_string_lossy();
    if path_str.contains(".cargo/bin") {
        ProviderInstallMethod::Cargo
    } else if path_str.contains("site-packages")
        || path_str.contains("dist-packages")
        || path_str.contains("venv")
    {
        ProviderInstallMethod::PyPiBootstrap
    } else if path_str.contains("install-leindex")
        || path_str.contains("install.sh")
        || path_str.contains("npm")
    {
        ProviderInstallMethod::InstallScript
    } else if path_str.contains("Nexus-Memory-System")
        || path_str.contains("LeIndex")
        || path_str.contains("LeIndexer")
        || path_str.contains("/target/release/")
    {
        ProviderInstallMethod::GitClone
    } else if path.is_absolute() {
        ProviderInstallMethod::Path
    } else {
        ProviderInstallMethod::Unknown
    }
}

pub(crate) fn discover_installation(
    env_var: &str,
    executable_name: &str,
) -> Option<ProviderInstallation> {
    if let Some(value) = std::env::var_os(env_var) {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(ProviderInstallation {
                method: classify_install_method(&path),
                detail: format!("Discovered via {}", env_var),
                executable: path,
            });
        }
    }

    let path_var = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path_var) {
        let candidate = directory.join(executable_name);
        if candidate.exists() {
            return Some(ProviderInstallation {
                method: classify_install_method(&candidate),
                detail: "Discovered on PATH".to_string(),
                executable: candidate,
            });
        }
    }

    None
}

pub(crate) async fn run_provider_check(
    executable: &Path,
    args: &[&str],
) -> (ProviderStatus, String) {
    match Command::new(executable).args(args).output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if !stdout.is_empty() {
                stdout
            } else if !stderr.is_empty() {
                stderr
            } else {
                "ok".to_string()
            };
            (ProviderStatus::Healthy, message)
        }
        Ok(output) => (
            ProviderStatus::Misconfigured,
            format!(
                "command failed with status {}",
                output.status.code().unwrap_or_default()
            ),
        ),
        Err(err) => (ProviderStatus::Missing, err.to_string()),
    }
}

pub(crate) fn combine_status(statuses: &[ProviderStatus]) -> ProviderStatus {
    if statuses
        .iter()
        .any(|status| *status == ProviderStatus::Missing)
    {
        ProviderStatus::Missing
    } else if statuses
        .iter()
        .any(|status| *status == ProviderStatus::Misconfigured)
    {
        ProviderStatus::Misconfigured
    } else if statuses
        .iter()
        .any(|status| *status == ProviderStatus::Degraded)
    {
        ProviderStatus::Degraded
    } else {
        ProviderStatus::Healthy
    }
}

impl ProviderInstallMethod {
    pub(crate) fn as_ref(&self) -> &'static str {
        match self {
            ProviderInstallMethod::Cargo => "cargo",
            ProviderInstallMethod::InstallScript => "install_script",
            ProviderInstallMethod::PyPiBootstrap => "pypi_bootstrap",
            ProviderInstallMethod::GitClone => "git_clone",
            ProviderInstallMethod::Path => "path",
            ProviderInstallMethod::Unknown => "unknown",
        }
    }
}
