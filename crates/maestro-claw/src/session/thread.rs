//! Thread model - Conversation branch
//!
//! A Thread represents a sequence of related turns within a session,
//! with optional summarization support for long conversations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Turn, TurnRole};

/// Default threshold for auto-summarization
const DEFAULT_SUMMARY_THRESHOLD: usize = 20;

/// Serde default function for summary_threshold
fn default_summary_threshold() -> usize {
    DEFAULT_SUMMARY_THRESHOLD
}

/// Message format for provider APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMessage {
    /// Role: "user", "assistant", "system", or "tool"
    pub role: String,
    /// Message content
    pub content: String,
    /// Tool calls (for assistant messages with tool calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallMessage>>,
    /// Tool call ID (for tool response messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Tool call in provider message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMessage {
    /// Tool call ID
    pub id: String,
    /// Tool type (usually "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function details
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
    /// Function arguments as JSON string
    pub arguments: String,
}

/// A conversation thread containing ordered turns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Unique identifier for this thread
    pub id: String,
    /// ID of the parent session
    pub session_id: String,
    /// Ordered list of turns in this thread
    pub turns: Vec<Turn>,
    /// Optional summary of the conversation
    #[serde(default)]
    pub summary: Option<String>,
    /// When this thread was created
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Turn count threshold for requesting summarization
    #[serde(default = "default_summary_threshold")]
    summary_threshold: usize,
}

impl Thread {
    /// Create a new thread belonging to the given session
    pub fn new(session_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            turns: Vec::new(),
            summary: None,
            created_at: Utc::now(),
            summary_threshold: DEFAULT_SUMMARY_THRESHOLD,
        }
    }

    /// Create a new thread with a specific ID
    pub fn with_id(id: String, session_id: String) -> Self {
        Self {
            id,
            session_id,
            turns: Vec::new(),
            summary: None,
            created_at: Utc::now(),
            summary_threshold: DEFAULT_SUMMARY_THRESHOLD,
        }
    }

    /// Get the thread ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get an iterator over the turns
    pub fn turns(&self) -> impl Iterator<Item = &Turn> {
        self.turns.iter()
    }

    /// Get the number of turns
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Get the summary, if any
    pub fn summary(&self) -> Option<&String> {
        self.summary.as_ref()
    }

    /// Set the summary
    pub fn set_summary(&mut self, summary: String) {
        self.summary = Some(summary);
    }

    /// Get the current summary threshold
    pub fn summary_threshold(&self) -> usize {
        self.summary_threshold
    }

    /// Set the summary threshold
    pub fn set_summary_threshold(&mut self, threshold: usize) {
        self.summary_threshold = threshold;
    }

    /// Check if this thread needs summarization
    pub fn needs_summary(&self) -> bool {
        self.turns.len() >= self.summary_threshold
    }

    /// Add a turn to this thread
    pub fn add_turn(&mut self, turn: Turn) {
        self.turns.push(turn);
    }

    /// Build and add a new turn, returning a reference to it
    pub fn build_next_turn(&mut self, role: TurnRole, content: String) -> &Turn {
        let turn = Turn::new(role, content);
        self.turns.push(turn);
        self.turns.last().unwrap()
    }

    /// Trim old turns to keep only the most recent `keep` turns (Rec-6).
    ///
    /// This is called after the summary is generated to prevent unbounded
    /// context growth.  At least 1 turn is always retained.
    pub fn trim_old_turns(&mut self, keep: usize) {
        let keep = keep.max(1);
        if self.turns.len() > keep {
            // Preserve the first turn if it's a System message (system prompt)
            let skip = if !self.turns.is_empty() && matches!(self.turns[0].role, TurnRole::System) {
                1
            } else {
                0
            };
            let total_to_keep = keep + skip;
            if self.turns.len() > total_to_keep {
                let drain_to = self.turns.len() - keep;
                self.turns.drain(skip..drain_to);
            }
        }
    }

    /// Convert thread history to provider message format (Rec-6)
    ///
    /// This creates messages suitable for sending to LLM providers,
    /// including proper formatting of tool calls and results.
    ///
    /// When a summary is set (i.e. after the thread was trimmed), the summary
    /// is prepended as a `system` message so the provider has full context
    /// even after old turns were discarded.
    pub fn to_messages(&self) -> Vec<ProviderMessage> {
        // Rec-6: prepend summary as a system context message when present
        let mut messages: Vec<ProviderMessage> = if let Some(ref summary) = self.summary {
            vec![ProviderMessage {
                role: "system".to_string(),
                content: format!("Conversation context summary: {}", summary),
                tool_calls: None,
                tool_call_id: None,
            }]
        } else {
            Vec::with_capacity(self.turns.len())
        };

        for turn in &self.turns {
            let msg = match turn.role {
                TurnRole::System => ProviderMessage {
                    role: "system".to_string(),
                    content: turn.content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                TurnRole::User => ProviderMessage {
                    role: "user".to_string(),
                    content: turn.content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                TurnRole::Assistant => {
                    // If there are tool calls, include them
                    let tool_calls = if turn.tool_calls.is_empty() {
                        None
                    } else {
                        Some(
                            turn.tool_calls
                                .iter()
                                .map(|tc| ToolCallMessage {
                                    id: tc.id.clone(),
                                    tool_type: "function".to_string(),
                                    function: FunctionCall {
                                        name: tc.name.clone(),
                                        arguments: tc.arguments.to_string(),
                                    },
                                })
                                .collect(),
                        )
                    };

                    ProviderMessage {
                        role: "assistant".to_string(),
                        content: turn.content.clone(),
                        tool_calls,
                        tool_call_id: None,
                    }
                }
                TurnRole::Tool => ProviderMessage {
                    role: "tool".to_string(),
                    content: turn.content.clone(),
                    tool_calls: None,
                    tool_call_id: turn.tool_call_id.clone(),
                },
            };
            messages.push(msg);
        }

        messages
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_creation() {
        let thread = Thread::new("session-123".to_string());
        assert!(!thread.id().is_empty());
        assert_eq!(thread.session_id(), "session-123");
        assert_eq!(thread.turn_count(), 0);
    }

    #[test]
    fn test_thread_add_turn() {
        let mut thread = Thread::new("session-123".to_string());
        let turn = Turn::new(TurnRole::User, "Hello".to_string());
        thread.add_turn(turn);
        assert_eq!(thread.turn_count(), 1);
    }

    #[test]
    fn test_thread_to_messages() {
        let mut thread = Thread::new("session-123".to_string());
        thread.build_next_turn(TurnRole::System, "You are helpful".to_string());
        thread.build_next_turn(TurnRole::User, "Hello".to_string());

        let messages = thread.to_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_summary_threshold_persisted_through_serde() {
        let mut thread = Thread::new("session-123".to_string());
        thread.set_summary_threshold(5);

        let json = serde_json::to_string(&thread).unwrap();
        let restored: Thread = serde_json::from_str(&json).unwrap();

        // Custom threshold must survive serde roundtrip
        assert!(
            restored.needs_summary() == (restored.turn_count() >= 5),
            "summary_threshold must be persisted through serde, not reset to default"
        );
        // Verify the threshold value is preserved:
        // With threshold=5 → needs_summary() is true after exactly 5 turns.
        // With the old default threshold=20 it would still be false → proves
        // the custom value was correctly persisted through the serde roundtrip.
        let mut restored = restored;
        for _ in 0..5 {
            restored.build_next_turn(TurnRole::User, "x".to_string());
        }
        assert!(
            restored.needs_summary(),
            "restored threshold should be 5, not default 20"
        );
    }

    #[test]
    fn test_thread_needs_summary() {
        let mut thread = Thread::new("session-123".to_string());
        thread.set_summary_threshold(3);

        thread.build_next_turn(TurnRole::User, "1".to_string());
        thread.build_next_turn(TurnRole::Assistant, "2".to_string());
        assert!(!thread.needs_summary());

        thread.build_next_turn(TurnRole::User, "3".to_string());
        assert!(thread.needs_summary());
    }
}
