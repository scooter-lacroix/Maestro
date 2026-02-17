use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use maestro_core::engine::{
    run_tool_loop, LoopConfig, LoopErrorKind, LoopOutcome, ThreadSession, ThreadState,
};
use maestro_core::traits::{Context, Message, Provider, Tool};
use serde_json::{json, Value};

struct SequenceProvider {
    messages: Vec<Message>,
    index: AtomicUsize,
}

impl SequenceProvider {
    fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            index: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Provider for SequenceProvider {
    fn name(&self) -> &str {
        "sequence"
    }

    async fn generate(&self, _context: &Context) -> Result<Message> {
        if self.messages.is_empty() {
            return Err(anyhow!("sequence provider requires at least one message"));
        }

        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        let selected = self
            .messages
            .get(idx)
            .or_else(|| self.messages.last())
            .cloned()
            .ok_or_else(|| anyhow!("failed to select provider message"))?;
        Ok(selected)
    }

    async fn stream(&self, _context: &Context) -> Result<BoxStream<'static, Result<String>>> {
        Ok(stream::empty().boxed())
    }
}

struct EchoTool {
    call_count: Arc<AtomicUsize>,
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "echo test tool"
    }

    fn input_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"echoed": input}))
    }
}

fn message(content: &str) -> Message {
    Message {
        role: "assistant".to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn direct_response_completes_session() {
    let provider = SequenceProvider::new(vec![message("done")]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(3, false);
    let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Response { message } => assert_eq!(message.content, "done"),
        other => panic!("expected response outcome, got {other:?}"),
    }
    assert_eq!(session.state(), &ThreadState::Completed);
}

#[tokio::test]
async fn approval_required_tool_call_sets_awaiting_approval() {
    let provider = SequenceProvider::new(vec![message(
        r#"{ "tool_calls": [{ "request_id": "req-1", "name": "echo", "arguments": {"value": 1}}]}"#,
    )]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let mut config = LoopConfig::new(3, false);
    config.require_approval_for_tool("echo");

    let call_count = Arc::new(AtomicUsize::new(0));
    let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    tools.insert(
        "echo".to_string(),
        Arc::new(EchoTool {
            call_count: call_count.clone(),
        }),
    );

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::NeedApproval {
            request_id,
            tool_name,
            arguments,
        } => {
            assert_eq!(request_id, "req-1");
            assert_eq!(tool_name, "echo");
            assert_eq!(arguments, json!({"value": 1}));
        }
        other => panic!("expected need approval outcome, got {other:?}"),
    }
    assert_eq!(session.state(), &ThreadState::AwaitingApproval);
    assert_eq!(session.pending_approval_request_id(), Some("req-1"));
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unknown_tool_call_fails_session() {
    let provider = SequenceProvider::new(vec![message(
        r#"{ "tool_calls": [{ "request_id": "req-unknown", "name": "missing", "arguments": {}}]}"#,
    )]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(3, false);
    let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Error(err) => assert_eq!(err.kind, LoopErrorKind::UnknownTool),
        other => panic!("expected error outcome, got {other:?}"),
    }
    assert_eq!(session.state(), &ThreadState::Failed);
}

#[tokio::test]
async fn malformed_tool_call_fails_session() {
    let provider = SequenceProvider::new(vec![message(
        r#"{ "tool_calls": [{ "name": "echo", "arguments": {} }]}"#,
    )]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(3, false);
    let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Error(err) => assert_eq!(err.kind, LoopErrorKind::MalformedToolCall),
        other => panic!("expected malformed-tool error outcome, got {other:?}"),
    }
    assert_eq!(session.state(), &ThreadState::Failed);
}

#[tokio::test]
async fn plain_text_mentioning_tool_calls_is_not_malformed() {
    let response_text = "The token \"tool_calls\" appears in prose.";
    let provider = SequenceProvider::new(vec![message(response_text)]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(3, false);
    let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Response { message } => assert_eq!(message.content, response_text),
        other => panic!("expected response outcome, got {other:?}"),
    }
    assert_eq!(session.state(), &ThreadState::Completed);
}

#[tokio::test]
async fn text_fallback_ignores_tool_prefixed_tags() {
    let payload = r#"<tooling name="echo">{"x": 7}</tooling>"#;
    let provider = SequenceProvider::new(vec![message(payload)]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(3, true);

    let call_count = Arc::new(AtomicUsize::new(0));
    let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    tools.insert(
        "echo".to_string(),
        Arc::new(EchoTool {
            call_count: call_count.clone(),
        }),
    );

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Response { message } => assert_eq!(message.content, payload),
        other => panic!("expected response outcome, got {other:?}"),
    }
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
    assert_eq!(session.state(), &ThreadState::Completed);
}

#[tokio::test]
async fn multiple_native_tool_calls_return_malformed_error() {
    let provider = SequenceProvider::new(vec![message(
        r#"{ "tool_calls": [
            { "request_id": "req-1", "name": "echo", "arguments": {"value": 1} },
            { "request_id": "req-2", "name": "echo", "arguments": {"value": 2} }
        ]}"#,
    )]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(3, false);

    let call_count = Arc::new(AtomicUsize::new(0));
    let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    tools.insert(
        "echo".to_string(),
        Arc::new(EchoTool {
            call_count: call_count.clone(),
        }),
    );

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Error(err) => {
            assert_eq!(err.kind, LoopErrorKind::MalformedToolCall);
            assert!(err.message.contains("multiple calls are not supported"));
        }
        other => panic!("expected malformed-tool error outcome, got {other:?}"),
    }
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
    assert_eq!(session.state(), &ThreadState::Failed);
}

#[tokio::test]
async fn max_iterations_exceeded_is_terminal_error() {
    let provider = SequenceProvider::new(vec![message(
        r#"{ "tool_calls": [{ "request_id": "req-loop", "name": "echo", "arguments": {"x": 1}}]}"#,
    )]);
    let mut context = Context::default();
    let mut session = ThreadSession::new();
    let config = LoopConfig::new(2, false);

    let call_count = Arc::new(AtomicUsize::new(0));
    let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    tools.insert(
        "echo".to_string(),
        Arc::new(EchoTool {
            call_count: call_count.clone(),
        }),
    );

    let outcome = run_tool_loop(&provider, &mut context, &mut session, &tools, &config).await;

    match outcome {
        LoopOutcome::Error(err) => assert_eq!(err.kind, LoopErrorKind::MaxIterationsExceeded),
        other => panic!("expected max-iterations error outcome, got {other:?}"),
    }
    assert_eq!(session.state(), &ThreadState::Failed);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn text_fallback_respects_enable_flag() {
    let tag_payload = r#"<tool name="echo">{"x": 7}</tool>"#;

    let provider_enabled = SequenceProvider::new(vec![message(tag_payload), message("after tool")]);
    let mut context_enabled = Context::default();
    let mut session_enabled = ThreadSession::new();
    let config_enabled = LoopConfig::new(3, true);

    let enabled_calls = Arc::new(AtomicUsize::new(0));
    let mut enabled_tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    enabled_tools.insert(
        "echo".to_string(),
        Arc::new(EchoTool {
            call_count: enabled_calls.clone(),
        }),
    );

    let enabled_outcome = run_tool_loop(
        &provider_enabled,
        &mut context_enabled,
        &mut session_enabled,
        &enabled_tools,
        &config_enabled,
    )
    .await;

    match enabled_outcome {
        LoopOutcome::Response { message } => assert_eq!(message.content, "after tool"),
        other => panic!("expected response outcome in fallback-enabled path, got {other:?}"),
    }
    assert_eq!(enabled_calls.load(Ordering::SeqCst), 1);
    assert_eq!(session_enabled.state(), &ThreadState::Completed);

    let provider_disabled =
        SequenceProvider::new(vec![message(tag_payload), message("after tool")]);
    let mut context_disabled = Context::default();
    let mut session_disabled = ThreadSession::new();
    let config_disabled = LoopConfig::new(3, false);

    let disabled_calls = Arc::new(AtomicUsize::new(0));
    let mut disabled_tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    disabled_tools.insert(
        "echo".to_string(),
        Arc::new(EchoTool {
            call_count: disabled_calls.clone(),
        }),
    );

    let disabled_outcome = run_tool_loop(
        &provider_disabled,
        &mut context_disabled,
        &mut session_disabled,
        &disabled_tools,
        &config_disabled,
    )
    .await;

    match disabled_outcome {
        LoopOutcome::Response { message } => assert_eq!(message.content, tag_payload),
        other => panic!("expected direct response when fallback disabled, got {other:?}"),
    }
    assert_eq!(disabled_calls.load(Ordering::SeqCst), 0);
    assert_eq!(session_disabled.state(), &ThreadState::Completed);
}
