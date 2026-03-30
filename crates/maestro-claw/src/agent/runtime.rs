use std::sync::Arc;
use std::{collections::BTreeSet, env};

use uuid::Uuid;

use super::{agent_loop, AgentConfig, AgentError, AgentResult, CliProvider, CliProviderConfig};
use crate::config::{schema::AutonomyConfig, Config};
use crate::hooks::HookSystem;
use crate::session::{Thread, Turn, TurnRole};
use crate::tools::{
    builtin::{
        CronAddTool, CronListTool, CronRemoveTool, FileTool, FileToolConfig, ShellTool,
        ShellToolConfig,
    },
    Tool, ToolRegistry,
};
use crate::{LoggingHook, MemoryHook};
use serde::{Deserialize, Serialize};

#[cfg(feature = "core-integration")]
use crate::integration::SecurityPolicyBridge;

fn is_autonomous(autonomy: &AutonomyConfig) -> bool {
    autonomy.level.eq_ignore_ascii_case("autonomous")
}

fn is_read_only(autonomy: &AutonomyConfig) -> bool {
    autonomy.level.eq_ignore_ascii_case("read-only")
}

fn build_file_tool_config(config: &Config) -> FileToolConfig {
    let autonomy = &config.autonomy;
    FileToolConfig {
        base_directory: autonomy
            .workspace_only
            .then(|| config.workspace_dir.clone()),
        allow_write: !is_read_only(autonomy),
        allow_delete: is_autonomous(autonomy),
        ..FileToolConfig::default()
    }
}

fn build_shell_tool_config(config: &Config) -> ShellToolConfig {
    let autonomy = &config.autonomy;
    ShellToolConfig {
        allow_moderate: !is_read_only(autonomy),
        allow_dangerous: is_autonomous(autonomy),
        working_directory: Some(config.workspace_dir.clone()),
        allowed_commands: autonomy.allowed_commands.clone(),
        ..ShellToolConfig::default()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct ManagedToolSuppressionPolicy {
    #[serde(default)]
    suppressed_tools: BTreeSet<String>,
    #[serde(default)]
    analysis_preferred_tools: BTreeSet<String>,
    #[serde(default)]
    memory_preferred_tools: BTreeSet<String>,
    #[serde(default)]
    retained_maestro_tools: BTreeSet<String>,
}

/// Snapshot of runtime diagnostics captured at agent startup.
///
/// This struct provides a machine-readable record of the suppression policy
/// state that was active when the agent was launched, useful for debugging
/// and telemetry without requiring a dependency on the full provider boundary types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDiagnosticsSnapshot {
    /// Whether a suppression policy was loaded from the environment.
    pub policy_loaded: bool,
    /// Count of suppressed tools.
    pub suppressed_count: usize,
    /// Count of analysis-preferred tools.
    pub analysis_preferred_count: usize,
    /// Count of memory-preferred tools.
    pub memory_preferred_count: usize,
    /// Count of retained Maestro tools.
    pub retained_count: usize,
    /// Total tools in the registry after filtering.
    pub active_tool_count: usize,
    /// The primary tool this agent is configured for.
    pub primary_tool: String,
}

impl RuntimeDiagnosticsSnapshot {
    /// Capture a diagnostics snapshot from policy bucket counts and tool state.
    pub fn capture(
        suppressed_count: usize,
        analysis_preferred_count: usize,
        memory_preferred_count: usize,
        retained_count: usize,
        active_tool_count: usize,
        primary_tool: &str,
    ) -> Self {
        Self {
            policy_loaded: suppressed_count > 0
                || analysis_preferred_count > 0
                || memory_preferred_count > 0
                || retained_count > 0,
            suppressed_count,
            analysis_preferred_count,
            memory_preferred_count,
            retained_count,
            active_tool_count,
            primary_tool: primary_tool.to_string(),
        }
    }

    /// Render a compact human-readable summary for logging.
    pub fn summary_line(&self) -> String {
        if !self.policy_loaded {
            format!("tool-policy:none | active:{}", self.active_tool_count)
        } else {
            format!(
                "tool-policy:loaded | suppressed:{} analysis:{} memory:{} retained:{} active:{}",
                self.suppressed_count,
                self.analysis_preferred_count,
                self.memory_preferred_count,
                self.retained_count,
                self.active_tool_count,
            )
        }
    }
}

impl ManagedToolSuppressionPolicy {
    fn from_env() -> Option<Self> {
        let raw = env::var("MAESTRO_TOOL_SUPPRESSION_POLICY").ok()?;
        serde_json::from_str(&raw).ok()
    }

    fn allows(&self, tool_name: &str) -> bool {
        self.retained_maestro_tools.contains(tool_name)
            || !self.suppressed_tools.contains(tool_name)
    }
}

fn filter_tools(
    tools: impl IntoIterator<Item = Arc<dyn Tool>>,
    policy: Option<&ManagedToolSuppressionPolicy>,
) -> Vec<Arc<dyn Tool>> {
    tools
        .into_iter()
        .filter(|tool| policy.map_or(true, |policy| policy.allows(tool.name())))
        .collect()
}

pub fn build_default_tools(config: &Config) -> Vec<Arc<dyn Tool>> {
    let policy = ManagedToolSuppressionPolicy::from_env();
    build_default_tools_with_policy(config, policy.as_ref())
}

fn build_default_tools_with_policy(
    config: &Config,
    policy: Option<&ManagedToolSuppressionPolicy>,
) -> Vec<Arc<dyn Tool>> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(FileTool::with_config(build_file_tool_config(config))),
        Arc::new(ShellTool::with_config(build_shell_tool_config(config))),
        Arc::new(CronAddTool::new(&config.workspace_dir)),
        Arc::new(CronListTool::new(&config.workspace_dir)),
        Arc::new(CronRemoveTool::new(&config.workspace_dir)),
    ];
    filter_tools(tools, policy)
}

pub fn build_tool_registry(
    tools: impl IntoIterator<Item = Arc<dyn Tool>>,
    #[cfg(feature = "core-integration")] security_bridge: Option<SecurityPolicyBridge>,
) -> Arc<ToolRegistry> {
    let mut registry = ToolRegistry::new();
    for tool in tools {
        #[cfg(feature = "core-integration")]
        let tool = if let Some(bridge) = &security_bridge {
            bridge.clone().wrap_shared_tool(tool)
        } else {
            tool
        };
        registry.register(tool);
    }
    Arc::new(registry)
}

pub fn build_default_tool_registry(config: &Config) -> Arc<ToolRegistry> {
    let policy = ManagedToolSuppressionPolicy::from_env();
    #[cfg(feature = "core-integration")]
    {
        return build_tool_registry(
            build_default_tools_with_policy(config, policy.as_ref()),
            None,
        );
    }

    #[cfg(not(feature = "core-integration"))]
    {
        build_tool_registry(build_default_tools_with_policy(config, policy.as_ref()))
    }
}

#[cfg(feature = "core-integration")]
pub fn build_default_tool_registry_with_extras(
    config: &Config,
    extra_tools: impl IntoIterator<Item = Arc<dyn Tool>>,
    security_bridge: Option<SecurityPolicyBridge>,
) -> Arc<ToolRegistry> {
    let policy = ManagedToolSuppressionPolicy::from_env();
    build_tool_registry(
        build_default_tools_with_policy(config, policy.as_ref())
            .into_iter()
            .chain(filter_tools(extra_tools, policy.as_ref())),
        security_bridge,
    )
}

#[cfg(not(feature = "core-integration"))]
pub fn build_default_tool_registry_with_extras(
    config: &Config,
    extra_tools: impl IntoIterator<Item = Arc<dyn Tool>>,
) -> Arc<ToolRegistry> {
    let policy = ManagedToolSuppressionPolicy::from_env();
    build_tool_registry(
        build_default_tools_with_policy(config, policy.as_ref())
            .into_iter()
            .chain(filter_tools(extra_tools, policy.as_ref())),
    )
}

pub fn build_default_hook_system(primary_tool: &str) -> Arc<HookSystem> {
    let mut hooks = HookSystem::new();
    hooks.register(Arc::new(LoggingHook::new(primary_tool)));
    hooks.register(Arc::new(MemoryHook::new(primary_tool)));
    Arc::new(hooks)
}

fn build_agent_config(timeout_secs: u64) -> AgentConfig {
    AgentConfig::default().with_timeout(timeout_secs)
}

pub async fn run_prompt(
    config: &Config,
    prompt: impl Into<String>,
    timeout_secs: u64,
) -> Result<AgentResult, AgentError> {
    let session_id = Uuid::new_v4().to_string();
    let mut thread = Thread::new(session_id);
    thread.add_turn(Turn::new(TurnRole::User, prompt.into()));
    run_thread(config, &mut thread, timeout_secs).await
}

pub async fn run_thread(
    config: &Config,
    thread: &mut Thread,
    timeout_secs: u64,
) -> Result<AgentResult, AgentError> {
    let provider = Arc::new(CliProvider::new(CliProviderConfig {
        tool: config.primary_tool.clone(),
        working_dir: config.workspace_dir.clone(),
        timeout_secs,
        ..CliProviderConfig::default()
    }));

    let policy = ManagedToolSuppressionPolicy::from_env();
    let tools = build_default_tool_registry(config);
    let hooks = build_default_hook_system(&config.primary_tool);
    let agent_config = build_agent_config(timeout_secs);

    // Capture and log runtime diagnostics at launch time
    let active_count = tools.len();
    let diag = if let Some(ref p) = policy {
        RuntimeDiagnosticsSnapshot::capture(
            p.suppressed_tools.len(),
            p.analysis_preferred_tools.len(),
            p.memory_preferred_tools.len(),
            p.retained_maestro_tools.len(),
            active_count,
            &config.primary_tool,
        )
    } else {
        RuntimeDiagnosticsSnapshot::capture(0, 0, 0, 0, active_count, &config.primary_tool)
    };
    tracing::info!("runtime launch diagnostics: {}", diag.summary_line());

    agent_loop(thread, provider, tools, hooks, agent_config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_config_respects_read_only_mode() {
        let mut config = Config::default();
        config.autonomy.level = "read-only".into();

        let shell = build_shell_tool_config(&config);

        assert!(!shell.allow_moderate);
        assert!(!shell.allow_dangerous);
    }

    #[test]
    fn default_tool_registry_registers_runtime_tools() {
        let config = Config::default();
        let registry = build_default_tool_registry(&config);

        for tool in ["file", "shell", "cron_add", "cron_list", "cron_remove"] {
            assert!(registry.get(tool).is_some(), "missing tool {tool}");
        }
    }

    struct ExtraTool;

    #[async_trait::async_trait]
    impl Tool for ExtraTool {
        fn name(&self) -> &str {
            "extra"
        }

        fn description(&self) -> &str {
            "extra tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _arguments: serde_json::Value) -> crate::tools::ToolOutput {
            crate::tools::ToolOutput::success("ok".into())
        }
    }

    #[test]
    fn tool_registry_builder_supports_extra_tools() {
        let config = Config::default();
        #[cfg(feature = "core-integration")]
        let registry = build_default_tool_registry_with_extras(
            &config,
            vec![Arc::new(ExtraTool) as Arc<dyn Tool>],
            None,
        );
        #[cfg(not(feature = "core-integration"))]
        let registry = build_default_tool_registry_with_extras(
            &config,
            vec![Arc::new(ExtraTool) as Arc<dyn Tool>],
        );
        assert!(registry.get("extra").is_some());
    }

    #[test]
    fn default_tool_registry_respects_managed_suppression_policy() {
        let config = Config::default();
        let previous = std::env::var("MAESTRO_TOOL_SUPPRESSION_POLICY").ok();
        std::env::set_var(
            "MAESTRO_TOOL_SUPPRESSION_POLICY",
            serde_json::json!({
                "suppressed_tools": ["cron_add"],
                "analysis_preferred_tools": ["project_map"],
                "memory_preferred_tools": ["working_set"],
                "retained_maestro_tools": ["shell", "file", "cron_list", "cron_remove"]
            })
            .to_string(),
        );

        let registry = build_default_tool_registry(&config);
        assert!(registry.get("cron_add").is_none());
        assert!(registry.get("shell").is_some());

        match previous {
            Some(value) => std::env::set_var("MAESTRO_TOOL_SUPPRESSION_POLICY", value),
            None => std::env::remove_var("MAESTRO_TOOL_SUPPRESSION_POLICY"),
        }
    }

    #[test]
    fn autonomous_configs_enable_full_runtime_capabilities() {
        let mut config = Config::default();
        config.autonomy.level = "autonomous".into();

        let shell = build_shell_tool_config(&config);
        let file = build_file_tool_config(&config);

        assert!(shell.allow_dangerous);
        assert!(file.allow_delete);
        assert!(file.allow_write);
    }

    #[test]
    fn runtime_diagnostics_snapshot_captures_policy_state() {
        let diag = RuntimeDiagnosticsSnapshot::capture(1, 1, 1, 1, 4, "claude");

        assert!(diag.policy_loaded);
        assert_eq!(diag.suppressed_count, 1);
        assert_eq!(diag.analysis_preferred_count, 1);
        assert_eq!(diag.memory_preferred_count, 1);
        assert_eq!(diag.retained_count, 1);
        assert_eq!(diag.active_tool_count, 4);
        assert_eq!(diag.primary_tool, "claude");
        let summary = diag.summary_line();
        assert!(summary.contains("tool-policy:loaded"));
        assert!(summary.contains("suppressed:1"));
    }

    #[test]
    fn runtime_diagnostics_snapshot_handles_no_policy() {
        let diag = RuntimeDiagnosticsSnapshot::capture(0, 0, 0, 0, 5, "codex");

        assert!(!diag.policy_loaded);
        assert_eq!(diag.active_tool_count, 5);
        let summary = diag.summary_line();
        assert!(summary.contains("tool-policy:none"));
    }
}
