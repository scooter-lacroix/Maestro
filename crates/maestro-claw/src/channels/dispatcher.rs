//! Channel dispatcher that routes inbound messages through the shared agent loop.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;

use crate::channels::{Channel, ChannelMessage, SendMessage};
use crate::config::Config;
use crate::cost::{CostEstimator, CostRecord, CostTracker, InvocationType};
use crate::observability::{create_observer, Observer, ObserverEvent, TelemetryCorrelation};

pub struct ChannelDispatcher {
    config: Config,
    observer: Box<dyn Observer>,
    cost_tracker: CostTracker,
}

impl ChannelDispatcher {
    pub fn new(config: &Config, workspace_dir: &Path) -> Self {
        let mut runtime_config = config.clone();
        runtime_config.workspace_dir = workspace_dir.to_path_buf();
        let observer = create_observer(&config.observability.backend, Some(workspace_dir));
        let cost_tracker = CostTracker::new(workspace_dir, f64::MAX, f64::MAX);

        Self {
            config: runtime_config,
            observer,
            cost_tracker,
        }
    }

    fn record_cost(
        &self,
        correlation: &TelemetryCorrelation,
        prompt_chars: usize,
        response_chars: usize,
        duration_ms: i64,
        success: bool,
        error_message: Option<String>,
    ) {
        let estimated_cost = CostEstimator::estimate_cost(
            &self.config.primary_tool,
            prompt_chars,
            response_chars,
            None,
        );

        let _ = self.cost_tracker.record(CostRecord {
            timestamp: Utc::now(),
            tool: self.config.primary_tool.clone(),
            provider: "cli".into(),
            model: None,
            duration_ms,
            prompt_chars,
            response_chars,
            estimated_cost_usd: estimated_cost,
            success,
            error_message,
            invocation_type: InvocationType::ChannelMessage,
            workspace_dir: Some(self.config.workspace_dir.to_string_lossy().to_string()),
            session_id: correlation.session_id.clone(),
            component: correlation.component.clone(),
            correlation: Some(correlation.clone()),
        });
    }

    pub async fn run(
        &self,
        channel: Arc<dyn Channel>,
        mut rx: mpsc::Receiver<ChannelMessage>,
    ) -> anyhow::Result<()> {
        let component = format!("channel:{}", channel.name());
        crate::health::mark_component_ok(&component);

        while let Some(message) = rx.recv().await {
            let started_at = Instant::now();
            let prompt_chars = message.content.chars().count();
            let correlation = TelemetryCorrelation::default()
                .with_sender(message.sender.clone())
                .with_principal(message.sender.clone())
                .with_component(&component)
                .with_surface(channel.name().to_string())
                .normalized_with(
                    Some(message.id.clone()),
                    Some(component.clone()),
                    Some(channel.name()),
                );

            self.observer.record_correlated_event(
                &ObserverEvent::ChannelMessage {
                    channel: channel.name().to_string(),
                    sender: message.sender.clone(),
                    message_id: message.id.clone(),
                },
                correlation.clone(),
            );

            tracing::info!(
                channel = channel.name(),
                sender = %message.sender,
                reply_target = %message.reply_target,
                "received channel message"
            );

            match crate::agent::run_prompt(&self.config, message.content.clone(), 300).await {
                Ok(result) => {
                    let reply_content = result.content().to_string();
                    let reply = SendMessage::new(reply_content.clone(), &message.reply_target);
                    let response_chars = reply_content.chars().count();

                    match channel.send(&reply).await {
                        Ok(()) => {
                            crate::health::mark_component_ok(&component);
                            self.observer.record_correlated_event(
                                &ObserverEvent::ChannelResponse {
                                    channel: channel.name().to_string(),
                                    message_id: message.id.clone(),
                                    success: true,
                                },
                                correlation.clone(),
                            );
                            self.record_cost(
                                &correlation,
                                prompt_chars,
                                response_chars,
                                started_at.elapsed().as_millis() as i64,
                                true,
                                None,
                            );
                        }
                        Err(error) => {
                            let error_message = error.to_string();
                            crate::health::mark_component_error(&component, &error_message);
                            self.observer.record_correlated_event(
                                &ObserverEvent::ChannelResponse {
                                    channel: channel.name().to_string(),
                                    message_id: message.id.clone(),
                                    success: false,
                                },
                                correlation.clone(),
                            );
                            self.observer.record_correlated_event(
                                &ObserverEvent::ChannelError {
                                    channel: channel.name().to_string(),
                                    error: error_message.clone(),
                                },
                                correlation.clone(),
                            );
                            self.record_cost(
                                &correlation,
                                prompt_chars,
                                response_chars,
                                started_at.elapsed().as_millis() as i64,
                                false,
                                Some(error_message.clone()),
                            );
                            tracing::warn!(
                                channel = channel.name(),
                                "failed to send reply: {error}"
                            );
                        }
                    }
                }
                Err(error) => {
                    let error_message = error.to_string();
                    crate::health::mark_component_error(&component, &error_message);
                    self.observer.record_correlated_event(
                        &ObserverEvent::ChannelError {
                            channel: channel.name().to_string(),
                            error: error_message.clone(),
                        },
                        correlation.clone(),
                    );
                    self.record_cost(
                        &correlation,
                        prompt_chars,
                        0,
                        started_at.elapsed().as_millis() as i64,
                        false,
                        Some(error_message.clone()),
                    );

                    tracing::error!(channel = channel.name(), "agent error: {error}");

                    let fallback =
                        SendMessage::new(format!("Error: {error_message}"), &message.reply_target);
                    match channel.send(&fallback).await {
                        Ok(()) => {
                            self.observer.record_correlated_event(
                                &ObserverEvent::ChannelResponse {
                                    channel: channel.name().to_string(),
                                    message_id: message.id.clone(),
                                    success: true,
                                },
                                correlation.clone(),
                            );
                        }
                        Err(send_error) => {
                            self.observer.record_correlated_event(
                                &ObserverEvent::ChannelResponse {
                                    channel: channel.name().to_string(),
                                    message_id: message.id.clone(),
                                    success: false,
                                },
                                correlation.clone(),
                            );
                            self.observer.record_correlated_event(
                                &ObserverEvent::ChannelError {
                                    channel: channel.name().to_string(),
                                    error: send_error.to_string(),
                                },
                                correlation.clone(),
                            );
                            tracing::warn!(
                                channel = channel.name(),
                                "failed to send error response: {send_error}"
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
