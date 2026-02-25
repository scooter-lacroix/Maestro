//! Shell execution tool with safety constraints
//!
//! The ShellTool provides controlled shell command execution with:
//! - Command risk classification (safe, moderate, dangerous)
//! - Timeout handling
//! - Blocked command patterns
//! - Output capture and sanitization

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::time::Duration;
use tokio::process::Command as AsyncCommand;
use tokio::time::timeout;

#[cfg(test)]
use std::process::Command;

use crate::tools::{Tool, ToolOutput};

/// Risk level classification for shell commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandRiskLevel {
    /// Safe commands that cannot cause damage (echo, ls, cat)
    Safe,
    /// Moderate risk commands that modify state (mkdir, touch, cp)
    Moderate,
    /// Dangerous commands that can cause data loss or system changes (rm, chmod, chown)
    Dangerous,
    /// Blocked commands that are never allowed (rm -rf /, mkfs, dd)
    Blocked,
}

/// Configuration for ShellTool
#[derive(Debug, Clone)]
pub struct ShellToolConfig {
    /// Maximum execution time in seconds
    pub timeout_secs: u64,
    /// Maximum output size in bytes
    pub max_output_bytes: usize,
    /// Whether to allow moderate risk commands
    pub allow_moderate: bool,
    /// Whether to allow dangerous commands
    pub allow_dangerous: bool,
    /// Working directory for commands
    pub working_directory: Option<std::path::PathBuf>,
}

impl Default for ShellToolConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_output_bytes: 1024 * 1024, // 1MB
            allow_moderate: true,
            allow_dangerous: false,
            working_directory: None,
        }
    }
}

/// Shell tool for executing commands
pub struct ShellTool {
    config: ShellToolConfig,
}

impl ShellTool {
    /// Create a new ShellTool with default configuration
    pub fn new() -> Self {
        Self {
            config: ShellToolConfig::default(),
        }
    }

    /// Create a new ShellTool with custom configuration
    pub fn with_config(config: ShellToolConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &ShellToolConfig {
        &self.config
    }

    /// Classify the risk level of a command
    pub fn classify_command(command: &str) -> CommandRiskLevel {
        let command_lower = command.to_lowercase();
        let command_trimmed = command_lower.trim();

        // Blocked patterns - never allowed
        // (MED-2) `sudo` and `eval` are always blocked because they trivially
        // bypass every downstream classification check.
        let blocked_patterns = [
            "rm -rf /",
            "rm -rf /*",
            "mkfs",
            "dd if=",
            ":(){ :|:& };:",  // Fork bomb
            "> /dev/sda",
            "chmod -R 777 /",
            "chown -R",
            "shutdown",
            "reboot",
            "halt",
            "init 0",
            "init 6",
            "systemctl stop",
            "systemctl disable",
            "iptables -F",
            "ufw disable",
            "sudo",           // MED-2: privilege escalation bypasses all guards
            "eval",           // MED-2: dynamic evaluation bypasses classification
        ];

        for pattern in &blocked_patterns {
            if command_trimmed.contains(pattern) {
                return CommandRiskLevel::Blocked;
            }
        }

        // Parse command name (first word)
        let cmd_name = command_trimmed.split_whitespace().next().unwrap_or("");

        // Dangerous commands
        // (MED-2) Shell interpreters and scripting runtimes are classified as
        // Dangerous because passing `-c "<payload>"` lets any blocked command
        // bypass the pattern/name checks above (e.g. `bash -c "rm -rf /"`).
        // They require `allow_dangerous: true` to execute.
        let dangerous_commands = [
            "rm", "rmdir", "del", "format", "fdisk", "parted", "wipefs",
            "chmod", "chown", "chgrp", "attr", "setfacl",
            "kill", "killall", "pkill", "xkill",
            "apt", "apt-get", "dpkg", "yum", "dnf", "pacman", "snap", "flatpak",
            "pip", "pip3", "npm", "yarn", "cargo install",
            "git push", "git reset --hard", "git clean -fdx",
            "docker rm", "docker rmi", "docker system prune",
            "kubectl delete",
            // Shell interpreters (MED-2)
            "bash", "sh", "zsh", "dash", "fish", "ksh", "csh", "tcsh",
            // Scripting language runtimes (MED-2)
            "python", "python2", "python3",
            "perl", "ruby", "php",
            "node", "nodejs", "deno", "bun",
            "lua", "tclsh", "wish",
            // exec replaces current process — treat as dangerous
            "exec",
        ];

        for dangerous in &dangerous_commands {
            if cmd_name == *dangerous || command_trimmed.starts_with(dangerous) {
                return CommandRiskLevel::Dangerous;
            }
        }

        // Moderate risk commands
        let moderate_commands = [
            "mkdir", "touch", "cp", "mv", "ln",
            "echo", "cat", "tee",
            "curl", "wget",
            "tar", "zip", "unzip",
            "git add", "git commit", "git checkout",
            "docker run", "docker build",
            "make", "cmake",
        ];

        for moderate in &moderate_commands {
            if cmd_name == *moderate || command_trimmed.starts_with(moderate) {
                return CommandRiskLevel::Moderate;
            }
        }

        // Safe commands
        CommandRiskLevel::Safe
    }

    /// Check if a command is allowed based on configuration
    fn is_command_allowed(&self, command: &str) -> Result<CommandRiskLevel, String> {
        let risk_level = Self::classify_command(command);

        match risk_level {
            CommandRiskLevel::Blocked => {
                Err(format!("Command is blocked: {}", command))
            }
            CommandRiskLevel::Dangerous => {
                if self.config.allow_dangerous {
                    Ok(risk_level)
                } else {
                    Err(format!("Dangerous command not allowed: {}", command))
                }
            }
            CommandRiskLevel::Moderate => {
                if self.config.allow_moderate {
                    Ok(risk_level)
                } else {
                    Err(format!("Moderate risk command not allowed: {}", command))
                }
            }
            CommandRiskLevel::Safe => Ok(risk_level),
        }
    }

    /// Execute a shell command synchronously (for testing)
    #[cfg(test)]
    fn execute_sync(&self, command: &str) -> ToolOutput {
        // Check if command is allowed
        if let Err(e) = self.is_command_allowed(command) {
            return ToolOutput::error(e);
        }

        // Execute using std::process::Command
        let result = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", command]).output()
        } else {
            Command::new("sh").args(["-c", command]).output()
        };

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    ToolOutput::success(stdout.to_string())
                } else {
                    ToolOutput::error(format!("Command failed: {}", stderr))
                }
            }
            Err(e) => ToolOutput::error(format!("Failed to execute command: {}", e)),
        }
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute shell commands with safety constraints. Commands are classified by risk level (safe, moderate, dangerous, blocked) and filtered accordingly."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Optional timeout in seconds (overrides default)",
                    "minimum": 1,
                    "maximum": 300
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        // Parse command argument
        let command = match arguments.get("command") {
            Some(v) => match v.as_str() {
                Some(s) => s,
                None => return ToolOutput::error("command must be a string".to_string()),
            },
            None => return ToolOutput::error("command argument required".to_string()),
        };

        // Check if command is allowed
        if let Err(e) = self.is_command_allowed(command) {
            return ToolOutput::error(e);
        }

        // Get timeout
        let timeout_secs = arguments
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.config.timeout_secs)
            .min(300); // Cap at 5 minutes

        // Execute command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = AsyncCommand::new("cmd");
            cmd.args(["/C", command]);
            cmd
        } else {
            let mut cmd = AsyncCommand::new("sh");
            cmd.args(["-c", command]);
            cmd
        };

        // Set working directory if configured
        if let Some(ref dir) = self.config.working_directory {
            cmd.current_dir(dir);
        }

        // Execute with timeout
        let result = timeout(Duration::from_secs(timeout_secs), async {
            cmd.output().await
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Truncate output if too large
                let output_str = if stdout.len() > self.config.max_output_bytes {
                    format!(
                        "{}\n... (output truncated, {} bytes)",
                        &stdout[..self.config.max_output_bytes],
                        stdout.len()
                    )
                } else if !stderr.is_empty() && !output.status.success() {
                    format!("Error: {}", stderr)
                } else {
                    stdout.to_string()
                };

                if output.status.success() {
                    ToolOutput::success(output_str)
                } else {
                    ToolOutput::error(output_str)
                }
            }
            Ok(Err(e)) => ToolOutput::error(format!("Failed to execute command: {}", e)),
            Err(_) => ToolOutput::error(format!(
                "Command timed out after {} seconds",
                timeout_secs
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_safe_commands() {
        // ls, pwd, whoami are safe (not in moderate/dangerous/blocked lists)
        assert_eq!(
            ShellTool::classify_command("ls -la"),
            CommandRiskLevel::Safe
        );
        assert_eq!(
            ShellTool::classify_command("pwd"),
            CommandRiskLevel::Safe
        );
        assert_eq!(
            ShellTool::classify_command("whoami"),
            CommandRiskLevel::Safe
        );
        // Note: echo is classified as Moderate because it can write to files (echo > file)
    }

    #[test]
    fn test_classify_moderate_commands() {
        assert_eq!(
            ShellTool::classify_command("mkdir test"),
            CommandRiskLevel::Moderate
        );
        assert_eq!(
            ShellTool::classify_command("touch file.txt"),
            CommandRiskLevel::Moderate
        );
        assert_eq!(
            ShellTool::classify_command("cp src dst"),
            CommandRiskLevel::Moderate
        );
        assert_eq!(
            ShellTool::classify_command("mv old new"),
            CommandRiskLevel::Moderate
        );
    }

    #[test]
    fn test_classify_dangerous_commands() {
        assert_eq!(
            ShellTool::classify_command("rm file.txt"),
            CommandRiskLevel::Dangerous
        );
        assert_eq!(
            ShellTool::classify_command("chmod 755 script.sh"),
            CommandRiskLevel::Dangerous
        );
        assert_eq!(
            ShellTool::classify_command("chown user file"),
            CommandRiskLevel::Dangerous
        );
        assert_eq!(
            ShellTool::classify_command("kill 1234"),
            CommandRiskLevel::Dangerous
        );
    }

    #[test]
    fn test_classify_blocked_commands() {
        assert_eq!(
            ShellTool::classify_command("rm -rf /"),
            CommandRiskLevel::Blocked
        );
        assert_eq!(
            ShellTool::classify_command("mkfs.ext4 /dev/sda1"),
            CommandRiskLevel::Blocked
        );
        assert_eq!(
            ShellTool::classify_command("dd if=/dev/zero of=/dev/sda"),
            CommandRiskLevel::Blocked
        );
        assert_eq!(
            ShellTool::classify_command("shutdown -h now"),
            CommandRiskLevel::Blocked
        );
    }

    #[test]
    fn test_safe_command_execution() {
        let tool = ShellTool::new();
        let output = tool.execute_sync("echo hello");
        assert!(!output.is_error);
        assert!(output.content.contains("hello"));
    }

    #[test]
    fn test_list_command() {
        let tool = ShellTool::new();
        let output = tool.execute_sync("ls");
        // ls should succeed
        assert!(!output.is_error);
    }

    #[test]
    fn test_blocked_command_rejected() {
        let tool = ShellTool::new();
        let output = tool.execute_sync("rm -rf /");
        assert!(output.is_error);
        assert!(output.content.contains("blocked"));
    }

    #[test]
    fn test_dangerous_command_rejected_by_default() {
        let tool = ShellTool::new();
        let output = tool.execute_sync("rm test.txt");
        assert!(output.is_error);
        assert!(output.content.contains("Dangerous"));
    }

    #[test]
    fn test_dangerous_command_allowed_with_config() {
        let config = ShellToolConfig {
            allow_dangerous: true,
            ..Default::default()
        };
        let tool = ShellTool::with_config(config);
        // Still blocked - rm -rf / should always be blocked
        let output = tool.execute_sync("rm -rf /");
        assert!(output.is_error);
    }

    #[test]
    fn test_moderate_command_allowed_by_default() {
        let tool = ShellTool::new();
        let output = tool.execute_sync("mkdir -p /tmp/maestro_test_dir_12345");
        assert!(!output.is_error);

        // Cleanup
        let _ = std::fs::remove_dir_all("/tmp/maestro_test_dir_12345");
    }

    #[test]
    fn test_interpreter_bypass_rejected_by_default() {
        // MED-2: Shell interpreters are Dangerous and must be rejected without
        // allow_dangerous: true, preventing `bash -c "rm -rf /"` style bypasses.
        let tool = ShellTool::new(); // allow_dangerous defaults to false

        let interpreters = [
            "bash -c 'rm -rf /'",
            "sh -c 'shutdown -h now'",
            "python3 -c 'import os; os.system(\"rm -rf /\")'",
            "perl -e 'system(\"rm -rf /\")'",
            "node -e 'require(\"child_process\").exec(\"shutdown\")'",
        ];

        for cmd in &interpreters {
            let output = tool.execute_sync(cmd);
            assert!(
                output.is_error,
                "Interpreter bypass '{}' should be rejected with default config",
                cmd
            );
        }
    }

    #[test]
    fn test_sudo_blocked() {
        // MED-2: sudo is always blocked regardless of allow_dangerous.
        let config = ShellToolConfig {
            allow_dangerous: true,
            ..Default::default()
        };
        let tool = ShellTool::with_config(config);

        let output = tool.execute_sync("sudo rm -rf /");
        assert!(output.is_error, "sudo must always be blocked");
        assert!(output.content.contains("blocked"));
    }

    #[test]
    fn test_eval_blocked() {
        // MED-2: eval is always blocked.
        let tool = ShellTool::new();
        let output = tool.execute_sync("eval 'shutdown now'");
        assert!(output.is_error, "eval must always be blocked");
        assert!(output.content.contains("blocked"));
    }

    #[test]
    fn test_parameters_schema() {
        let tool = ShellTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["command"]["type"].is_string());
        assert!(schema["required"].as_array().unwrap().contains(&json!("command")));
    }

    #[test]
    fn test_tool_name_and_description() {
        let tool = ShellTool::new();
        assert_eq!(tool.name(), "shell");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_async_execution_echo() {
        let tool = ShellTool::new();
        let output = tool.execute(json!({"command": "echo hello async"})).await;
        assert!(!output.is_error);
        assert!(output.content.contains("hello async"));
    }

    #[tokio::test]
    async fn test_async_execution_blocked() {
        let tool = ShellTool::new();
        let output = tool.execute(json!({"command": "rm -rf /"})).await;
        assert!(output.is_error);
    }

    #[tokio::test]
    async fn test_async_execution_missing_command() {
        let tool = ShellTool::new();
        let output = tool.execute(json!({})).await;
        assert!(output.is_error);
        assert!(output.content.contains("required"));
    }

    #[tokio::test]
    async fn test_async_execution_invalid_command_type() {
        let tool = ShellTool::new();
        let output = tool.execute(json!({"command": 123})).await;
        assert!(output.is_error);
        assert!(output.content.contains("must be a string"));
    }

    #[tokio::test]
    async fn test_async_execution_timeout() {
        let config = ShellToolConfig {
            timeout_secs: 1,
            ..Default::default()
        };
        let tool = ShellTool::with_config(config);

        // Use sleep command that exceeds timeout
        let output = tool
            .execute(json!({"command": "sleep 5"}))
            .await;
        assert!(output.is_error);
        assert!(output.content.contains("timed out"));
    }

    #[test]
    fn test_tool_spec_conversion() {
        let tool = ShellTool::new();
        let spec = tool.to_spec();

        assert_eq!(spec.name, "shell");
        assert!(!spec.description.is_empty());
        assert!(spec.parameters.is_object());
    }
}
