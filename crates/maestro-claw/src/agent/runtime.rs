use std::sync::Arc;

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

pub fn build_default_tools(config: &Config) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileTool::with_config(build_file_tool_config(config))),
        Arc::new(ShellTool::with_config(build_shell_tool_config(config))),
        Arc::new(CronAddTool::new(&config.workspace_dir)),
        Arc::new(CronListTool::new(&config.workspace_dir)),
        Arc::new(CronRemoveTool::new(&config.workspace_dir)),
    ]
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
    #[cfg(feature = "core-integration")]
    {
        return build_tool_registry(build_default_tools(config), None);
    }

    #[cfg(not(feature = "core-integration"))]
    {
        build_tool_registry(build_default_tools(config))
    }
}

#[cfg(feature = "core-integration")]
pub fn build_default_tool_registry_with_extras(
    config: &Config,
    extra_tools: impl IntoIterator<Item = Arc<dyn Tool>>,
    security_bridge: Option<SecurityPolicyBridge>,
) -> Arc<ToolRegistry> {
    build_tool_registry(
        build_default_tools(config).into_iter().chain(extra_tools),
        security_bridge,
    )
}

#[cfg(not(feature = "core-integration"))]
pub fn build_default_tool_registry_with_extras(
    config: &Config,
    extra_tools: impl IntoIterator<Item = Arc<dyn Tool>>,
) -> Arc<ToolRegistry> {
    build_tool_registry(build_default_tools(config).into_iter().chain(extra_tools))
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

    let tools = build_default_tool_registry(config);
    let hooks = build_default_hook_system(&config.primary_tool);
    let agent_config = build_agent_config(timeout_secs);

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
    fn autonomous_configs_enable_full_runtime_capabilities() {
        let mut config = Config::default();
        config.autonomy.level = "autonomous".into();

        let shell = build_shell_tool_config(&config);
        let file = build_file_tool_config(&config);

        assert!(shell.allow_dangerous);
        assert!(file.allow_delete);
        assert!(file.allow_write);
    }
}
