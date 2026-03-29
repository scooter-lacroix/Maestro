//! MaestroClaw Setup Wizard
//!
//! This module provides the setup wizard functionality for MaestroClaw,
//! including tool detection, provider selection, and configuration guidance.

use std::collections::HashSet;
use std::path::PathBuf;

use super::channels::ChannelType;
use maestro_claw::config::Config;

/// Provider choice for AI model selection
#[derive(Debug, Clone)]
pub struct ProviderChoice {
    pub id: String,
    pub label: String,
    pub is_configured: bool,
    pub icon: &'static str,
}

/// Tool availability information
#[derive(Debug, Clone)]
pub struct ToolAvailability {
    pub name: String,
    pub available: bool,
    pub missing_hint: Option<String>,
}

/// Steps in the setup wizard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
    ToolDetection,
    PrimaryToolSelection,
    ProviderSelection,
    ChannelSetup,
    CronSetup,
    ToolSummary,
    Complete,
}

impl WizardStep {
    /// Returns the 1-indexed step number for progress display.
    pub fn number(self) -> usize {
        match self {
            WizardStep::Welcome => 1,
            WizardStep::ToolDetection => 2,
            WizardStep::PrimaryToolSelection => 3,
            WizardStep::ProviderSelection => 4,
            WizardStep::ChannelSetup => 5,
            WizardStep::CronSetup => 6,
            WizardStep::ToolSummary => 7,
            WizardStep::Complete => 8,
        }
    }

    /// Returns the display label for the step.
    pub fn label(self) -> &'static str {
        match self {
            WizardStep::Welcome => "Welcome",
            WizardStep::ToolDetection => "Tool Detection",
            WizardStep::PrimaryToolSelection => "Primary Tool",
            WizardStep::ProviderSelection => "Provider",
            WizardStep::ChannelSetup => "Channels",
            WizardStep::CronSetup => "Cron",
            WizardStep::ToolSummary => "Summary",
            WizardStep::Complete => "Complete",
        }
    }

    pub const TOTAL_STEPS: usize = 8;
}

/// Setup wizard state for MaestroClaw
#[derive(Debug, Clone)]
pub struct SetupWizard {
    /// List of available tools detected on the system
    pub available_tools: Vec<String>,
    /// Current cursor position for navigation
    pub cursor: usize,
    /// Selected primary tool index
    pub selected_primary_tool: Option<usize>,
    /// Selected provider index
    pub selected_provider: Option<usize>,
    /// Selected channels
    pub selected_channels: HashSet<ChannelType>,
    /// Whether scheduled cron automation is enabled
    pub cron_enabled: bool,
    /// Maximum cron run history to retain
    pub cron_max_run_history: usize,
    /// Tool details: (name, version, binary_path)
    pub tool_details: Vec<(String, Option<String>, Option<String>)>,
    /// List of available providers
    pub provider_list: Vec<ProviderChoice>,
    /// Tool availability summary
    pub tool_summary: Vec<ToolAvailability>,
    /// Whether the wizard has been dismissed
    dismissed: bool,
    /// Whether the wizard has been completed
    completed: bool,
    /// Current step in the wizard
    current_step: WizardStep,
    /// Workspace directory (same source of truth as Config::workspace_dir)
    pub workspace_dir: PathBuf,
}

impl SetupWizard {
    /// Create a new setup wizard with the given workspace directory.
    ///
    /// The `workspace_dir` should come from `Config::default().workspace_dir`
    /// (or `Config::load()?.workspace_dir`), keeping a single source of truth
    /// shared with doctor, onboarding, and gateway.
    pub fn new(workspace_dir: PathBuf) -> Self {
        let mut wizard = Self {
            available_tools: Vec::new(),
            cursor: 0,
            selected_primary_tool: None,
            selected_provider: None,
            selected_channels: HashSet::new(),
            cron_enabled: true,
            cron_max_run_history: 50,
            tool_details: Vec::new(),
            provider_list: Vec::new(),
            tool_summary: Vec::new(),
            dismissed: false,
            completed: false,
            current_step: WizardStep::Welcome,
            workspace_dir,
        };

        wizard.detect_tools();
        wizard.build_provider_list();
        wizard.build_tool_summary();

        wizard
    }

    /// Detect available tools on the system with version information
    pub fn detect_tools(&mut self) {
        // Agent tools with version detection
        let agent_tools = ["claude", "codex", "gemini", "qwen", "amp", "droid", "iflow"];

        // Supplementary tools (presence only)
        let supplementary_tools = [
            "git", "gh", "docker", "kubectl", "npm", "yarn", "pnpm", "cargo", "rustc", "python3",
            "python", "pip", "uv", "node", "bun", "deno", "go",
        ];

        let mut detected = Vec::new();
        let mut details = Vec::new();

        // Detect agent tools with version information
        for tool in &agent_tools {
            if let Ok(path) = which::which(tool) {
                let version = Self::get_tool_version(tool, &path);
                detected.push(tool.to_string());
                details.push((
                    tool.to_string(),
                    version,
                    path.to_str().map(|s| s.to_string()),
                ));
            }
        }

        // Auto-select first agent tool as primary, but only if none already chosen
        if !details.is_empty() && self.selected_primary_tool.is_none() {
            self.selected_primary_tool = Some(0);
        }

        // Detect supplementary tools (presence only)
        for tool in &supplementary_tools {
            if which::which(tool).is_ok() {
                detected.push(tool.to_string());
            }
        }

        self.available_tools = detected;
        self.tool_details = details;
    }

    /// Get version string for a tool by running `tool --version`
    fn get_tool_version(_tool: &str, path: &std::path::Path) -> Option<String> {
        use std::process::Command;

        let result = Command::new(path).arg("--version").output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Capture first line of stdout
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .map(|line| line.trim().to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Build list of available providers
    pub fn build_provider_list(&mut self) {
        let mut providers = Vec::new();

        // Check for OpenAI API key
        let openai_configured = std::env::var("OPENAI_API_KEY").is_ok();
        providers.push(ProviderChoice {
            id: "openai".to_string(),
            label: "OpenAI (GPT models via OPENAI_API_KEY)".to_string(),
            is_configured: openai_configured,
            icon: "🤖",
        });

        // Check for Anthropic API key
        let anthropic_configured = std::env::var("ANTHROPIC_API_KEY").is_ok();
        providers.push(ProviderChoice {
            id: "anthropic".to_string(),
            label: "Anthropic (Claude models via ANTHROPIC_API_KEY)".to_string(),
            is_configured: anthropic_configured,
            icon: "🧠",
        });

        // Check for OpenRouter API key
        let openrouter_configured = std::env::var("OPENROUTER_API_KEY").is_ok();
        providers.push(ProviderChoice {
            id: "openrouter".to_string(),
            label: "OpenRouter (100+ models, pay-per-use)".to_string(),
            is_configured: openrouter_configured,
            icon: "🌐",
        });

        // Check for Ollama binary
        let ollama_configured = which::which("ollama").is_ok();
        providers.push(ProviderChoice {
            id: "ollama".to_string(),
            label: "Ollama (local models, no API key needed)".to_string(),
            is_configured: ollama_configured,
            icon: "🏠",
        });

        // Check for custom OpenAI base URL
        let custom_configured = std::env::var("OPENAI_BASE_URL").is_ok();
        providers.push(ProviderChoice {
            id: "custom".to_string(),
            label: "Custom OpenAI-compatible endpoint".to_string(),
            is_configured: custom_configured,
            icon: "⚙️",
        });

        self.provider_list = providers;
    }

    /// Build tool availability summary
    pub fn build_tool_summary(&mut self) {
        let mut summary = Vec::new();

        // Shell/Terminal - always available in CLI context
        summary.push(ToolAvailability {
            name: "Shell / Terminal".to_string(),
            available: true,
            missing_hint: None,
        });

        // File Operations - always available
        summary.push(ToolAvailability {
            name: "File Operations".to_string(),
            available: true,
            missing_hint: None,
        });

        // Memory built-in - always available
        summary.push(ToolAvailability {
            name: "Memory (built-in)".to_string(),
            available: true,
            missing_hint: None,
        });

        // Cron Scheduler - always available
        summary.push(ToolAvailability {
            name: "Cron Scheduler".to_string(),
            available: true,
            missing_hint: None,
        });

        // MCP Servers - check workspace-local mcp/servers.toml
        // (same source of truth as doctor, onboarding, and gateway)
        let mcp_config_path = self.workspace_dir.join("mcp").join("servers.toml");
        let mcp_available = mcp_config_path.exists();
        summary.push(ToolAvailability {
            name: "MCP Servers".to_string(),
            available: mcp_available,
            missing_hint: if mcp_available {
                None
            } else {
                Some("Run 'maestro claw setup' to configure".to_string())
            },
        });

        // Gateway Web API - not available
        summary.push(ToolAvailability {
            name: "Gateway (Web API)".to_string(),
            available: false,
            missing_hint: Some("Start with 'maestro claw daemon'".to_string()),
        });

        self.tool_summary = summary;
    }

    /// Advance to the next step.
    ///
    /// When transitioning from `ChannelSetup` to `ToolSummary`, the tool
    /// summary is rebuilt so that MCP status reflects the current state of
    /// the workspace at entry time (e.g. files created during ChannelSetup).
    pub fn next_step(&mut self) {
        self.current_step = match self.current_step {
            WizardStep::Welcome => WizardStep::ToolDetection,
            WizardStep::ToolDetection => WizardStep::PrimaryToolSelection,
            WizardStep::PrimaryToolSelection => WizardStep::ProviderSelection,
            WizardStep::ProviderSelection => WizardStep::ChannelSetup,
            WizardStep::ChannelSetup => WizardStep::CronSetup,
            WizardStep::CronSetup => {
                self.build_tool_summary();
                WizardStep::ToolSummary
            }
            WizardStep::ToolSummary => {
                self.completed = true;
                WizardStep::Complete
            }
            WizardStep::Complete => WizardStep::Complete,
        };
    }

    /// Go back to the previous step
    pub fn previous_step(&mut self) {
        self.current_step = match self.current_step {
            WizardStep::Welcome => WizardStep::Welcome,
            WizardStep::ToolDetection => WizardStep::Welcome,
            WizardStep::PrimaryToolSelection => WizardStep::ToolDetection,
            WizardStep::ProviderSelection => WizardStep::PrimaryToolSelection,
            WizardStep::ChannelSetup => WizardStep::ProviderSelection,
            WizardStep::CronSetup => WizardStep::ChannelSetup,
            WizardStep::ToolSummary => WizardStep::CronSetup,
            WizardStep::Complete => WizardStep::ToolSummary,
        };
    }

    /// Get the current step
    pub fn current_step(&self) -> WizardStep {
        self.current_step
    }

    /// Check if wizard has been dismissed
    pub fn is_dismissed(&self) -> bool {
        self.dismissed
    }

    /// Dismiss the wizard
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Check if wizard has been completed
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Mark wizard as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// Reset the wizard state
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.selected_primary_tool = None;
        self.selected_provider = None;
        self.selected_channels.clear();
        self.cron_enabled = true;
        self.cron_max_run_history = 50;
        self.tool_details.clear();
        self.provider_list.clear();
        self.tool_summary.clear();
        self.dismissed = false;
        self.completed = false;
        self.current_step = WizardStep::Welcome;
        self.available_tools.clear();

        // Re-detect tools and rebuild lists
        self.detect_tools();
        self.build_provider_list();
        self.build_tool_summary();
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new(Config::default().workspace_dir)
    }
}
