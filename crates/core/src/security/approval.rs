//! Approval Manager and Policy Hooks
//!
//! This module implements ZeroClaw-style approval management with:
//! - Tool-level approval requirements registry
//! - Decision recording (approve/reject/always)
//! - "Always" auto-approve behavior
//! - Channel-aware policy entrypoints (CLI-interactive vs non-interactive)
//!
//! Based on ZeroClaw patterns from `analysis_foundation_20260217.md`:
//! - `src/approval/mod.rs:ApprovalManager`
//! - `src/approval/mod.rs:needs_approval`
//! - `src/approval/mod.rs:record_decision`

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Channel type for policy-aware approval decisions.
///
/// Different channels have different approval policies:
/// - **CLI**: Interactive, requires explicit approval
/// - **Telegram/Discord/Slack**: Non-interactive, auto-approves by policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// Interactive CLI channel - requires user approval
    Cli,
    /// Telegram bot channel - non-interactive, auto-approves
    Telegram,
    /// Discord bot channel - non-interactive, auto-approves
    Discord,
    /// Slack app channel - non-interactive, auto-approves
    Slack,
}

impl ChannelType {
    /// Returns the string identifier for this channel type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Telegram => "telegram",
            Self::Discord => "discord",
            Self::Slack => "slack",
        }
    }

    /// Returns true if this is an interactive channel (requires approval).
    pub fn is_interactive(&self) -> bool {
        matches!(self, Self::Cli)
    }

    /// Returns true if this is a non-interactive channel (auto-approves).
    pub fn is_non_interactive(&self) -> bool {
        !self.is_interactive()
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for ChannelType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "cli" => Ok(Self::Cli),
            "telegram" => Ok(Self::Telegram),
            "discord" => Ok(Self::Discord),
            "slack" => Ok(Self::Slack),
            _ => Err(format!("Unknown channel type: {value}")),
        }
    }
}

/// User decision for a tool approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Approve this tool call once
    Approve,
    /// Reject this tool call
    Reject,
    /// Always approve this tool (future auto-approval)
    Always,
}

/// Registry for tools that require approval before execution.
///
/// This defines which tools are considered "sensitive" and need
/// user confirmation before running.
#[derive(Debug, Clone)]
pub struct ToolApprovalRegistry {
    tools: HashSet<String>,
}

impl ToolApprovalRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashSet::new(),
        }
    }

    /// Mark a tool as requiring approval.
    pub fn require_approval(&mut self, tool_name: impl Into<String>) {
        self.tools.insert(tool_name.into());
    }

    /// Mark multiple tools as requiring approval.
    pub fn require_approval_batch(&mut self, tool_names: &[&str]) {
        for tool in tool_names {
            self.tools.insert(tool.to_string());
        }
    }

    /// Remove approval requirement for a tool.
    pub fn remove_approval_requirement(&mut self, tool_name: &str) {
        self.tools.remove(tool_name);
    }

    /// Check if a tool requires approval.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.tools.contains(tool_name)
    }

    /// Get all tools that require approval.
    pub fn approval_required_tools(&self) -> Vec<String> {
        self.tools.iter().cloned().collect()
    }
}

impl Default for ToolApprovalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Approval manager for tool execution decisions.
///
/// This manager handles:
/// - Checking if a tool needs approval
/// - Recording user decisions (approve/reject/always)
/// - Applying "always" auto-approve rules (channel-specific)
/// - Channel-aware policy resolution
///
/// Thread-safe: uses Arc<RwLock<>> for interior mutability.
#[derive(Debug, Clone)]
pub struct ApprovalManager {
    registry: ToolApprovalRegistry,
    // Maps (tool_name, channel_id) -> ApprovalDecision
    decisions: Arc<RwLock<HashMap<(String, String), ApprovalDecision>>>,
}

impl ApprovalManager {
    /// Create a new approval manager with the given registry.
    pub fn new(registry: ToolApprovalRegistry) -> Self {
        Self {
            registry,
            decisions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new approval manager with an empty registry.
    pub fn new_empty() -> Self {
        Self::new(ToolApprovalRegistry::new())
    }

    /// Check if a tool requires approval for the given channel.
    ///
    /// Returns `false` if:
    /// - Tool is not in the approval registry
    /// - Channel is non-interactive (Telegram/Discord/Slack)
    /// - Tool has "always" approval recorded for this channel
    ///
    /// Returns `true` if:
    /// - Tool is in registry AND channel is interactive (CLI) AND no "always" decision for this channel
    pub fn needs_approval(&self, tool_name: &str, channel: ChannelType) -> bool {
        // Non-interactive channels auto-approve by policy
        if channel.is_non_interactive() {
            return false;
        }

        // Check for "always" approval for this specific channel
        if self.is_always_approved_for_channel(tool_name, channel) {
            return false;
        }

        // Check registry
        self.registry.requires_approval(tool_name)
    }

    /// Record a user's approval decision.
    ///
    /// This decision will be used for future approval checks.
    /// Decisions are channel-specific - an "always" for CLI doesn't affect Telegram.
    pub fn record_decision(
        &self,
        tool_name: impl Into<String>,
        channel: ChannelType,
        decision: ApprovalDecision,
    ) {
        let tool_name = tool_name.into();
        let channel_id = channel.as_str().to_string();

        // Store the decision
        let mut decisions = self.decisions.write().unwrap();
        decisions.insert((tool_name, channel_id), decision);
    }

    /// Get a recorded decision for a tool/channel combination.
    pub fn get_decision(&self, tool_name: &str, channel: ChannelType) -> Option<ApprovalDecision> {
        let decisions = self.decisions.read().unwrap();
        let channel_id = channel.as_str();
        decisions
            .get(&(tool_name.to_string(), channel_id.to_string()))
            .copied()
    }

    /// Check if there's a recorded decision for a tool/channel.
    pub fn has_decision_for(&self, tool_name: &str, channel_id: &str) -> bool {
        let decisions = self.decisions.read().unwrap();
        decisions.contains_key(&(tool_name.to_string(), channel_id.to_string()))
    }

    /// Check if a tool has "always" approval recorded for a specific channel.
    fn is_always_approved_for_channel(&self, tool_name: &str, channel: ChannelType) -> bool {
        let decisions = self.decisions.read().unwrap();
        let channel_id = channel.as_str();
        decisions.get(&(tool_name.to_string(), channel_id.to_string()))
            == Some(&ApprovalDecision::Always)
    }

    /// Check if a tool has "always" approval recorded (any channel).
    pub fn is_always_approved(&self, tool_name: &str) -> bool {
        let decisions = self.decisions.read().unwrap();
        decisions
            .iter()
            .any(|((name, _), decision)| name == tool_name && *decision == ApprovalDecision::Always)
    }

    /// Check if a tool should be auto-approved for the given channel.
    ///
    /// Returns true only if the tool has an explicit "always" decision recorded for THIS channel.
    /// Note: This is different from `needs_approval` which handles non-interactive channels.
    pub fn should_auto_approve(&self, tool_name: &str, channel: ChannelType) -> bool {
        // Only check for channel-specific "always" approval
        self.is_always_approved_for_channel(tool_name, channel)
    }

    /// Clear a decision for a specific tool/channel combination.
    pub fn clear_decision(&self, tool_name: &str, channel: ChannelType) {
        let channel_id = channel.as_str();
        let mut decisions = self.decisions.write().unwrap();
        decisions.remove(&(tool_name.to_string(), channel_id.to_string()));
    }

    /// Clear all decisions for a specific tool (across all channels).
    pub fn clear_all_for_tool(&self, tool_name: &str) {
        let mut decisions = self.decisions.write().unwrap();
        // retain takes FnMut(&K, &V) where K is (String, String)
        decisions.retain(|key, _value| key.0 != tool_name);
    }

    /// Get the underlying registry for modifying approval requirements.
    pub fn registry(&self) -> &ToolApprovalRegistry {
        &self.registry
    }

    /// Get mutable reference to the registry.
    pub fn registry_mut(&mut self) -> &mut ToolApprovalRegistry {
        &mut self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_is_interactive() {
        assert!(ChannelType::Cli.is_interactive());
        assert!(!ChannelType::Telegram.is_interactive());
        assert!(!ChannelType::Discord.is_interactive());
        assert!(!ChannelType::Slack.is_interactive());
    }

    #[test]
    fn test_tool_registry_new() {
        let registry = ToolApprovalRegistry::new();
        assert!(!registry.requires_approval("test"));
    }

    #[test]
    fn test_tool_registry_add() {
        let mut registry = ToolApprovalRegistry::new();
        registry.require_approval("test");
        assert!(registry.requires_approval("test"));
    }

    #[test]
    fn test_approval_manager_new_empty() {
        let manager = ApprovalManager::new_empty();
        assert!(!manager.needs_approval("test", ChannelType::Cli));
    }
}
