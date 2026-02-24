//! Agent Status Display Types for Cockpit TUI
//!
//! This module provides types for displaying agent status and session information
//! in the MaesterClaw tab of the Cockpit TUI.

use serde::{Deserialize, Serialize};

/// Maximum preview length for turn content
const MAX_PREVIEW_LENGTH: usize = 60;

/// Agent status for display in TUI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is ready to accept requests
    Ready,
    /// Agent is currently running a session
    Running {
        session_id: String,
        turn_count: usize,
    },
    /// Agent is idle (no active session)
    Idle,
    /// Agent encountered an error
    Error {
        message: String,
    },
}

impl AgentStatus {
    /// Create a new ready status
    pub fn ready() -> Self {
        Self::Ready
    }

    /// Create a running status
    pub fn running(session_id: String, turn_count: usize) -> Self {
        Self::Running {
            session_id,
            turn_count,
        }
    }

    /// Create an idle status
    pub fn idle() -> Self {
        Self::Idle
    }

    /// Create an error status
    pub fn error(message: String) -> Self {
        Self::Error { message }
    }

    /// Get display label for this status
    pub fn label(&self) -> &str {
        match self {
            Self::Ready => "Ready",
            Self::Running { .. } => "Running",
            Self::Idle => "Idle",
            Self::Error { .. } => "Error",
        }
    }

    /// Check if the agent is active (ready or running)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Ready | Self::Running { .. })
    }

    /// Check if the agent is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    /// Get icon for this status (for TUI display)
    pub fn icon(&self) -> &str {
        match self {
            Self::Ready => "●",
            Self::Running { .. } => "◐",
            Self::Idle => "○",
            Self::Error { .. } => "✗",
        }
    }
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Session display information for TUI list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDisplay {
    /// Session ID
    pub id: String,
    /// Display title (truncated)
    pub title: String,
    /// Number of threads
    pub thread_count: usize,
    /// Total turns across all threads
    pub turn_count: usize,
    /// Session status
    pub status: String,
    /// When created
    pub created_at: String,
    /// Whether this session is selected
    pub is_selected: bool,
}

impl SessionDisplay {
    /// Create a new session display
    pub fn new(
        id: String,
        title: String,
        thread_count: usize,
        turn_count: usize,
        status: String,
        created_at: String,
    ) -> Self {
        // Truncate title if needed
        let title = if title.len() > MAX_PREVIEW_LENGTH {
            format!("{}...", &title[..MAX_PREVIEW_LENGTH.saturating_sub(3)])
        } else {
            title
        };

        Self {
            id,
            title,
            thread_count,
            turn_count,
            status,
            created_at,
            is_selected: false,
        }
    }

    /// Format for list display
    pub fn format_list_item(&self) -> String {
        format!(
            "{} {} ({}) - {} turns",
            if self.is_selected { ">" } else { " " },
            self.title,
            self.status,
            self.turn_count
        )
    }
}

/// Turn display information for TUI history view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnDisplay {
    /// Turn role (User, Assistant, System, Tool)
    pub role: String,
    /// Content preview (truncated)
    pub preview: String,
    /// Whether this turn has tool calls
    pub has_tool_calls: bool,
    /// Number of tool calls
    pub tool_call_count: usize,
    /// Names of tools called
    pub tool_names: Vec<String>,
    /// Turn index in thread
    pub index: usize,
}

impl TurnDisplay {
    /// Create from turn data
    pub fn new(
        role: String,
        content: &str,
        tool_calls: &[ToolCallInfo],
        index: usize,
    ) -> Self {
        let preview = if content.len() > MAX_PREVIEW_LENGTH {
            format!("{}...", &content[..MAX_PREVIEW_LENGTH.saturating_sub(3)])
        } else {
            content.to_string()
        };

        let has_tool_calls = !tool_calls.is_empty();
        let tool_call_count = tool_calls.len();
        let tool_names = tool_calls.iter().map(|tc| tc.name.clone()).collect();

        Self {
            role,
            preview,
            has_tool_calls,
            tool_call_count,
            tool_names,
            index,
        }
    }

    /// Get role icon for display
    pub fn role_icon(&self) -> &str {
        match self.role.as_str() {
            "User" => "?",
            "Assistant" => "!",
            "System" => "#",
            "Tool" => "$",
            _ => "-",
        }
    }

    /// Format for history display
    pub fn format_history_item(&self) -> String {
        let tool_indicator = if self.has_tool_calls {
            format!(" [{} tool(s)]", self.tool_call_count)
        } else {
            String::new()
        };
        format!(
            "{} {}: {}{}",
            self.index,
            self.role,
            self.preview,
            tool_indicator
        )
    }
}

/// Tool call information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Arguments preview
    pub arguments_preview: String,
}

impl ToolCallInfo {
    /// Create a new tool call info
    pub fn new(id: String, name: String, arguments: &str) -> Self {
        let arguments_preview = if arguments.len() > 40 {
            format!("{}...", &arguments[..37])
        } else {
            arguments.to_string()
        };

        Self {
            id,
            name,
            arguments_preview,
        }
    }
}

/// Session statistics for display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total turns across all threads
    pub total_turns: usize,
    /// Number of user turns
    pub user_turns: usize,
    /// Number of assistant turns
    pub assistant_turns: usize,
    /// Number of tool turns
    pub tool_turns: usize,
    /// Number of tool calls
    pub tool_calls: usize,
}

impl SessionStats {
    /// Create empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a turn count
    pub fn add_turn(&mut self, role: &str, tool_call_count: usize) {
        self.total_turns += 1;
        match role {
            "User" | "user" => self.user_turns += 1,
            "Assistant" | "assistant" => self.assistant_turns += 1,
            "Tool" | "tool" => self.tool_turns += 1,
            _ => {}
        }
        self.tool_calls += tool_call_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_label() {
        assert_eq!(AgentStatus::ready().label(), "Ready");
        assert_eq!(AgentStatus::idle().label(), "Idle");
        assert_eq!(AgentStatus::running("s1".into(), 5).label(), "Running");
        assert_eq!(AgentStatus::error("err".into()).label(), "Error");
    }

    #[test]
    fn test_agent_status_is_active() {
        assert!(AgentStatus::ready().is_active());
        assert!(AgentStatus::running("s1".into(), 5).is_active());
        assert!(!AgentStatus::idle().is_active());
        assert!(!AgentStatus::error("err".into()).is_active());
    }

    #[test]
    fn test_agent_status_icons() {
        assert_eq!(AgentStatus::ready().icon(), "●");
        assert_eq!(AgentStatus::running("s1".into(), 5).icon(), "◐");
        assert_eq!(AgentStatus::idle().icon(), "○");
        assert_eq!(AgentStatus::error("err".into()).icon(), "✗");
    }

    #[test]
    fn test_session_display_truncation() {
        let long_title = "This is a very long session title that should be truncated for display";
        let display = SessionDisplay::new(
            "id".into(),
            long_title.into(),
            1,
            5,
            "active".into(),
            "2026-02-23".into(),
        );

        assert!(display.title.len() <= 63);
        assert!(display.title.ends_with("..."));
    }

    #[test]
    fn test_turn_display() {
        let turn = TurnDisplay::new(
            "Assistant".into(),
            "Let me check that file for you.",
            &[ToolCallInfo::new("c1".into(), "bash".into(), r#"{"cmd":"ls"}"#)],
            0,
        );

        assert_eq!(turn.role, "Assistant");
        assert!(turn.has_tool_calls);
        assert_eq!(turn.tool_call_count, 1);
        assert_eq!(turn.tool_names, vec!["bash"]);
    }

    #[test]
    fn test_session_stats() {
        let mut stats = SessionStats::new();
        stats.add_turn("User", 0);
        stats.add_turn("Assistant", 2);
        stats.add_turn("Tool", 0);

        assert_eq!(stats.total_turns, 3);
        assert_eq!(stats.user_turns, 1);
        assert_eq!(stats.assistant_turns, 1);
        assert_eq!(stats.tool_turns, 1);
        assert_eq!(stats.tool_calls, 2);
    }
}
