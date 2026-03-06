use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::Value;

use crate::engine::router::{parse_intent, Intent, RouteErrorKind};
use crate::engine::ThreadSession;
use crate::traits::{Context, Message, Provider, Tool};

#[derive(Debug, Clone)]
pub struct LoopConfig {
    pub max_iterations: usize,
    pub enable_text_fallback: bool,
    approval_policy: ApprovalPolicy,
}

#[derive(Debug, Clone)]
enum ApprovalPolicy {
    Never,
    ToolNames(HashSet<String>),
}

impl LoopConfig {
    pub fn new(max_iterations: usize, enable_text_fallback: bool) -> Self {
        Self {
            max_iterations,
            enable_text_fallback,
            approval_policy: ApprovalPolicy::Never,
        }
    }

    pub fn require_approval_for_tool(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        match &mut self.approval_policy {
            ApprovalPolicy::Never => {
                let mut names = HashSet::new();
                names.insert(tool_name);
                self.approval_policy = ApprovalPolicy::ToolNames(names);
            }
            ApprovalPolicy::ToolNames(names) => {
                names.insert(tool_name);
            }
        }
    }

    fn requires_approval(&self, tool_name: &str) -> bool {
        match &self.approval_policy {
            ApprovalPolicy::Never => false,
            ApprovalPolicy::ToolNames(names) => names.contains(tool_name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopErrorKind {
    UnknownTool,
    MalformedToolCall,
    MaxIterationsExceeded,
    ProviderError,
    ToolExecutionError,
    SessionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopError {
    pub kind: LoopErrorKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LoopOutcome {
    Response {
        message: Message,
    },
    NeedApproval {
        request_id: String,
        tool_name: String,
        arguments: Value,
    },
    Error(LoopError),
}

pub async fn run_tool_loop(
    provider: &dyn Provider,
    context: &mut Context,
    session: &mut ThreadSession,
    tools: &HashMap<String, Arc<dyn Tool>>,
    config: &LoopConfig,
) -> LoopOutcome {
    for _ in 0..config.max_iterations {
        let response = match provider.generate(context).await {
            Ok(message) => message,
            Err(err) => {
                return fail_with(
                    session,
                    LoopErrorKind::ProviderError,
                    format!("provider generate failed: {err}"),
                )
            }
        };

        context.messages.push(response.clone());

        let intent = match parse_intent(&response.content, config.enable_text_fallback) {
            Ok(intent) => intent,
            Err(err) => {
                let kind = match err.kind {
                    RouteErrorKind::MalformedToolCall => LoopErrorKind::MalformedToolCall,
                };
                return fail_with(session, kind, err.message);
            }
        };

        match intent {
            Intent::Response => {
                if let Err(err) = session.mark_completed() {
                    return fail_with(
                        session,
                        LoopErrorKind::SessionState,
                        format!("failed to mark completed: {err}"),
                    );
                }
                return LoopOutcome::Response { message: response };
            }
            Intent::ToolCall(tool_call) => {
                if config.requires_approval(&tool_call.name) {
                    if let Err(err) =
                        session.transition_to_awaiting_approval(tool_call.request_id.clone())
                    {
                        return fail_with(
                            session,
                            LoopErrorKind::SessionState,
                            format!("failed to transition to awaiting approval: {err}"),
                        );
                    }

                    return LoopOutcome::NeedApproval {
                        request_id: tool_call.request_id,
                        tool_name: tool_call.name,
                        arguments: tool_call.arguments,
                    };
                }

                let tool = match tools.get(&tool_call.name) {
                    Some(tool) => tool,
                    None => {
                        return fail_with(
                            session,
                            LoopErrorKind::UnknownTool,
                            format!("unknown tool '{}'", tool_call.name),
                        )
                    }
                };

                let result = match tool.execute(tool_call.arguments.clone()).await {
                    Ok(output) => output,
                    Err(err) => {
                        return fail_with(
                            session,
                            LoopErrorKind::ToolExecutionError,
                            format!("tool '{}' execution failed: {err}", tool_call.name),
                        )
                    }
                };

                context.messages.push(Message {
                    role: "tool".to_string(),
                    content: result.to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
        }
    }

    fail_with(
        session,
        LoopErrorKind::MaxIterationsExceeded,
        format!("max iterations exceeded ({})", config.max_iterations),
    )
}

fn fail_with(session: &mut ThreadSession, kind: LoopErrorKind, message: String) -> LoopOutcome {
    let _ = session.mark_failed();
    LoopOutcome::Error(LoopError { kind, message })
}
