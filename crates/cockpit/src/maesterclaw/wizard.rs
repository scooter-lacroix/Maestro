//! MaestroClaw Setup Wizard
//!
//! This module provides the setup wizard functionality for MaestroClaw,
//! including tool detection and configuration guidance.



/// Setup wizard state for MaestroClaw
#[derive(Debug, Clone)]
pub struct SetupWizard {
    /// List of available tools detected on the system
    pub available_tools: Vec<String>,
    /// Whether the wizard has been dismissed
    dismissed: bool,
    /// Whether the wizard has been completed
    completed: bool,
    /// Current step in the wizard
    current_step: WizardStep,
}

/// Steps in the setup wizard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    /// Initial welcome screen
    Welcome,
    /// Tool detection screen
    ToolDetection,
    /// Configuration screen
    Configuration,
    /// Completion screen
    Complete,
}

impl SetupWizard {
    /// Create a new setup wizard
    pub fn new() -> Self {
        Self {
            available_tools: Vec::new(),
            dismissed: false,
            completed: false,
            current_step: WizardStep::Welcome,
        }
    }

    /// Detect available tools on the system
    pub fn detect_tools(&mut self) {
        // Detect common CLI tools
        let tools = [
            "claude", "claude-code", "gemini", "codex", "qwen", "amp",
            "git", "gh", "docker", "kubectl", "npm", "yarn", "pnpm",
            "cargo", "rustc", "python", "python3", "pip", "uv",
            "node", "bun", "deno", "go", "rustc",
        ];

        let mut detected = Vec::new();
        for tool in &tools {
            if which::which(tool).is_ok() {
                detected.push(tool.to_string());
            }
        }

        self.available_tools = detected;
    }

    /// Advance to the next step
    pub fn next_step(&mut self) {
        self.current_step = match self.current_step {
            WizardStep::Welcome => WizardStep::ToolDetection,
            WizardStep::ToolDetection => WizardStep::Configuration,
            WizardStep::Configuration => {
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
            WizardStep::Configuration => WizardStep::ToolDetection,
            WizardStep::Complete => WizardStep::Configuration,
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
        self.dismissed = false;
        self.completed = false;
        self.current_step = WizardStep::Welcome;
        self.available_tools.clear();
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new()
    }
}