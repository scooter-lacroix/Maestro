//! MaestroClaw configuration
//!
//! Lightweight config for CLI-tool-based agent integration.
//! No API keys needed — tools are locally installed binaries.

pub mod schema;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Per-tool configuration for a CLI coding agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolConfig {
    pub name: String,
    pub binary_path: Option<String>,
    #[serde(default)]
    pub available: bool,
    pub version: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_host")]
    pub host: String,
    #[serde(default = "default_gateway_port")]
    pub port: u16,
    /// API key for gateway authentication
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

/// Top-level MaestroClaw configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the config file on disk
    #[serde(skip)]
    pub config_path: PathBuf,

    /// Working directory for MaestroClaw sessions
    #[serde(skip)]
    pub workspace_dir: PathBuf,

    /// Name of the primary CLI tool (e.g. "claude", "codex")
    pub primary_tool: String,

    /// All known agent tools and their status
    #[serde(default)]
    pub agent_tools: Vec<AgentToolConfig>,

    /// Gateway settings
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Daemon settings
    #[serde(default)]
    pub daemon: schema::DaemonConfig,

    /// Runtime settings
    #[serde(default)]
    pub runtime: schema::RuntimeConfig,

    /// Autonomy and tool permission settings
    #[serde(default)]
    pub autonomy: schema::AutonomyConfig,

    /// Cron/scheduler settings
    #[serde(default)]
    pub cron: schema::CronConfig,

    /// Channel transport settings
    #[serde(default)]
    pub channels: schema::ChannelsConfig,

    /// Heartbeat task settings
    #[serde(default)]
    pub heartbeat: schema::HeartbeatConfig,

    /// Observability backend settings
    #[serde(default)]
    pub observability: schema::ObservabilityConfig,

    /// Bootstrap metadata tracking setup completeness
    #[serde(default)]
    pub bootstrap: schema::BootstrapMetadata,

    /// Setup completeness status (computed, not persisted)
    #[serde(skip)]
    pub setup_status: schema::SetupStatus,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home.join(".config").join("maestroclaw");
        Self {
            config_path: config_dir.join("config.toml"),
            workspace_dir: config_dir.join("workspace"),
            primary_tool: "claude".into(),
            agent_tools: Vec::new(),
            gateway: GatewayConfig::default(),
            daemon: schema::DaemonConfig::default(),
            runtime: schema::RuntimeConfig::default(),
            autonomy: schema::AutonomyConfig::default(),
            cron: schema::CronConfig::default(),
            channels: schema::ChannelsConfig::default(),
            heartbeat: schema::HeartbeatConfig::default(),
            observability: schema::ObservabilityConfig::default(),
            bootstrap: schema::BootstrapMetadata::default(),
            setup_status: schema::SetupStatus::default(),
        }
    }
}

impl Config {
    /// Load config from the default path, falling back to defaults.
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        if config.config_path.exists() {
            let text = std::fs::read_to_string(&config.config_path)
                .with_context(|| format!("reading {}", config.config_path.display()))?;
            let loaded: Config = toml::from_str(&text)
                .with_context(|| format!("parsing {}", config.config_path.display()))?;
            let config_path = config.config_path.clone();
            let workspace_dir = config.workspace_dir.clone();
            config = loaded;
            config.config_path = config_path;
            config.workspace_dir = workspace_dir;
        }
        config.setup_status = config.compute_setup_status();
        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&self.config_path, text)
            .with_context(|| format!("writing {}", self.config_path.display()))?;
        Ok(())
    }

    /// Compute the current setup status based on config and filesystem state.
    pub fn compute_setup_status(&self) -> schema::SetupStatus {
        use crate::onboard::assets_present;

        let webhook_secret_configured = self.has_webhook_secret();
        let workspace_scaffold_complete = self.bootstrap.workspace_scaffold.is_complete();
        let missing_scaffold_items = self
            .bootstrap
            .workspace_scaffold
            .missing_items()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let (bootstrap_state_valid, bootstrap_mismatches) = self.validate_bootstrap_state();
        let primary_tool_in_path = self.primary_tool_path().is_some();

        let mut repair_actions = Vec::new();
        if !self.config_path.exists() {
            repair_actions.push(format!(
                "Run `maestro claw setup` to create {}",
                self.config_path.display()
            ));
        }
        if !workspace_is_writable(&self.workspace_dir) {
            repair_actions.push(format!(
                "Ensure the workspace path is writable: {}",
                self.workspace_dir.display()
            ));
        }
        if self.gateway.api_key.as_deref().unwrap_or("").is_empty() {
            repair_actions.push("Generate or set gateway.api_key via `maestro claw setup`".into());
        } else if !self.gateway_api_key_is_strong() {
            repair_actions.push(
                "Rotate gateway.api_key to a strong `mcw_...` value via `maestro claw setup`"
                    .into(),
            );
        }
        if !webhook_secret_configured {
            repair_actions
                .push("Generate or set channels.webhook.secret via `maestro claw setup`".into());
        } else if !self.webhook_secret_is_strong() {
            repair_actions.push(
                "Rotate channels.webhook.secret to a value with at least 32 characters".into(),
            );
        }
        if !workspace_scaffold_complete {
            repair_actions.push(format!(
                "Scaffold missing workspace assets: {}",
                missing_scaffold_items.join(", ")
            ));
        }
        if !self.bootstrap.workspace_scaffold.cron_jobs_file {
            repair_actions.push("Create cron/jobs.toml for scheduled task configuration".into());
        }
        if !self.bootstrap.workspace_scaffold.mcp_servers_file {
            repair_actions
                .push("Create mcp/servers.toml for managed MCP server configuration".into());
        }
        if !bootstrap_state_valid && !bootstrap_mismatches.is_empty() {
            repair_actions.push(format!(
                "Repair bootstrap metadata or recreate missing assets: {}",
                bootstrap_mismatches.join(", ")
            ));
        }
        if self.bootstrap.setup_timestamp.is_none() || self.bootstrap.setup_version.is_none() {
            repair_actions.push(
                "Rerun `maestro claw setup` to record bootstrap timestamp/version metadata".into(),
            );
        }
        if !self.is_primary_tool_available() {
            repair_actions.push(format!(
                "Install or select a different primary tool (`{}` is unavailable)",
                self.primary_tool
            ));
        } else if !primary_tool_in_path {
            repair_actions.push(format!(
                "Ensure `{}` is available in PATH",
                self.primary_tool
            ));
        }

        schema::SetupStatus {
            config_present: self.config_path.exists(),
            workspace_writable: workspace_is_writable(&self.workspace_dir),
            secrets_configured: self.are_secrets_configured(),
            assets_scaffolded: assets_present(&self.workspace_dir),
            primary_tool_available: self.is_primary_tool_available(),
            webhook_secret_configured,
            workspace_scaffold_complete,
            bootstrap_state_valid,
            primary_tool_in_path,
            missing_scaffold_items,
            repair_actions,
        }
    }

    /// Check if secrets are properly configured.
    pub fn are_secrets_configured(&self) -> bool {
        self.gateway_api_key_is_strong() && self.webhook_secret_is_strong()
    }

    /// Check if the primary tool is available on the system.
    pub fn is_primary_tool_available(&self) -> bool {
        self.agent_tools
            .iter()
            .find(|t| t.name == self.primary_tool)
            .map(|t| t.available)
            .unwrap_or(false)
    }

    fn primary_tool_path(&self) -> Option<String> {
        self.agent_tools
            .iter()
            .find(|tool| tool.name == self.primary_tool)
            .and_then(|tool| tool.binary_path.clone())
    }

    /// Get a display-ready summary of setup status.
    pub fn setup_status_summary(&self) -> String {
        let status = &self.setup_status;
        let items = vec![
            ("Config file", status.config_present),
            ("Workspace writable", status.workspace_writable),
            ("Secrets configured", status.secrets_configured),
            ("Assets scaffolded", status.assets_scaffolded),
            ("Primary tool available", status.primary_tool_available),
            (
                "Webhook secret configured",
                status.webhook_secret_configured,
            ),
            (
                "Scaffold metadata complete",
                status.workspace_scaffold_complete,
            ),
            ("Bootstrap metadata valid", status.bootstrap_state_valid),
            ("Primary tool in PATH", status.primary_tool_in_path),
        ];

        let mut summary = String::new();
        for (name, ok) in items {
            let icon = if ok { "✅" } else { "❌" };
            summary.push_str(&format!("  {} {}\n", icon, name));
        }

        if status.is_complete() {
            summary.push_str("\n  🎉 Setup is complete!");
        } else {
            summary.push_str(&format!(
                "\n  ⚠️  Setup incomplete: {} item(s) missing. Run `maestro claw doctor` for guidance.",
                status.incomplete_count()
            ));
            if !status.repair_actions.is_empty() {
                summary.push_str("\n  Suggested repairs:");
                for action in &status.repair_actions {
                    summary.push_str(&format!("\n    - {}", action));
                }
            }
        }

        summary
    }
}

/// Check if the workspace directory is writable.
fn workspace_is_writable(workspace_dir: &PathBuf) -> bool {
    if !workspace_dir.exists() {
        // Parent directory must be writable to create workspace
        return workspace_dir
            .parent()
            .map(|p| {
                let probe = p.join(format!(".maestroclaw_probe_{}", uuid::Uuid::new_v4()));
                std::fs::write(&probe, b"probe").is_ok() && std::fs::remove_file(&probe).is_ok()
            })
            .unwrap_or(false);
    }

    let probe = workspace_dir.join(format!(".maestroclaw_probe_{}", uuid::Uuid::new_v4()));
    std::fs::write(&probe, b"probe").is_ok() && std::fs::remove_file(&probe).is_ok()
}

/// Generate a cryptographically secure random secret
pub fn generate_secret(length: usize) -> String {
    let mut secret = String::new();
    while secret.len() < length {
        secret.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    secret.truncate(length);
    secret
}

/// Generate a webhook secret (32 bytes = 256 bits of entropy)
pub fn generate_webhook_secret() -> String {
    generate_secret(32)
}

/// Simple hash for secret validation (not for storage)
pub fn hash_secret(secret: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    secret.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

impl Config {
    /// Ensure webhook secret exists and is strong.
    /// Returns true if a new secret was generated or rotated.
    pub fn ensure_webhook_secret(&mut self) -> bool {
        let needs_secret = !self.webhook_secret_is_strong();

        if needs_secret {
            let secret = generate_webhook_secret();
            self.channels.webhook = Some(schema::WebhookConfig {
                secret: Some(secret),
            });
            self.bootstrap.webhook_secret_generated = true;
            true
        } else {
            false
        }
    }

    /// Get the webhook secret if configured
    pub fn webhook_secret(&self) -> Option<&str> {
        self.channels
            .webhook
            .as_ref()
            .and_then(|w| w.secret.as_deref())
    }

    /// Check if webhook secret is properly configured
    pub fn has_webhook_secret(&self) -> bool {
        self.webhook_secret()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    }

    /// Check if the configured gateway API key meets the expected bootstrap format.
    pub fn gateway_api_key_is_strong(&self) -> bool {
        self.gateway
            .api_key
            .as_deref()
            .map(|key| key.starts_with("mcw_") && key.len() >= 36)
            .unwrap_or(false)
    }

    /// Check if the configured webhook secret meets the minimum strength requirement.
    pub fn webhook_secret_is_strong(&self) -> bool {
        self.webhook_secret()
            .map(|secret| secret.len() >= 32)
            .unwrap_or(false)
    }

    /// Validate that the current bootstrap state matches actual files
    pub fn validate_bootstrap_state(&self) -> (bool, Vec<String>) {
        let mut valid = true;
        let mut mismatches = Vec::new();

        // Check workspace scaffold files
        let ws = &self.bootstrap.workspace_scaffold;

        if ws.agents_md {
            let path = self.workspace_dir.join("AGENTS.md");
            if !path.exists() {
                valid = false;
                mismatches.push("AGENTS.md recorded but missing".to_string());
            }
        }

        if ws.heartbeat_md {
            let path = self.workspace_dir.join("HEARTBEAT.md");
            if !path.exists() {
                valid = false;
                mismatches.push("HEARTBEAT.md recorded but missing".to_string());
            }
        }

        if ws.skills_dir {
            let path = self.workspace_dir.join("skills");
            if !path.exists() {
                valid = false;
                mismatches.push("skills/ directory recorded but missing".to_string());
            }
        }

        if ws.extensions_dir {
            let path = self.workspace_dir.join("extensions");
            if !path.exists() {
                valid = false;
                mismatches.push("extensions/ directory recorded but missing".to_string());
            }
        }

        if ws.mcp_dir {
            let path = self.workspace_dir.join("mcp");
            if !path.exists() {
                valid = false;
                mismatches.push("mcp/ directory recorded but missing".to_string());
            }
        }

        if ws.cost_dir {
            let path = self.workspace_dir.join("cost");
            if !path.exists() {
                valid = false;
                mismatches.push("cost/ directory recorded but missing".to_string());
            }
        }

        if ws.observability_dir {
            let path = self.workspace_dir.join("observability");
            if !path.exists() {
                valid = false;
                mismatches.push("observability/ directory recorded but missing".to_string());
            }
        }

        if ws.cron_dir {
            let path = self.workspace_dir.join("cron");
            if !path.exists() {
                valid = false;
                mismatches.push("cron/ directory recorded but missing".to_string());
            }
        }

        if ws.cron_jobs_file {
            let path = self.workspace_dir.join("cron").join("jobs.toml");
            if !path.exists() {
                valid = false;
                mismatches.push("cron/jobs.toml recorded but missing".to_string());
            }
        }

        if ws.mcp_servers_file {
            let path = self.workspace_dir.join("mcp").join("servers.toml");
            if !path.exists() {
                valid = false;
                mismatches.push("mcp/servers.toml recorded but missing".to_string());
            }
        }

        (valid, mismatches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_values() {
        let config = Config::default();
        assert_eq!(config.primary_tool, "claude");
        assert_eq!(config.gateway.host, "127.0.0.1");
        assert_eq!(config.gateway.port, 9800);
        assert_eq!(config.autonomy.level, "supervised");
    }

    #[test]
    fn generate_secret_produces_expected_length() {
        let secret = generate_secret(32);
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_webhook_secret_is_32_chars() {
        let secret = generate_webhook_secret();
        assert_eq!(secret.len(), 32);
    }

    #[test]
    fn hash_secret_is_deterministic() {
        let secret = "test-secret-123";
        let hash1 = hash_secret(secret);
        let hash2 = hash_secret(secret);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_secret_produces_different_hashes() {
        let hash1 = hash_secret("secret1");
        let hash2 = hash_secret("secret2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn config_ensure_webhook_secret_generates_when_absent() {
        let mut config = Config::default();
        assert!(config.channels.webhook.is_none());

        let generated = config.ensure_webhook_secret();
        assert!(generated);
        assert!(config.has_webhook_secret());
        assert!(config.bootstrap.webhook_secret_generated);
        assert_eq!(config.webhook_secret().unwrap().len(), 32);
    }

    #[test]
    fn config_ensure_webhook_secret_preserves_existing() {
        let mut config = Config::default();
        config.channels.webhook = Some(schema::WebhookConfig {
            secret: Some("existing-secret-1234567890-abcdef".to_string()),
        });

        let generated = config.ensure_webhook_secret();
        assert!(!generated);
        assert_eq!(
            config.webhook_secret(),
            Some("existing-secret-1234567890-abcdef")
        );
    }

    #[test]
    fn config_ensure_webhook_secret_rotates_weak_existing_secret() {
        let mut config = Config::default();
        config.channels.webhook = Some(schema::WebhookConfig {
            secret: Some("short".to_string()),
        });

        let generated = config.ensure_webhook_secret();
        assert!(generated);
        assert!(config.webhook_secret_is_strong());
    }

    #[test]
    fn config_validate_bootstrap_state_detects_mismatches() {
        let mut config = Config::default();
        // Use a temp directory that doesn't exist
        config.workspace_dir = PathBuf::from("/nonexistent/workspace");
        config.bootstrap.workspace_scaffold.agents_md = true;
        config.bootstrap.workspace_scaffold.heartbeat_md = true;
        config.bootstrap.workspace_scaffold.skills_dir = true;
        config.bootstrap.workspace_scaffold.cron_jobs_file = true;

        let (valid, mismatches) = config.validate_bootstrap_state();
        assert!(!valid);
        assert!(!mismatches.is_empty());
        assert!(mismatches.iter().any(|m| m.contains("AGENTS.md")));
        assert!(mismatches.iter().any(|m| m.contains("cron/jobs.toml")));
    }

    #[test]
    fn secret_strength_checks_match_bootstrap_expectations() {
        let mut config = Config::default();
        config.gateway.api_key = Some("mcw_12345678901234567890123456789012".to_string());
        config.channels.webhook = Some(schema::WebhookConfig {
            secret: Some("12345678901234567890123456789012".to_string()),
        });

        assert!(config.gateway_api_key_is_strong());
        assert!(config.webhook_secret_is_strong());

        config.gateway.api_key = Some("weak-key".to_string());
        config.channels.webhook = Some(schema::WebhookConfig {
            secret: Some("short".to_string()),
        });

        assert!(!config.gateway_api_key_is_strong());
        assert!(!config.webhook_secret_is_strong());
        assert!(!config.are_secrets_configured());
    }
}
