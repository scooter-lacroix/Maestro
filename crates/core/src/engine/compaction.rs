//! Context Compaction + Retry-on-Overflow
//!
//! This module implements IronClaw-style context compaction:
//! - Detect context window overflow
//! - Compact messages with summary insertion
//! - Single retry semantics after compaction
//!
//! Based on IronClaw patterns from `analysis_foundation_20260217.md`:
//! - `src/agent/loop_.rs:compact_context_on_overflow`
//! - `src/agent/loop_.rs:retry_after_compaction`

use crate::traits::Message;
use std::sync::atomic::{AtomicU32, Ordering};

/// Configuration for context compaction.
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Maximum tokens allowed in context.
    max_tokens: usize,
    /// Tokens to reserve for summary.
    summary_tokens: usize,
    /// Maximum retry attempts after compaction.
    max_retries: u32,
}

impl CompactionConfig {
    /// Create a new compaction config.
    pub fn new(max_tokens: usize, summary_tokens: usize, max_retries: u32) -> Self {
        Self {
            max_tokens,
            summary_tokens,
            max_retries,
        }
    }

    /// Get the maximum tokens allowed.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Get the summary token reserve.
    pub fn summary_tokens(&self) -> usize {
        self.summary_tokens
    }

    /// Get the maximum retry attempts.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            // Default to 128k tokens (Claude-style context window)
            max_tokens: 128_000,
            // Reserve 1000 tokens for summary
            summary_tokens: 1000,
            // Single retry allowed
            max_retries: 1,
        }
    }
}

/// Strategy for compacting context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionStrategy {
    /// Truncate oldest messages.
    Truncate,
    /// Summarize and replace old messages.
    Summarize,
}

impl CompactionStrategy {
    /// Get the name of this strategy.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Truncate => "truncate",
            Self::Summarize => "summarize",
        }
    }
}

/// Result of a compaction operation.
#[derive(Debug, Clone)]
pub enum CompactionResult {
    /// No compaction was needed.
    NoCompactionNeeded,
    /// Compaction was performed.
    Compacted {
        /// The compacted message list.
        compacted: Vec<Message>,
        /// Optional summary that was inserted.
        summary: Option<String>,
    },
    /// Compaction failed.
    Failed(String),
}

/// Context compactor with retry tracking.
///
/// This compactor handles:
/// - Detecting when context exceeds limits
/// - Compacting with configurable strategies
/// - Tracking retry attempts
#[derive(Debug)]
pub struct ContextCompactor {
    config: CompactionConfig,
    retry_count: AtomicU32,
}

impl ContextCompactor {
    /// Create a new context compactor.
    pub fn new(config: CompactionConfig) -> Self {
        Self {
            config,
            retry_count: AtomicU32::new(0),
        }
    }

    /// Check if compaction is needed for the given messages.
    pub fn needs_compaction(&self, messages: &[Message]) -> bool {
        let total_tokens = self.estimate_total_tokens(messages);
        total_tokens > self.config.max_tokens
    }

    /// Estimate the total tokens in the messages.
    fn estimate_total_tokens(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum()
    }

    /// Estimate tokens in a single message.
    fn estimate_message_tokens(&self, message: &Message) -> usize {
        // Rough estimate: ~4 characters per token
        // Plus overhead for role, timestamp, etc.
        let content_tokens = self.estimate_tokens(&message.content);
        let role_tokens = message.role.len() / 4 + 1;
        content_tokens + role_tokens + 5 // overhead
    }

    /// Estimate tokens in a string.
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // Simple heuristic: ~4 characters per token
        text.len().div_ceil(4)
    }

    /// Compact the messages using the default strategy.
    pub fn compact(&self, messages: &[Message]) -> CompactionResult {
        self.compact_with_strategy(messages, CompactionStrategy::Summarize)
    }

    /// Compact the messages using a specific strategy.
    pub fn compact_with_strategy(
        &self,
        messages: &[Message],
        strategy: CompactionStrategy,
    ) -> CompactionResult {
        if !self.needs_compaction(messages) {
            return CompactionResult::NoCompactionNeeded;
        }

        match strategy {
            CompactionStrategy::Truncate => self.truncate_messages(messages),
            CompactionStrategy::Summarize => self.summarize_messages(messages),
        }
    }

    /// Truncate oldest messages to fit within limit.
    fn truncate_messages(&self, messages: &[Message]) -> CompactionResult {
        let target_tokens = self.config.max_tokens - self.config.summary_tokens;

        // Find how many messages we can keep
        let mut kept_tokens = 0;
        let mut kept_messages = Vec::new();

        // Keep system messages first
        for msg in messages {
            if msg.role == "system" {
                let tokens = self.estimate_message_tokens(msg);
                if kept_tokens + tokens <= target_tokens {
                    kept_messages.push(msg.clone());
                    kept_tokens += tokens;
                }
            }
        }

        // Then keep recent messages
        for msg in messages.iter().rev() {
            if msg.role == "system" {
                continue; // Already handled
            }
            let tokens = self.estimate_message_tokens(msg);
            if kept_tokens + tokens <= target_tokens {
                kept_messages.insert(
                    kept_messages.len()
                        - kept_messages.iter().filter(|m| m.role != "system").count(),
                    msg.clone(),
                );
                kept_tokens += tokens;
            } else {
                break;
            }
        }

        // Create summary of truncated content
        let truncated_count = messages.len() - kept_messages.len();
        let summary = if truncated_count > 0 {
            Some(format!("[{} earlier messages truncated]", truncated_count))
        } else {
            None
        };

        // Insert summary at the beginning (after system messages)
        if let Some(ref summary_text) = summary {
            let summary_msg = Message {
                role: "system".to_string(),
                content: summary_text.clone(),
                timestamp: chrono::Utc::now(),
            };
            // Insert after existing system messages
            let insert_pos = kept_messages
                .iter()
                .take_while(|m| m.role == "system")
                .count();
            kept_messages.insert(insert_pos, summary_msg);
        }

        CompactionResult::Compacted {
            compacted: kept_messages,
            summary,
        }
    }

    /// Summarize old messages and replace them.
    fn summarize_messages(&self, messages: &[Message]) -> CompactionResult {
        let target_tokens = self.config.max_tokens - self.config.summary_tokens;

        // Find how many messages we can keep
        let mut kept_tokens = 0;
        let mut kept_messages = Vec::new();
        let mut summarized_messages = Vec::new();

        // Keep system messages
        for msg in messages {
            if msg.role == "system" {
                kept_messages.push(msg.clone());
            }
        }

        // Calculate which messages to summarize vs keep
        let non_system: Vec<_> = messages.iter().filter(|m| m.role != "system").collect();

        // Keep recent messages, summarize older ones
        for (_i, msg) in non_system.iter().enumerate().rev() {
            let tokens = self.estimate_message_tokens(msg);
            if kept_tokens + tokens <= target_tokens {
                kept_messages.push((*msg).clone());
                kept_tokens += tokens;
            } else {
                summarized_messages.push((*msg).clone());
            }
        }

        // Reverse to maintain order
        kept_messages.reverse();

        // Create summary
        let summary = if !summarized_messages.is_empty() {
            Some(format!(
                "[Summary of {} earlier messages: {} conversation turns omitted for context window limits]",
                summarized_messages.len(),
                summarized_messages.len() / 2
            ))
        } else {
            None
        };

        // Insert summary
        if let Some(ref summary_text) = summary {
            let summary_msg = Message {
                role: "system".to_string(),
                content: summary_text.clone(),
                timestamp: chrono::Utc::now(),
            };
            let insert_pos = kept_messages
                .iter()
                .take_while(|m| m.role == "system")
                .count();
            kept_messages.insert(insert_pos, summary_msg);
        }

        CompactionResult::Compacted {
            compacted: kept_messages,
            summary,
        }
    }

    /// Get the current retry count.
    pub fn retry_count(&self) -> u32 {
        self.retry_count.load(Ordering::SeqCst)
    }

    /// Check if a retry has been attempted.
    pub fn has_retried(&self) -> bool {
        self.retry_count() > 0
    }

    /// Increment the retry counter.
    pub fn increment_retry(&self) {
        self.retry_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Reset the retry counter.
    pub fn reset_retry(&self) {
        self.retry_count.store(0, Ordering::SeqCst);
    }

    /// Check if another retry is allowed.
    pub fn can_retry(&self) -> bool {
        self.retry_count() < self.config.max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CompactionConfig::default();
        assert!(config.max_tokens() > 0);
    }

    #[test]
    fn test_compactor_new() {
        let compactor = ContextCompactor::new(CompactionConfig::default());
        assert_eq!(compactor.retry_count(), 0);
    }

    #[test]
    fn test_needs_compaction_empty() {
        let compactor = ContextCompactor::new(CompactionConfig::default());
        assert!(!compactor.needs_compaction(&[]));
    }
}
