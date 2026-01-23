//! Maestro Integration Manager
//!
//! Provides first-class integration with multiple AI coding tools:
//! - Claude Code, OpenCode, Codex CLI, Gemini CLI, Qwen Code, Amp CLI, Droid CLI
//!
//! Usage:
//!   maestro integrate install <tool>     # Install integration for a tool
//!   maestro integrate install --all      # Install all integrations
//!   maestro integrate uninstall <tool>   # Remove integration
//!   maestro integrate doctor <tool>      # Validate integration
//!   maestro integrate print <tool>       # Emit config patches

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use serde_json::{json, Map as JsonMap, Value};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use toml::{map::Map as TomlMap, Value as TomlValue};

/// Available integration targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum IntegrationTool {
    /// Anthropic Claude Code
    Claude,
    /// OpenCode
    OpenCode,
    /// OpenAI Codex CLI
    Codex,
    /// Google Gemini CLI
    Gemini,
    /// Qwen Code
    Qwen,
    /// Sourcegraph Amp CLI
    Amp,
    /// Factory Droid CLI
    Droid,
}

impl IntegrationTool {
    /// Get all tools as a slice
    pub fn all() -> &'static [IntegrationTool] {
        &[
            IntegrationTool::Claude,
            IntegrationTool::OpenCode,
            IntegrationTool::Codex,
            IntegrationTool::Gemini,
            IntegrationTool::Qwen,
            IntegrationTool::Amp,
            IntegrationTool::Droid,
        ]
    }

    /// Get the config directory for this tool
    pub fn config_dir(&self) -> Option<Utf8PathBuf> {
        let home = dirs::home_dir()?;
        let home_str = home.to_str()?;

        match self {
            IntegrationTool::Claude => Some(Utf8PathBuf::from(format!("{}/.claude", home_str))),
            IntegrationTool::OpenCode => Some(Utf8PathBuf::from(format!("{}/.config/opencode", home_str))),
            IntegrationTool::Codex => {
                let codex_home = std::env::var("CODEX_HOME").ok();
                Some(Utf8PathBuf::from(codex_home.unwrap_or_else(|| format!("{}/.codex", home_str))))
            }
            IntegrationTool::Gemini => Some(Utf8PathBuf::from(format!("{}/.gemini", home_str))),
            IntegrationTool::Qwen => Some(Utf8PathBuf::from(format!("{}/.qwen", home_str))),
            IntegrationTool::Amp => Some(Utf8PathBuf::from(format!("{}/.config/amp", home_str))),
            IntegrationTool::Droid => Some(Utf8PathBuf::from(format!("{}/.factory", home_str))),
        }
    }

    /// Get the command/prompt directory for this tool
    pub fn commands_dir(&self) -> Option<Utf8PathBuf> {
        let config = self.config_dir()?;
        match self {
            IntegrationTool::Claude => Some(config.join("commands")),
            IntegrationTool::OpenCode => Some(config.join("commands")),
            IntegrationTool::Codex => Some(config.join("prompts")),
            IntegrationTool::Gemini => Some(config.join("commands/maestro")),
            IntegrationTool::Qwen => Some(config.join("commands/maestro")),
            IntegrationTool::Amp | IntegrationTool::Droid => None, // MCP-only
        }
    }

    /// Get the MCP config file path for this tool
    pub fn mcp_config_path(&self) -> Option<Utf8PathBuf> {
        let config = self.config_dir()?;
        match self {
            IntegrationTool::Claude => Some(config.join(".mcp.json")),
            IntegrationTool::OpenCode => Some(config.join("opencode.json")),
            IntegrationTool::Codex => Some(config.join("config.toml")),
            IntegrationTool::Gemini => Some(config.join("settings.json")),
            IntegrationTool::Qwen => Some(config.join("settings.json")),
            IntegrationTool::Amp => Some(config.join("settings.json")),
            IntegrationTool::Droid => Some(config.join("mcp.json")),
        }
    }

    /// Check if this tool uses JSON config
    pub fn uses_json_config(&self) -> bool {
        !matches!(self, IntegrationTool::Codex)
    }

    /// Get the MCP server name for LeIndex
    pub fn mcp_server_name(&self) -> &str {
        "leindex"
    }

    /// Get the LeIndex MCP server config
    pub fn leindex_mcp_config(&self) -> Value {
        let command = if cfg!(target_os = "windows") {
            "maestro.exe"
        } else {
            "maestro"
        };

        match self {
            IntegrationTool::Claude | IntegrationTool::Gemini | IntegrationTool::Qwen => json!({
                "command": command,
                "args": ["mcp", "proxy", "leindex"]
            }),
            IntegrationTool::OpenCode => json!({
                "type": "local",
                "command": [command, "mcp", "proxy", "leindex"]
            }),
            IntegrationTool::Amp => json!({
                "command": command,
                "args": ["mcp", "proxy", "leindex"],
                "env": {}
            }),
            IntegrationTool::Droid => json!({
                "type": "stdio",
                "command": command,
                "args": ["mcp", "proxy", "leindex"]
            }),
            IntegrationTool::Codex => {
                // Codex uses TOML, represented as JSON here for consistency
                json!({
                    "command": command,
                    "args": ["mcp", "proxy", "leindex"]
                })
            }
        }
    }

    /// Get the MCP config path for this tool
    pub fn mcp_config_json_path(&self) -> Option<&str> {
        match self {
            IntegrationTool::Claude => Some("mcpServers"),
            IntegrationTool::OpenCode => Some("mcp"),
            IntegrationTool::Gemini | IntegrationTool::Qwen => Some("mcpServers"),
            IntegrationTool::Amp => Some("amp.mcpServers"),
            IntegrationTool::Droid => Some("mcpServers"),
            IntegrationTool::Codex => None, // Uses TOML
        }
    }
}

/// Integration manager
pub struct Integrator {
    dry_run: bool,
    verbose: bool,
}

impl Integrator {
    /// Create a new integrator
    pub fn new(dry_run: bool, verbose: bool) -> Self {
        Self { dry_run, verbose }
    }

    /// Log a message if verbose
    fn log(&self, msg: &str) {
        if self.verbose {
            eprintln!("[integrate] {}", msg);
        }
    }

    /// Create a backup of a file
    fn backup_file(&self, path: &Path) -> Result<Utf8PathBuf> {
        if !path.exists() {
            let path_str = path.to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path"))?;
            return Ok(Utf8PathBuf::from(path_str));
        }

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let backup_path = format!("{}.backup.{}", path.display(), timestamp);
        fs::copy(path, &backup_path)
            .with_context(|| format!("Failed to backup {}", path.display()))?;

        self.log(&format!("Backed up {} to {}", path.display(), backup_path));
        Ok(Utf8PathBuf::from(backup_path))
    }

    /// Merge MCP server config into existing config
    fn merge_mcp_config(
        &self,
        tool: IntegrationTool,
        existing: &Value,
    ) -> Result<Value> {
        let server_name = tool.mcp_server_name();
        let new_config = tool.leindex_mcp_config();
        let config_path = tool.mcp_config_json_path()
            .ok_or_else(|| anyhow::anyhow!("No MCP config path for {:?}", tool))?;

        // Navigate to the MCP config location using a recursive approach
        fn insert_at_path(mut value: Value, parts: &[&str], key: &str, config: Value) -> Value {
            if parts.is_empty() {
                // We're at the final location - insert the config
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(key.to_string(), config);
                } else {
                    value = json!({});
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(key.to_string(), config);
                    }
                }
                return value;
            }

            // Ensure we have an object at this level
            if !value.is_object() {
                value = json!({});
            }

            // Get the next part
            let part = &parts[0];
            let remaining = &parts[1..];

            // Get or create the next level
            let next_value = if let Some(obj) = value.as_object() {
                obj.get(*part).cloned().unwrap_or(json!({}))
            } else {
                json!({})
            };

            // Recursively process the next level
            let updated_next = insert_at_path(next_value, remaining, key, config);

            // Reconstruct this level with the updated next level
            if let Some(obj) = value.as_object_mut() {
                obj.insert(part.to_string(), updated_next);
            }

            value
        }

        let parts: Vec<&str> = config_path.split('.').collect();
        let result = insert_at_path(existing.clone(), &parts, server_name, new_config);
        Ok(result)
    }

    /// Update a JSON config file with MCP server config
    fn update_json_config(
        &self,
        tool: IntegrationTool,
        config_path: &Path,
    ) -> Result<()> {
        if !config_path.exists() {
            self.log(&format!("Config file {} does not exist, creating new", config_path.display()));
            let server_name = tool.mcp_server_name();
            let new_config = tool.leindex_mcp_config();
            let config_path_str = tool.mcp_config_json_path()
                .ok_or_else(|| anyhow::anyhow!("No MCP config path for {:?}", tool))?;

            // Use the same recursive insertion logic as merge_mcp_config
            let config_obj = if config_path_str.contains('.') {
                // Build the nested structure from scratch
                let parts: Vec<&str> = config_path_str.split('.').collect();
                let mut result = json!({});

                fn insert_new_at_path(mut value: Value, parts: &[&str], key: &str, config: Value) -> Value {
                    if parts.is_empty() {
                        if let Some(obj) = value.as_object_mut() {
                            obj.insert(key.to_string(), config);
                        } else {
                            value = json!({});
                            if let Some(obj) = value.as_object_mut() {
                                obj.insert(key.to_string(), config);
                            }
                        }
                        return value;
                    }

                    if !value.is_object() {
                        value = json!({});
                    }

                    let part = &parts[0];
                    let remaining = &parts[1..];

                    let next_value = if let Some(obj) = value.as_object() {
                        obj.get(*part).cloned().unwrap_or(json!({}))
                    } else {
                        json!({})
                    };

                    let updated_next = insert_new_at_path(next_value, remaining, key, config);

                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(part.to_string(), updated_next);
                    }

                    value
                }

                insert_new_at_path(result, &parts, server_name, new_config)
            } else {
                // Simple case - no nesting needed
                json!({ server_name: new_config })
            };

            if !self.dry_run {
                fs::write(
                    config_path,
                    serde_json::to_string_pretty(&config_obj)?,
                )?;
            }
            return Ok(());
        }

        self.backup_file(config_path)?;

        let existing = fs::read_to_string(config_path)?;
        let existing_json: Value = serde_json::from_str(&existing)
            .with_context(|| format!("Invalid JSON in {}", config_path.display()))?;

        let updated = self.merge_mcp_config(tool, &existing_json)?;

        if !self.dry_run {
            fs::write(
                config_path,
                serde_json::to_string_pretty(&updated)?,
            )?;
        }

        self.log(&format!("Updated MCP config in {}", config_path.display()));
        Ok(())
    }

    /// Update a TOML config file with MCP server config
    fn update_toml_config(
        &self,
        tool: IntegrationTool,
        config_path: &Path,
    ) -> Result<()> {
        if !config_path.exists() {
            self.log(&format!("Config file {} does not exist, creating new", config_path.display()));

            // Create basic TOML structure
            let toml_content = format!(
                r#"[mcp_servers.{}]
command = "{}"
args = ["mcp", "proxy", "{}"]
"#,
                tool.mcp_server_name(),
                if cfg!(target_os = "windows") {
                    "maestro.exe"
                } else {
                    "maestro"
                },
                tool.mcp_server_name()
            );

            if !self.dry_run {
                fs::write(config_path, toml_content)?;
            }
            return Ok(());
        }

        self.backup_file(config_path)?;

        let existing = fs::read_to_string(config_path)?;
        let mut toml_val: TomlValue = existing.parse()
            .with_context(|| format!("Invalid TOML in {}", config_path.display()))?;

        // Build the TOML structure for the MCP server
        let server_name = tool.mcp_server_name();
        let command = if cfg!(target_os = "windows") {
            "maestro.exe"
        } else {
            "maestro"
        };

        // Ensure mcp_servers table exists
        if toml_val.is_table() {
            let table = toml_val.as_table_mut().unwrap();
            if !table.contains_key("mcp_servers") {
                table.insert("mcp_servers".to_string(), TomlValue::Table(TomlMap::new()));
            }

            if let Some(mcp_servers) = table.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) {
                let mut server_table = TomlMap::new();
                server_table.insert("command".to_string(), TomlValue::String(command.to_string()));
                server_table.insert(
                    "args".to_string(),
                    TomlValue::Array(vec![
                        TomlValue::String("mcp".to_string()),
                        TomlValue::String("proxy".to_string()),
                        TomlValue::String(server_name.to_string()),
                    ]),
                );

                mcp_servers.insert(server_name.to_string(), TomlValue::Table(server_table));
            }
        }

        if !self.dry_run {
            fs::write(config_path, toml_val.to_string())?;
        }

        self.log(&format!("Updated TOML config in {}", config_path.display()));
        Ok(())
    }

    /// Ensure directory exists
    fn ensure_dir(&self, path: &Path) -> Result<()> {
        if !path.exists() && !self.dry_run {
            fs::create_dir_all(path)
                .with_context(|| format!("Failed to create directory {}", path.display()))?;
        }
        self.log(&format!("Ensured directory exists: {}", path.display()));
        Ok(())
    }

    /// Get canonical command list for Maestro
    pub fn canonical_commands() -> &'static [&'static str] {
        &[
            "setup",
            "newTrack",
            "implement",
            "orchestrate",
            "status",
            "revert",
            "configure",
            "leindex",
            "tui",
            "memory",
        ]
    }

    /// Install integration for a tool
    pub fn install(&self, tool: IntegrationTool) -> Result<()> {
        self.log(&format!("Installing integration for {:?}", tool));

        // 1. Install command/prompt artifacts if applicable
        if let Some(commands_dir) = tool.commands_dir() {
            self.ensure_dir(commands_dir.as_std_path())?;

            // For Claude Code, install command files
            if matches!(tool, IntegrationTool::Claude) {
                self.install_claude_commands(commands_dir.as_std_path())?;
            }
            // For OpenCode, ensure skill and command files
            else if matches!(tool, IntegrationTool::OpenCode) {
                self.install_opencode_artifacts(tool)?;
            }
            // For Gemini/Qwen, install TOML command files
            else if matches!(tool, IntegrationTool::Gemini | IntegrationTool::Qwen) {
                self.install_toml_commands(tool, commands_dir.as_std_path())?;
            }
            // For Codex, install prompt files
            else if matches!(tool, IntegrationTool::Codex) {
                self.install_codex_prompts(commands_dir.as_std_path())?;
            }
        }

        // 2. Update MCP config
        if let Some(mcp_config) = tool.mcp_config_path() {
            self.ensure_dir(mcp_config.as_std_path().parent().unwrap())?;

            if tool.uses_json_config() {
                self.update_json_config(tool, mcp_config.as_std_path())?;
            } else {
                self.update_toml_config(tool, mcp_config.as_std_path())?;
            }
        }

        // 3. Special handling for OpenCode: update opencode.json command entries
        if matches!(tool, IntegrationTool::OpenCode) {
            self.update_opencode_json_commands(tool)?;
        }

        eprintln!("Installed integration for {:?}", tool);
        Ok(())
    }

    /// Install Claude Code command files
    fn install_claude_commands(&self, commands_dir: &Path) -> Result<()> {
        self.log("Installing Claude Code command files");

        // Command files would be installed from a canonical location
        // For now, we create placeholder structure
        for cmd in Self::canonical_commands() {
            let cmd_file = commands_dir.join(format!("maestro:{}", cmd));
            if !cmd_file.exists() && !self.dry_run {
                fs::write(
                    &cmd_file,
                    format!(
                        "# Maestro /{} command\n\
                        \n\
                        This is a placeholder. The actual command protocol\n\
                        should be loaded from the canonical source.\n",
                        cmd
                    ),
                )?;
            }
        }

        Ok(())
    }

    /// Install OpenCode artifacts (skill + commands)
    fn install_opencode_artifacts(&self, tool: IntegrationTool) -> Result<()> {
        self.log("Installing OpenCode artifacts");

        let config_dir = tool.config_dir()
            .ok_or_else(|| anyhow::anyhow!("No config dir for OpenCode"))?;

        // Skill directory
        let skill_dir = config_dir.join("skill/maestro");
        self.ensure_dir(skill_dir.as_std_path())?;

        // Templates directory
        let templates_dir = skill_dir.join("templates");
        self.ensure_dir(templates_dir.as_std_path())?;

        // Commands directory
        let commands_dir = config_dir.join("commands");
        self.ensure_dir(commands_dir.as_std_path())?;

        // Create command files
        for cmd in Self::canonical_commands() {
            let cmd_file = commands_dir.join(format!("maestro:{}.md", cmd));
            if !cmd_file.exists() && !self.dry_run {
                fs::write(
                    &cmd_file,
                    format!(
                        "# Maestro /{} command\n\
                        \n\
                        This is a placeholder. The actual command protocol\n\
                        should be loaded from the canonical source.\n",
                        cmd
                    ),
                )?;
            }
        }

        Ok(())
    }

    /// Install Gemini/Qwen TOML command files
    fn install_toml_commands(&self, tool: IntegrationTool, commands_dir: &Path) -> Result<()> {
        self.log(&format!("Installing {} TOML commands", format!("{:?}", tool).to_lowercase()));

        self.ensure_dir(commands_dir)?;

        for cmd in Self::canonical_commands() {
            let cmd_file = commands_dir.join(format!("{}.toml", cmd));
            if !cmd_file.exists() && !self.dry_run {
                let toml_content = format!(
                    r#"# Maestro {} command
name = "maestro:{}"
description = "Maestro {} command"

[command]
type = "custom"
template = """
Load Maestro and execute: /maestro {} {{{{args}}}}
"""
"#,
                    cmd, cmd, cmd, cmd
                );
                fs::write(&cmd_file, toml_content)?;
            }
        }

        Ok(())
    }

    /// Install Codex prompt files
    fn install_codex_prompts(&self, prompts_dir: &Path) -> Result<()> {
        self.log("Installing Codex prompt files");

        self.ensure_dir(prompts_dir)?;

        for cmd in Self::canonical_commands() {
            let prompt_file = prompts_dir.join(format!("maestro:{}.md", cmd));
            if !prompt_file.exists() && !self.dry_run {
                fs::write(
                    &prompt_file,
                    format!(
                        "# Maestro {} Command\n\
                        \n\
                        description: Execute Maestro {} command\n\
                        argument-hint: <args>\n\
                        \n\
                        Load Maestro and execute: /maestro {} {{{{args}}}}\n",
                        cmd, cmd, cmd
                    ),
                )?;
            }
        }

        Ok(())
    }

    /// Update OpenCode opencode.json with command entries
    fn update_opencode_json_commands(&self, tool: IntegrationTool) -> Result<()> {
        let config_dir = tool.config_dir()
            .ok_or_else(|| anyhow::anyhow!("No config dir for OpenCode"))?;
        let opencode_json = config_dir.join("opencode.json");

        if !opencode_json.exists() {
            self.log(&format!("opencode.json does not exist, creating new"));
            let mut commands = JsonMap::new();

            // Add maestro parent command
            commands.insert(
                "maestro".to_string(),
                json!({
                    "template": "Load Maestro skill. Available: setup, newTrack, implement, orchestrate, status, revert, configure, leindex, tui, memory.",
                    "description": "Maestro spec-driven development framework"
                }),
            );

            // Add subcommands
            for cmd in Self::canonical_commands() {
                commands.insert(
                    format!("maestro:{}", cmd),
                    json!({
                        "template": format!("Read and execute from ~/.config/opencode/commands/maestro:{}.md with args: $ARGUMENTS", cmd),
                        "description": format!("Maestro {} command", cmd)
                    }),
                );
            }

            let config = json!({
                "command": commands,
                "mcp": {
                    "leindex": {
                        "type": "local",
                        "command": ["maestro", "mcp", "proxy", "leindex"]
                    }
                }
            });

            if !self.dry_run {
                fs::write(
                    opencode_json.as_std_path(),
                    serde_json::to_string_pretty(&config)?,
                )?;
            }
            return Ok(());
        }

        self.backup_file(opencode_json.as_std_path())?;

        let existing = fs::read_to_string(opencode_json.as_std_path())?;
        let mut existing_json: Value = serde_json::from_str(&existing)
            .with_context(|| "Invalid JSON in opencode.json")?;

        // Ensure command object exists
        if !existing_json.is_object() {
            existing_json = json!({});
        }

        let obj = existing_json.as_object_mut().unwrap();
        if !obj.contains_key("command") {
            obj.insert("command".to_string(), json!( {}));
        }

        if let Some(commands) = obj.get_mut("command").and_then(|v| v.as_object_mut()) {
            // Add maestro parent command
            if !commands.contains_key("maestro") {
                commands.insert(
                    "maestro".to_string(),
                    json!({
                        "template": "Load Maestro skill. Available: setup, newTrack, implement, orchestrate, status, revert, configure, leindex, tui, memory.",
                        "description": "Maestro spec-driven development framework"
                    }),
                );
            }

            // Add subcommands
            for cmd in Self::canonical_commands() {
                let key = format!("maestro:{}", cmd);
                if !commands.contains_key(&key) {
                    commands.insert(
                        key,
                        json!({
                            "template": format!("Read and execute from ~/.config/opencode/commands/maestro:{}.md with args: $ARGUMENTS", cmd),
                            "description": format!("Maestro {} command", cmd)
                        }),
                    );
                }
            }
        }

        if !self.dry_run {
            fs::write(
                opencode_json.as_std_path(),
                serde_json::to_string_pretty(&existing_json)?,
            )?;
        }

        self.log("Updated opencode.json with Maestro command entries");
        Ok(())
    }

    /// Uninstall integration for a tool
    pub fn uninstall(&self, tool: IntegrationTool) -> Result<()> {
        self.log(&format!("Uninstalling integration for {:?}", tool));

        // Remove command files
        if let Some(commands_dir) = tool.commands_dir() {
            if commands_dir.exists() {
                if !self.dry_run {
                    fs::remove_dir_all(commands_dir.as_std_path())?;
                }
                self.log(&format!("Removed commands directory: {}", commands_dir));
            }
        }

        // Remove MCP config entry
        if let Some(mcp_config) = tool.mcp_config_path() {
            if mcp_config.exists() {
                self.backup_file(mcp_config.as_std_path())?;

                if tool.uses_json_config() {
                    self.remove_mcp_from_json(tool, mcp_config.as_std_path())?;
                } else {
                    self.remove_mcp_from_toml(tool, mcp_config.as_std_path())?;
                }
            }
        }

        eprintln!("Uninstalled integration for {:?}", tool);
        Ok(())
    }

    /// Remove MCP server from JSON config
    fn remove_mcp_from_json(&self, tool: IntegrationTool, config_path: &Path) -> Result<()> {
        let server_name = tool.mcp_server_name();
        let config_key = tool.mcp_config_json_path()
            .ok_or_else(|| anyhow::anyhow!("No MCP config path for {:?}", tool))?;

        let existing = fs::read_to_string(config_path)?;
        let mut existing_json: Value = serde_json::from_str(&existing)?;

        // Navigate to and remove the server entry using a helper to avoid borrow issues
        let parts: Vec<&str> = config_key.split('.').collect();

        // Use a helper function to navigate the JSON tree
        fn navigate_to_target<'a>(value: &'a mut Value, parts: &[&str]) -> Option<&'a mut Value> {
            let mut current = value;
            for part in parts {
                current = current.as_object_mut()?.get_mut(*part)?;
            }
            Some(current)
        }

        if let Some(target) = navigate_to_target(&mut existing_json, &parts) {
            if let Some(obj) = target.as_object_mut() {
                obj.remove(server_name);
            }
        }

        if !self.dry_run {
            fs::write(
                config_path,
                serde_json::to_string_pretty(&existing_json)?,
            )?;
        }

        self.log(&format!("Removed {} from {}", server_name, config_path.display()));
        Ok(())
    }

    /// Remove MCP server from TOML config
    fn remove_mcp_from_toml(&self, tool: IntegrationTool, config_path: &Path) -> Result<()> {
        let server_name = tool.mcp_server_name();

        let existing = fs::read_to_string(config_path)?;
        let mut toml_val: TomlValue = existing.parse()?;

        if let Some(table) = toml_val.as_table_mut() {
            if let Some(mcp_servers) = table.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) {
                mcp_servers.remove(server_name);
            }
        }

        if !self.dry_run {
            fs::write(config_path, toml_val.to_string())?;
        }

        self.log(&format!("Removed {} from {}", server_name, config_path.display()));
        Ok(())
    }

    /// Validate integration for a tool
    pub fn doctor(&self, tool: IntegrationTool) -> Result<DoctorReport> {
        self.log(&format!("Validating integration for {:?}", tool));

        let mut report = DoctorReport {
            tool: format!("{:?}", tool),
            passed: true,
            checks: Vec::new(),
        };

        // Check 1: Config directory exists
        let check = self.check_config_dir(tool);
        report.checks.push(check);
        if !report.checks.last().unwrap().passed {
            report.passed = false;
        }

        // Check 2: Commands directory exists (if applicable)
        if tool.commands_dir().is_some() {
            let check = self.check_commands_dir(tool);
            report.checks.push(check);
            if !report.checks.last().unwrap().passed {
                report.passed = false;
            }
        }

        // Check 3: MCP config exists
        let check = self.check_mcp_config(tool);
        report.checks.push(check);
        if !report.checks.last().unwrap().passed {
            report.passed = false;
        }

        // Check 4: LeIndex MCP server registered
        let check = self.check_leindex_registered(tool);
        report.checks.push(check);
        if !report.checks.last().unwrap().passed {
            report.passed = false;
        }

        // Check 5: No forbidden cross-tool references
        let check = self.check_no_cross_tool_refs(tool);
        report.checks.push(check);
        if !report.checks.last().unwrap().passed {
            report.passed = false;
        }

        Ok(report)
    }

    /// Check if config directory exists
    fn check_config_dir(&self, tool: IntegrationTool) -> CheckResult {
        let config_dir = tool.config_dir();
        let exists = config_dir.as_ref().map(|d| d.exists()).unwrap_or(false);

        CheckResult {
            name: "Config directory exists".to_string(),
            passed: exists,
            message: if exists {
                format!("Found: {}", config_dir.unwrap())
            } else {
                format!("Missing: {:?}", tool.config_dir())
            },
        }
    }

    /// Check if commands directory exists
    fn check_commands_dir(&self, tool: IntegrationTool) -> CheckResult {
        let commands_dir = tool.commands_dir();
        let exists = commands_dir.as_ref().map(|d| d.exists()).unwrap_or(false);

        CheckResult {
            name: "Commands directory exists".to_string(),
            passed: exists,
            message: if exists {
                format!("Found: {}", commands_dir.unwrap())
            } else {
                format!("Not required for this tool")
            },
        }
    }

    /// Check if MCP config file exists
    fn check_mcp_config(&self, tool: IntegrationTool) -> CheckResult {
        let mcp_config = tool.mcp_config_path();
        let exists = mcp_config.as_ref().map(|c| c.exists()).unwrap_or(false);

        CheckResult {
            name: "MCP config file exists".to_string(),
            passed: exists,
            message: if exists {
                format!("Found: {}", mcp_config.unwrap())
            } else {
                format!("Missing: {:?}", tool.mcp_config_path())
            },
        }
    }

    /// Check if LeIndex is registered in MCP config
    fn check_leindex_registered(&self, tool: IntegrationTool) -> CheckResult {
        let mcp_config = tool.mcp_config_path();
        let server_name = tool.mcp_server_name();

        let (exists, message) = match mcp_config {
            Some(path) if path.exists() => {
                if tool.uses_json_config() {
                    match fs::read_to_string(path.as_std_path()) {
                        Ok(content) => {
                            match serde_json::from_str::<Value>(&content) {
                                Ok(json) => {
                                    let config_key = tool.mcp_config_json_path();
                                    if let Some(key) = config_key {
                                        let parts: Vec<&str> = key.split('.').collect();

                                        // Helper to navigate JSON tree
                                        fn navigate_immutable<'a>(value: &'a Value, parts: &[&str]) -> Option<&'a Value> {
                                            let mut current = value;
                                            for part in parts {
                                                current = current.as_object()?.get(*part)?;
                                            }
                                            Some(current)
                                        }

                                        let registered = navigate_immutable(&json, &parts)
                                            .and_then(|v| v.as_object())
                                            .and_then(|o| o.get(server_name))
                                            .is_some();

                                        (registered, if registered {
                                            format!("LeIndex registered as '{}'", server_name)
                                        } else {
                                            format!("LeIndex not found in {}", key)
                                        })
                                    } else {
                                        (false, "No MCP config path".to_string())
                                    }
                                }
                                Err(e) => (false, format!("Invalid JSON: {}", e))
                            }
                        }
                        Err(e) => (false, format!("Cannot read: {}", e))
                    }
                } else {
                    // TOML
                    match fs::read_to_string(path.as_std_path()) {
                        Ok(content) => {
                            match content.parse::<TomlValue>() {
                                Ok(toml) => {
                                    let registered = toml.as_table()
                                        .and_then(|t| t.get("mcp_servers"))
                                        .and_then(|v| v.as_table())
                                        .and_then(|t| t.get(server_name))
                                        .is_some();

                                    (registered, if registered {
                                        format!("LeIndex registered as '{}'", server_name)
                                    } else {
                                        format!("LeIndex not found in mcp_servers")
                                    })
                                }
                                Err(e) => (false, format!("Invalid TOML: {}", e))
                            }
                        }
                        Err(e) => (false, format!("Cannot read: {}", e))
                    }
                }
            }
            _ => (false, "MCP config file not found".to_string())
        };

        CheckResult {
            name: "LeIndex MCP server registered".to_string(),
            passed: exists,
            message,
        }
    }

    /// Check for forbidden cross-tool references
    fn check_no_cross_tool_refs(&self, tool: IntegrationTool) -> CheckResult {
        // For OpenCode, ensure no references to ~/.claude
        if matches!(tool, IntegrationTool::OpenCode) {
            let config_dir = tool.config_dir();
            let has_forbidden_refs = if let Some(dir) = config_dir {
                self.scan_for_claude_refs(dir.as_std_path())
            } else {
                false
            };

            return CheckResult {
                name: "No cross-tool references".to_string(),
                passed: !has_forbidden_refs,
                message: if has_forbidden_refs {
                    "Found references to ~/.claude/ (should be self-contained)".to_string()
                } else {
                    "No forbidden cross-tool references found".to_string()
                },
            };
        }

        CheckResult {
            name: "No cross-tool references".to_string(),
            passed: true,
            message: "Not applicable for this tool".to_string(),
        }
    }

    /// Scan a directory for references to ~/.claude
    fn scan_for_claude_refs(&self, dir: &Path) -> bool {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if self.scan_for_claude_refs(&path) {
                        return true;
                    }
                } else if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("/.claude/") || content.contains("~/.claude") {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Print config patch for a tool
    pub fn print(&self, tool: IntegrationTool) -> Result<()> {
        let mcp_config = tool.leindex_mcp_config();
        let server_name = tool.mcp_server_name();

        eprintln!("# MCP configuration patch for {:?}", tool);
        eprintln!();

        if tool.uses_json_config() {
            if let Some(key) = tool.mcp_config_json_path() {
                eprintln!("{}:", key);
                eprintln!("  {}:", server_name);
                eprintln!("{}", serde_json::to_string_pretty(&mcp_config)?);
            }
        } else {
            // TOML format for Codex
            eprintln!("[mcp_servers.{}]", server_name);
            if let Some(cmd) = mcp_config.get("command") {
                eprintln!("command = {}", cmd);
            }
            if let Some(args) = mcp_config.get("args") {
                if let Some(arr) = args.as_array() {
                    eprintln!("args = [");
                    for arg in arr {
                        if let Some(s) = arg.as_str() {
                            eprintln!("  \"{}\",", s);
                        }
                    }
                    eprintln!("]");
                }
            }
        }

        Ok(())
    }

    /// Install all integrations
    pub fn install_all(&self) -> Result<()> {
        eprintln!("Installing all integrations...");
        for tool in IntegrationTool::all() {
            if let Err(e) = self.install(*tool) {
                eprintln!("Warning: Failed to install {:?}: {}", tool, e);
            }
        }
        eprintln!("All integrations installed");
        Ok(())
    }
}

/// Doctor check result
#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Doctor report
#[derive(Debug)]
pub struct DoctorReport {
    pub tool: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,
}

impl DoctorReport {
    /// Print the report
    pub fn print(&self) {
        println!("# Doctor Report for {}", self.tool);
        println!();

        if self.passed {
            println!("Status: PASSED");
        } else {
            println!("Status: FAILED");
        }
        println!();

        for check in &self.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            println!("[{}] {}", status, check.name);
            println!("  {}", check.message);
            println!();
        }
    }
}

/// Run the integration command
pub async fn run(
    action: IntegrateAction,
    tool: Option<IntegrationTool>,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let integrator = Integrator::new(dry_run, verbose);

    match action {
        IntegrateAction::Install => {
            if let Some(t) = tool {
                integrator.install(t)?;
            } else {
                integrator.install_all()?;
            }
        }
        IntegrateAction::Uninstall => {
            if let Some(t) = tool {
                integrator.uninstall(t)?;
            } else {
                anyhow::bail!("Tool must be specified for uninstall");
            }
        }
        IntegrateAction::Doctor => {
            if let Some(t) = tool {
                let report = integrator.doctor(t)?;
                report.print();
            } else {
                anyhow::bail!("Tool must be specified for doctor");
            }
        }
        IntegrateAction::Print => {
            if let Some(t) = tool {
                integrator.print(t)?;
            } else {
                anyhow::bail!("Tool must be specified for print");
            }
        }
    }

    Ok(())
}

/// Integration actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::Subcommand)]
pub enum IntegrateAction {
    /// Install integration for a tool (or all tools with --all)
    Install,
    /// Uninstall integration for a tool
    Uninstall,
    /// Validate integration for a tool
    Doctor,
    /// Emit config patch for a tool
    Print,
}
