//! ProviderAdapter — bridges the rich `providers::Provider` trait to the
//! simplified `agent::Provider` trait used by the agent loop.
//!
//! MED-6 / Rec-1: This adapter eliminates the need to re-implement the
//! agent loop interface for every provider. Any type that implements the
//! full `providers::Provider` trait can be wrapped here and used directly
//! with `agent_loop`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::providers::{self, ProviderCapabilities};
use crate::session::{ProviderMessage, Turn, TurnRole};
use crate::tools::ToolSpec;

use super::r#loop::{AgentError, Provider, ProviderResponse};

/// Adapts a `providers::Provider` implementation for use in `agent_loop`.
///
/// The adapter:
/// 1. Converts `Vec<ProviderMessage>` (agent loop format) → `Vec<Turn>` (provider format).
/// 2. Selects `chat_with_tools` when tools are available and the provider
///    supports native tools; otherwise falls back to `chat`.
/// 3. Converts `ChatResponse` back to `ProviderResponse`.
pub struct ProviderAdapter {
    inner: Arc<dyn providers::Provider>,
}

impl ProviderAdapter {
    /// Wrap any `providers::Provider` implementation.
    pub fn new(provider: Arc<dyn providers::Provider>) -> Self {
        Self { inner: provider }
    }

    /// Access the underlying rich provider.
    pub fn inner(&self) -> &dyn providers::Provider {
        self.inner.as_ref()
    }

    /// Return the underlying provider capabilities.
    pub fn capabilities(&self) -> ProviderCapabilities {
        self.inner.capabilities()
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert an agent-loop `ProviderMessage` back into a session `Turn`.
///
/// This is the inverse of `Thread::to_messages()`.
fn provider_message_to_turn(msg: &ProviderMessage) -> Turn {
    let role = match msg.role.as_str() {
        "assistant" => TurnRole::Assistant,
        "system" => TurnRole::System,
        "tool" => TurnRole::Tool,
        _ => TurnRole::User,
    };

    let mut turn = Turn::new(role, msg.content.clone());

    // Restore tool calls on assistant messages
    if let Some(ref tcs) = msg.tool_calls {
        for tc in tcs {
            let args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_default();
            turn.add_tool_call(tc.id.clone(), tc.function.name.clone(), args);
        }
    }

    // Restore tool call ID on tool-result messages
    if let Some(ref id) = msg.tool_call_id {
        turn.set_tool_call_id(id.clone());
    }

    turn
}

/// Convert a `providers::ChatResponse` into the agent loop `ProviderResponse`.
fn chat_response_to_provider(resp: providers::ChatResponse) -> ProviderResponse {
    let is_finished = resp.tool_calls.is_empty();
    ProviderResponse {
        content: resp.content,
        tool_calls: resp.tool_calls,
        is_finished,
    }
}

// ---------------------------------------------------------------------------
// Provider impl
// ---------------------------------------------------------------------------

#[async_trait]
impl Provider for ProviderAdapter {
    async fn execute(
        &self,
        messages: Vec<ProviderMessage>,
        tools: Vec<ToolSpec>,
    ) -> Result<ProviderResponse, AgentError> {
        // Convert messages to the Turn-based format expected by the rich provider
        let turns: Vec<Turn> = messages.iter().map(provider_message_to_turn).collect();

        let chat_response = if tools.is_empty() || !self.inner.capabilities().native_tools {
            // No tools or provider doesn't support native tools — plain chat
            self.inner
                .chat(&turns)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))?
        } else {
            self.inner
                .chat_with_tools(&turns, &tools)
                .await
                .map_err(|e| AgentError::ProviderError(e.to_string()))?
        };

        Ok(chat_response_to_provider(chat_response))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Turn, TurnRole};
    use futures::Stream;

    // ---- minimal stub Provider ----

    struct StubProvider {
        response_content: String,
        supports_tools: bool,
    }

    #[async_trait]
    impl providers::Provider for StubProvider {
        fn name(&self) -> &str {
            "stub"
        }
        fn model(&self) -> &str {
            "stub-model"
        }

        fn capabilities(&self) -> providers::ProviderCapabilities {
            let mut caps = providers::ProviderCapabilities::none();
            caps.native_tools = self.supports_tools;
            caps
        }

        async fn chat(
            &self,
            _: &[Turn],
        ) -> Result<providers::ChatResponse, providers::ProviderError> {
            Ok(providers::ChatResponse::text(self.response_content.clone()))
        }

        async fn stream_chat(
            &self,
            _: &[Turn],
        ) -> Result<
            Box<
                dyn Stream<Item = Result<providers::StreamChunk, providers::ProviderError>>
                    + Send
                    + Unpin,
            >,
            providers::ProviderError,
        > {
            unimplemented!("not needed in tests")
        }

        async fn chat_with_tools(
            &self,
            _: &[Turn],
            _: &[ToolSpec],
        ) -> Result<providers::ChatResponse, providers::ProviderError> {
            Ok(providers::ChatResponse::text(format!(
                "{} (with tools)",
                self.response_content
            )))
        }

        async fn warmup(&self) -> Result<(), providers::ProviderError> {
            Ok(())
        }
        async fn health_check(&self) -> Result<(), providers::ProviderError> {
            Ok(())
        }

        fn format_messages(&self, _: &[Turn]) -> Vec<serde_json::Value> {
            vec![]
        }
    }

    fn make_adapter(content: &str, supports_tools: bool) -> ProviderAdapter {
        ProviderAdapter::new(Arc::new(StubProvider {
            response_content: content.to_string(),
            supports_tools,
        }))
    }

    #[tokio::test]
    async fn test_adapter_plain_chat() {
        let adapter = make_adapter("Hello", false);
        let result = adapter
            .execute(
                vec![ProviderMessage {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
                vec![],
            )
            .await
            .unwrap();

        assert_eq!(result.content, "Hello");
        assert!(result.is_finished);
        assert!(result.tool_calls.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_with_tools_uses_chat_with_tools() {
        let adapter = make_adapter("Answer", true);
        let tool_spec = ToolSpec {
            name: "fake".to_string(),
            description: "a fake tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        };

        let result = adapter
            .execute(
                vec![ProviderMessage {
                    role: "user".to_string(),
                    content: "Use a tool".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
                vec![tool_spec],
            )
            .await
            .unwrap();

        assert!(result.content.contains("with tools"));
    }

    #[tokio::test]
    async fn test_adapter_without_native_tools_uses_plain_chat() {
        // Provider supports_tools=false but we pass tool specs → should fall back to chat
        let adapter = make_adapter("Plain", false);
        let tool_spec = ToolSpec {
            name: "fake".to_string(),
            description: "a fake tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        };

        let result = adapter
            .execute(
                vec![ProviderMessage {
                    role: "user".to_string(),
                    content: "Use a tool".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
                vec![tool_spec],
            )
            .await
            .unwrap();

        assert_eq!(result.content, "Plain"); // chat() not chat_with_tools()
    }

    #[test]
    fn test_provider_message_to_turn_user() {
        let msg = ProviderMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        let turn = provider_message_to_turn(&msg);
        assert_eq!(*turn.role(), TurnRole::User);
        assert_eq!(turn.content(), "hello");
    }

    #[test]
    fn test_provider_message_to_turn_tool_result() {
        let msg = ProviderMessage {
            role: "tool".to_string(),
            content: "42".to_string(),
            tool_calls: None,
            tool_call_id: Some("call-1".to_string()),
        };
        let turn = provider_message_to_turn(&msg);
        assert_eq!(*turn.role(), TurnRole::Tool);
        assert_eq!(turn.tool_call_id(), Some("call-1"));
    }
}
