//! Heartbeat engine for periodic maintenance tasks.

use std::path::Path;
use std::time::Instant;

use anyhow::{anyhow, Result};
use chrono::Utc;
use tokio::time::{self, Duration};

use crate::config::Config;
use crate::cost::{CostEstimator, CostRecord, CostTracker, InvocationType};
use crate::observability::{create_observer, Observer, ObserverEvent, TelemetryCorrelation};

pub struct HeartbeatEngine {
    interval_minutes: u32,
    config: Config,
    observer: Box<dyn Observer>,
    cost_tracker: CostTracker,
}

impl HeartbeatEngine {
    pub fn new(config: &Config) -> Self {
        let observer = create_observer(&config.observability.backend, Some(&config.workspace_dir));
        let cost_tracker = CostTracker::new(&config.workspace_dir, f64::MAX, f64::MAX);

        Self {
            interval_minutes: config.heartbeat.interval_minutes,
            config: config.clone(),
            observer,
            cost_tracker,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let minutes = self.interval_minutes.max(5);
        tracing::info!("heartbeat started: every {minutes} minutes");
        let mut interval = time::interval(Duration::from_secs(u64::from(minutes) * 60));

        loop {
            interval.tick().await;
            crate::health::mark_component_ok("heartbeat");
            self.observer.record_correlated_event(
                &ObserverEvent::HeartbeatTick,
                TelemetryCorrelation::default()
                    .with_component("heartbeat")
                    .with_surface("heartbeat"),
            );

            match self.tick().await {
                Ok(count) if count > 0 => tracing::info!("heartbeat processed {count} task(s)"),
                Ok(_) => {}
                Err(error) => {
                    crate::health::mark_component_error("heartbeat", error.to_string());
                    tracing::warn!("heartbeat error: {error}");
                }
            }
        }
    }

    async fn tick(&self) -> Result<usize> {
        let tasks = self.collect_tasks().await?;
        let mut completed = 0usize;
        let mut failures = Vec::new();

        for task in tasks {
            let prompt = Self::task_prompt(&task);
            let started_at = Instant::now();
            let correlation = TelemetryCorrelation::default()
                .with_component("heartbeat")
                .with_surface("heartbeat")
                .normalized_with(
                    Some(format!("heartbeat:{task}")),
                    Some("heartbeat".into()),
                    Some("heartbeat"),
                );
            self.observer.record_correlated_event(
                &ObserverEvent::HeartbeatTaskStart {
                    task_name: task.clone(),
                },
                correlation.clone(),
            );

            match self.execute_task(&task).await {
                Ok(response) => {
                    completed += 1;
                    let duration_ms = started_at.elapsed().as_millis() as i64;
                    self.observer.record_correlated_event(
                        &ObserverEvent::HeartbeatTaskComplete {
                            task_name: task.clone(),
                            duration_ms,
                            success: true,
                        },
                        correlation.clone(),
                    );
                    self.record_cost(
                        prompt.chars().count(),
                        response.chars().count(),
                        duration_ms,
                        true,
                        None,
                        correlation.clone(),
                    );
                }
                Err(error) => {
                    let error_message = error.to_string();
                    let duration_ms = started_at.elapsed().as_millis() as i64;
                    self.observer.record_correlated_event(
                        &ObserverEvent::HeartbeatTaskError {
                            task_name: task.clone(),
                            error: error_message.clone(),
                        },
                        correlation.clone(),
                    );
                    self.record_cost(
                        prompt.chars().count(),
                        0,
                        duration_ms,
                        false,
                        Some(error_message.clone()),
                        correlation.clone(),
                    );
                    failures.push(format!("{task}: {error_message}"));
                }
            }
        }

        if failures.is_empty() {
            Ok(completed)
        } else {
            Err(anyhow!(
                "{} heartbeat task(s) failed: {}",
                failures.len(),
                failures.join("; ")
            ))
        }
    }

    pub async fn collect_tasks(&self) -> Result<Vec<String>> {
        let path = self.config.workspace_dir.join("HEARTBEAT.md");
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        Ok(Self::parse_tasks(&content))
    }

    pub async fn ensure_heartbeat_file(workspace_dir: &Path) -> Result<()> {
        let path = workspace_dir.join("HEARTBEAT.md");
        if path.exists() {
            return Ok(());
        }

        tokio::fs::write(
            &path,
            "# Heartbeat Tasks\n\nPeriodic tasks for MaestroClaw.\n\n\
             <!-- Add tasks as markdown list items.\n\
             - Check for dependency updates\n\
             -->\n",
        )
        .await?;

        Ok(())
    }

    fn parse_tasks(content: &str) -> Vec<String> {
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if !(trimmed.starts_with("- ") || trimmed.starts_with("* ")) {
                    return None;
                }

                let task = trimmed[2..].trim();
                if task.is_empty() || task.starts_with("<!--") || task.starts_with("//") {
                    return None;
                }

                Some(task.to_string())
            })
            .collect()
    }

    fn task_prompt(task: &str) -> String {
        format!("[Heartbeat Task] {task}")
    }

    async fn execute_task(&self, task: &str) -> Result<String> {
        crate::agent::run_prompt(&self.config, Self::task_prompt(task), 600)
            .await
            .map(|result| result.content().to_string())
            .map_err(|error| anyhow!("Heartbeat task failed: {error}"))
    }

    fn record_cost(
        &self,
        prompt_chars: usize,
        response_chars: usize,
        duration_ms: i64,
        success: bool,
        error_message: Option<String>,
        correlation: TelemetryCorrelation,
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
            invocation_type: InvocationType::HeartbeatTask,
            workspace_dir: Some(self.config.workspace_dir.to_string_lossy().to_string()),
            session_id: correlation.session_id.clone(),
            component: correlation
                .component
                .clone()
                .or_else(|| Some("heartbeat".into())),
            correlation: Some(correlation),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tasks_extracts_markdown_list_items() {
        let tasks = HeartbeatEngine::parse_tasks("# Heartbeat\n\n- Check updates\n* Run tests\n");
        assert_eq!(tasks, vec!["Check updates", "Run tests"]);
    }

    #[test]
    fn parse_tasks_skips_comment_entries() {
        let tasks = HeartbeatEngine::parse_tasks("- Real task\n- <!-- skip -->\n- // skip\n");
        assert_eq!(tasks, vec!["Real task"]);
    }

    #[test]
    fn task_prompt_uses_heartbeat_prefix() {
        assert_eq!(
            HeartbeatEngine::task_prompt("Check updates"),
            "[Heartbeat Task] Check updates"
        );
    }

    #[tokio::test]
    async fn ensure_heartbeat_file_creates_template() {
        let tmp = tempfile::TempDir::new().unwrap();
        HeartbeatEngine::ensure_heartbeat_file(tmp.path())
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(tmp.path().join("HEARTBEAT.md"))
            .await
            .unwrap();
        assert!(content.contains("# Heartbeat Tasks"));
    }
}
