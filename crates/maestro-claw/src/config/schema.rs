//! Configuration schema types for MaestroClaw

use serde::{Deserialize, Serialize};

/// CLI agent tool configuration
/// MaestroClaw doesn't use API keys — these are locally-installed CLI tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolConfig {
    /// Tool name (e.g., "claude-code", "codex", "gemini-cli", "qwen-code", "iflow")
    pub name: String,
    /// Binary path (auto-detected if None)
    pub binary_path: Option<String>,
    /// Whether this tool is available on the system
    #[serde(default)]
    pub available: bool,
    /// Version string
    pub version: Option<String>,
    /// Tool-specific extra arguments
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Autonomy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyConfig {
    /// Autonomy level: "read-only", "supervised", "autonomous"
    #[serde(default = "default_autonomy_level")]
    pub level: String,
    /// Restrict file operations to workspace only
    #[serde(default = "default_true")]
    pub workspace_only: bool,
    /// Allowed shell commands
    #[serde(default = "default_allowed_commands")]
    pub allowed_commands: Vec<String>,
    /// Max actions per hour (rate limit)
    #[serde(default = "default_max_actions")]
    pub max_actions_per_hour: u32,
}

fn default_autonomy_level() -> String {
    "supervised".to_string()
}
fn default_true() -> bool {
    true
}
fn default_allowed_commands() -> Vec<String> {
    vec![
        "ls".into(),
        "pwd".into(),
        "cat".into(),
        "grep".into(),
        "rg".into(),
        "find".into(),
        "echo".into(),
        "head".into(),
        "tail".into(),
        "wc".into(),
        "sed".into(),
        "awk".into(),
        "git".into(),
        "cargo".into(),
        "npm".into(),
        "pnpm".into(),
        "yarn".into(),
        "mkdir".into(),
        "touch".into(),
        "cp".into(),
        "mv".into(),
        "which".into(),
        "python".into(),
        "python3".into(),
        "pytest".into(),
    ]
}
fn default_max_actions() -> u32 {
    100
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: default_autonomy_level(),
            workspace_only: true,
            allowed_commands: default_allowed_commands(),
            max_actions_per_hour: default_max_actions(),
        }
    }
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime kind: "native" or "docker"
    #[serde(default = "default_runtime_kind")]
    pub kind: String,
    /// Scheduler poll interval in seconds
    #[serde(default = "default_poll_secs")]
    pub scheduler_poll_secs: u64,
    /// Max retries for scheduler jobs
    #[serde(default = "default_retries")]
    pub scheduler_retries: u32,
    /// Provider backoff base in ms
    #[serde(default = "default_backoff_ms")]
    pub backoff_ms: u64,
}

fn default_runtime_kind() -> String {
    "native".to_string()
}
fn default_poll_secs() -> u64 {
    10
}
fn default_retries() -> u32 {
    2
}
fn default_backoff_ms() -> u64 {
    500
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            kind: default_runtime_kind(),
            scheduler_poll_secs: default_poll_secs(),
            scheduler_retries: default_retries(),
            backoff_ms: default_backoff_ms(),
        }
    }
}

/// Heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_heartbeat_interval")]
    pub interval_minutes: u32,
}

fn default_heartbeat_interval() -> u32 {
    15
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: default_heartbeat_interval(),
        }
    }
}

/// Cron/scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_run_history")]
    pub max_run_history: usize,
}

fn default_max_run_history() -> usize {
    50
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_run_history: default_max_run_history(),
        }
    }
}

/// Channels configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub matrix: Option<MatrixConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub mattermost: Option<MattermostConfig>,
    pub webhook: Option<WebhookConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub guild_id: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub homeserver_url: String,
    pub access_token: String,
    pub bot_user_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub room_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub bridge_url: String,
    pub api_token: String,
    pub phone_number_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostConfig {
    pub server_url: String,
    pub bot_token: String,
    pub team_id: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub secret: Option<String>,
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_backend")]
    pub backend: String,
    #[serde(default = "default_true")]
    pub auto_save: bool,
}

fn default_memory_backend() -> String {
    "sqlite".to_string()
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: default_memory_backend(),
            auto_save: true,
        }
    }
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_host")]
    pub host: String,
    #[serde(default = "default_gateway_port")]
    pub port: u16,
    /// API key for gateway authentication (generated during bootstrap if absent)
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_gateway_host() -> String {
    "127.0.0.1".to_string()
}
fn default_gateway_port() -> u16 {
    9800
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_gateway_host(),
            port: default_gateway_port(),
            api_key: None,
        }
    }
}

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_state_flush_secs")]
    pub state_flush_secs: u64,
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_secs: u64,
    #[serde(default = "default_max_backoff")]
    pub max_backoff_secs: u64,
}

fn default_state_flush_secs() -> u64 {
    5
}
fn default_initial_backoff() -> u64 {
    2
}
fn default_max_backoff() -> u64 {
    60
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            state_flush_secs: default_state_flush_secs(),
            initial_backoff_secs: default_initial_backoff(),
            max_backoff_secs: default_max_backoff(),
        }
    }
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    #[serde(default = "default_obs_backend")]
    pub backend: String,
}

fn default_obs_backend() -> String {
    "log".to_string()
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            backend: default_obs_backend(),
        }
    }
}

/// MaestroClaw-specific claw config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawConfig {
    /// The known CLI tool names that MaestroClaw can drive
    pub known_tools: Vec<String>,
}

impl Default for ClawConfig {
    fn default() -> Self {
        Self {
            known_tools: vec![
                "claude".into(),
                "codex".into(),
                "gemini".into(),
                "qwen".into(),
                "iflow".into(),
                "amp".into(),
                "droid".into(),
            ],
        }
    }
}

/// Metadata tracking bootstrap/setup completeness
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapMetadata {
    /// Whether the gateway API key was auto-generated during bootstrap
    #[serde(default)]
    pub gateway_api_key_generated: bool,
    /// Whether the webhook secret was auto-generated during bootstrap
    #[serde(default)]
    pub webhook_secret_generated: bool,
    /// Timestamp of initial setup
    #[serde(default)]
    pub setup_timestamp: Option<i64>,
    /// Version of MaestroClaw that performed the setup
    #[serde(default)]
    pub setup_version: Option<String>,
    /// Tracks which workspace scaffold items were created
    #[serde(default)]
    pub workspace_scaffold: WorkspaceScaffold,
    /// Hash of the generated webhook secret (for validation)
    #[serde(skip)]
    pub webhook_secret_hash: Option<String>,
}

/// Tracks workspace scaffolding completeness
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceScaffold {
    /// AGENTS.md created
    #[serde(default)]
    pub agents_md: bool,
    /// HEARTBEAT.md created
    #[serde(default)]
    pub heartbeat_md: bool,
    /// skills/ directory created
    #[serde(default)]
    pub skills_dir: bool,
    /// extensions/ directory created
    #[serde(default)]
    pub extensions_dir: bool,
    /// mcp/ directory created
    #[serde(default)]
    pub mcp_dir: bool,
    /// cost/ directory created
    #[serde(default)]
    pub cost_dir: bool,
    /// observability/ directory created
    #[serde(default)]
    pub observability_dir: bool,
    /// cron/ directory created
    #[serde(default)]
    pub cron_dir: bool,
    /// cron/jobs.toml created
    #[serde(default)]
    pub cron_jobs_file: bool,
    /// mcp/servers.toml created
    #[serde(default)]
    pub mcp_servers_file: bool,
}

impl WorkspaceScaffold {
    /// Returns true if all required scaffold items are present
    pub fn is_complete(&self) -> bool {
        self.agents_md
            && self.heartbeat_md
            && self.skills_dir
            && self.extensions_dir
            && self.mcp_dir
            && self.cost_dir
            && self.observability_dir
            && self.cron_dir
            && self.cron_jobs_file
            && self.mcp_servers_file
    }

    /// Returns the number of incomplete scaffold items
    pub fn incomplete_count(&self) -> usize {
        let mut count = 0;
        if !self.agents_md {
            count += 1;
        }
        if !self.heartbeat_md {
            count += 1;
        }
        if !self.skills_dir {
            count += 1;
        }
        if !self.extensions_dir {
            count += 1;
        }
        if !self.mcp_dir {
            count += 1;
        }
        if !self.cost_dir {
            count += 1;
        }
        if !self.observability_dir {
            count += 1;
        }
        if !self.cron_dir {
            count += 1;
        }
        if !self.cron_jobs_file {
            count += 1;
        }
        if !self.mcp_servers_file {
            count += 1;
        }
        count
    }

    /// Returns a list of missing scaffold items
    pub fn missing_items(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.agents_md {
            missing.push("AGENTS.md");
        }
        if !self.heartbeat_md {
            missing.push("HEARTBEAT.md");
        }
        if !self.skills_dir {
            missing.push("skills/");
        }
        if !self.extensions_dir {
            missing.push("extensions/");
        }
        if !self.mcp_dir {
            missing.push("mcp/");
        }
        if !self.cost_dir {
            missing.push("cost/");
        }
        if !self.observability_dir {
            missing.push("observability/");
        }
        if !self.cron_dir {
            missing.push("cron/");
        }
        if !self.cron_jobs_file {
            missing.push("cron/jobs.toml");
        }
        if !self.mcp_servers_file {
            missing.push("mcp/servers.toml");
        }
        missing
    }
}

/// Setup completeness status
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupStatus {
    pub config_present: bool,
    pub workspace_writable: bool,
    pub secrets_configured: bool,
    pub assets_scaffolded: bool,
    pub primary_tool_available: bool,
    /// Webhook secret is present and valid
    pub webhook_secret_configured: bool,
    /// Workspace scaffold is complete
    pub workspace_scaffold_complete: bool,
    /// Bootstrap metadata matches actual files
    pub bootstrap_state_valid: bool,
    /// Primary tool binary is in PATH
    pub primary_tool_in_path: bool,
    /// List of missing scaffold items (if any)
    pub missing_scaffold_items: Vec<String>,
    /// List of repair actions needed
    pub repair_actions: Vec<String>,
}

impl SetupStatus {
    /// Returns true if all setup requirements are met
    pub fn is_complete(&self) -> bool {
        self.config_present
            && self.workspace_writable
            && self.secrets_configured
            && self.assets_scaffolded
            && self.primary_tool_available
            && self.webhook_secret_configured
            && self.workspace_scaffold_complete
            && self.bootstrap_state_valid
            && self.primary_tool_in_path
    }

    /// Returns true if bootstrap-level setup is complete
    pub fn is_bootstrap_complete(&self) -> bool {
        self.config_present
            && self.workspace_writable
            && self.webhook_secret_configured
            && self.workspace_scaffold_complete
    }

    /// Returns the number of incomplete items
    pub fn incomplete_count(&self) -> usize {
        let mut count = 0;
        if !self.config_present {
            count += 1;
        }
        if !self.workspace_writable {
            count += 1;
        }
        if !self.secrets_configured {
            count += 1;
        }
        if !self.assets_scaffolded {
            count += 1;
        }
        if !self.primary_tool_available {
            count += 1;
        }
        if !self.webhook_secret_configured {
            count += 1;
        }
        if !self.workspace_scaffold_complete {
            count += 1;
        }
        if !self.bootstrap_state_valid {
            count += 1;
        }
        if !self.primary_tool_in_path {
            count += 1;
        }
        count
    }

    /// Returns the number of bootstrap-level incomplete items
    pub fn bootstrap_incomplete_count(&self) -> usize {
        let mut count = 0;
        if !self.config_present {
            count += 1;
        }
        if !self.workspace_writable {
            count += 1;
        }
        if !self.webhook_secret_configured {
            count += 1;
        }
        if !self.workspace_scaffold_complete {
            count += 1;
        }
        count
    }

    /// Returns true if any repair actions are needed
    pub fn needs_repair(&self) -> bool {
        !self.repair_actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_scaffold_requires_runtime_files() {
        let scaffold = WorkspaceScaffold {
            agents_md: true,
            heartbeat_md: true,
            skills_dir: true,
            extensions_dir: true,
            mcp_dir: true,
            cost_dir: true,
            observability_dir: true,
            cron_dir: true,
            cron_jobs_file: false,
            mcp_servers_file: true,
        };

        assert!(!scaffold.is_complete());
        assert!(scaffold.missing_items().contains(&"cron/jobs.toml"));
    }

    #[test]
    fn setup_status_counts_primary_tool_path_requirement() {
        let status = SetupStatus {
            config_present: true,
            workspace_writable: true,
            secrets_configured: true,
            assets_scaffolded: true,
            primary_tool_available: true,
            webhook_secret_configured: true,
            workspace_scaffold_complete: true,
            bootstrap_state_valid: true,
            primary_tool_in_path: false,
            missing_scaffold_items: Vec::new(),
            repair_actions: Vec::new(),
        };

        assert!(!status.is_complete());
        assert_eq!(status.incomplete_count(), 1);
    }
}
