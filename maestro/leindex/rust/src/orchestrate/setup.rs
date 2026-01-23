//! Orchestrate setup and first-run detection
//!
//! Provides helpers for detecting missing configuration and tools,
//! and guiding users through the setup process.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

/// Supported agent tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentTool {
    Claude,
    Gemini,
    Qwen,
    OpenCode,
    Amp,
    Codex,
    Droid,
}

impl AgentTool {
    pub fn as_str(&self) -> &str {
        match self {
            AgentTool::Claude => "claude",
            AgentTool::Gemini => "gemini",
            AgentTool::Qwen => "qwen",
            AgentTool::OpenCode => "opencode",
            AgentTool::Amp => "amp",
            AgentTool::Codex => "codex",
            AgentTool::Droid => "droid",
        }
    }

    pub fn binary_name(&self) -> &str {
        self.as_str()
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(AgentTool::Claude),
            "gemini" => Some(AgentTool::Gemini),
            "qwen" => Some(AgentTool::Qwen),
            "opencode" => Some(AgentTool::OpenCode),
            "amp" => Some(AgentTool::Amp),
            "codex" => Some(AgentTool::Codex),
            "droid" => Some(AgentTool::Droid),
            _ => None,
        }
    }
}

/// Setup status for orchestrate
#[derive(Debug, Clone)]
pub struct SetupStatus {
    /// Maestro config exists
    pub maestro_config_exists: bool,
    /// Tracks directory exists
    pub tracks_dir_exists: bool,
    /// tracks.md file exists
    pub tracks_md_exists: bool,
    /// Available tools
    pub available_tools: Vec<AgentTool>,
    /// Whether bubblewrap (sandbox) is available
    pub sandbox_available: bool,
}

impl SetupStatus {
    /// Check if setup is complete (minimal requirements)
    pub fn is_minimally_configured(&self) -> bool {
        self.tracks_md_exists && !self.available_tools.is_empty()
    }

    /// Check if setup is complete (recommended)
    pub fn is_fully_configured(&self) -> bool {
        self.maestro_config_exists
            && self.tracks_md_exists
            && !self.available_tools.is_empty()
            && self.available_tools.iter().any(|t| matches!(t, AgentTool::Claude | AgentTool::Gemini | AgentTool::Qwen))
    }

    /// Get missing requirements
    pub fn missing_requirements(&self) -> Vec<String> {
        let mut missing = Vec::new();

        if !self.tracks_md_exists {
            missing.push("tracks.md not found. Run 'maestro newTrack' to create one.".to_string());
        }

        if self.available_tools.is_empty() {
            missing.push(
                "No AI tools found. Install at least one: claude, gemini, qwen, or opencode.".to_string()
            );
        }

        missing
    }

    /// Get recommended improvements
    pub fn recommended_improvements(&self) -> Vec<String> {
        let mut improvements = Vec::new();

        if !self.maestro_config_exists {
            improvements.push("~/.maestro/config.toml not found. Run 'maestro configure' to set defaults.".to_string());
        }

        if !self.available_tools.iter().any(|t| matches!(t, AgentTool::Claude | AgentTool::Gemini | AgentTool::Qwen)) {
            improvements.push("No primary AI tool (claude/gemini/qwen) found. These are recommended for orchestrate.".to_string());
        }

        if !self.sandbox_available {
            improvements.push("bubblewrap (bwrap) not found. Install for sandbox mode: apt install bubblewrap".to_string());
        }

        improvements
    }
}

/// Check if a binary is available in PATH
pub fn check_binary_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the Maestro config directory
pub fn get_maestro_config_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".maestro"))
}

/// Detect the current setup status
pub fn detect_setup_status(tracks_dir: &Path) -> Result<SetupStatus> {
    let config_dir = get_maestro_config_dir()?;
    let config_file = config_dir.join("config.toml");
    let tracks_md = tracks_dir.join("tracks.md");

    // Check for all supported tools
    let all_tools = [
        AgentTool::Claude,
        AgentTool::Gemini,
        AgentTool::Qwen,
        AgentTool::OpenCode,
        AgentTool::Amp,
        AgentTool::Codex,
        AgentTool::Droid,
    ];

    let available_tools: Vec<AgentTool> = all_tools
        .iter()
        .filter(|t| check_binary_available(t.binary_name()))
        .copied()
        .collect();

    // Check for bubblewrap
    let sandbox_available = check_binary_available("bwrap");

    Ok(SetupStatus {
        maestro_config_exists: config_file.exists(),
        tracks_dir_exists: tracks_dir.exists(),
        tracks_md_exists: tracks_md.exists(),
        available_tools,
        sandbox_available,
    })
}

/// Default setup recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupRecommendation {
    /// Recommended tool for orchestrate
    pub recommended_tool: Option<String>,
    /// Whether to enable sandbox
    pub recommend_sandbox: bool,
    /// Recommended mode (planning/building)
    pub recommended_mode: String,
}

impl Default for SetupRecommendation {
    fn default() -> Self {
        Self {
            recommended_tool: None,
            recommend_sandbox: false,
            recommended_mode: "building".to_string(),
        }
    }
}

/// Generate setup recommendations based on detected status
pub fn generate_recommendations(status: &SetupStatus) -> SetupRecommendation {
    // Pick the best available tool
    let recommended_tool = status.available_tools.first().map(|t| t.as_str().to_string());

    // Recommend sandbox if available
    let recommend_sandbox = status.sandbox_available;

    SetupRecommendation {
        recommended_tool,
        recommend_sandbox,
        recommended_mode: "building".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_tool_from_str() {
        assert_eq!(AgentTool::from_str("claude"), Some(AgentTool::Claude));
        assert_eq!(AgentTool::from_str("gemini"), Some(AgentTool::Gemini));
        assert_eq!(AgentTool::from_str("unknown"), None);
    }

    #[test]
    fn test_setup_status_minimal() {
        let status = SetupStatus {
            maestro_config_exists: false,
            tracks_dir_exists: true,
            tracks_md_exists: true,
            available_tools: vec![AgentTool::Claude],
            sandbox_available: false,
        };

        assert!(status.is_minimally_configured());
        assert!(!status.is_fully_configured());
    }
}
