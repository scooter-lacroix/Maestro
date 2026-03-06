//! Cron scheduler for MaestroClaw — polls due jobs and executes them

use std::time::Instant;

use serde_json::json;

use crate::config::Config;
use crate::cost::{CostEstimator, CostRecord, CostTracker, InvocationType};
use crate::cron::{due_jobs, record_run, reschedule_after_run, CronJob, JobType};
use crate::observability::{create_observer, ObserverEvent, TelemetryCorrelation};
use anyhow::Result;
use chrono::Utc;
use tokio::time::{self, Duration};

pub async fn run(config: &Config) -> Result<()> {
    let poll_secs = config.runtime.scheduler_poll_secs.max(5);
    let max_history = config.cron.max_run_history;
    let mut interval = time::interval(Duration::from_secs(poll_secs));
    let observer = create_observer(&config.observability.backend, Some(&config.workspace_dir));
    let cost_tracker = CostTracker::new(&config.workspace_dir, f64::MAX, f64::MAX);

    crate::health::mark_component_ok("scheduler");

    loop {
        interval.tick().await;
        observer.record_correlated_event(
            &ObserverEvent::SchedulerTick,
            TelemetryCorrelation::default()
                .with_component("scheduler")
                .with_surface("cron"),
        );

        let jobs = match due_jobs(&config.workspace_dir, Utc::now()) {
            Ok(jobs) => jobs,
            Err(e) => {
                crate::health::mark_component_error("scheduler", e.to_string());
                tracing::warn!("Scheduler query failed: {e}");
                continue;
            }
        };

        for job in jobs {
            crate::health::mark_component_ok("scheduler");
            let started = Utc::now();
            let started_instant = Instant::now();
            let correlation = TelemetryCorrelation::default()
                .with_component("scheduler")
                .with_surface("cron")
                .normalized_with(Some(job.id.clone()), Some("scheduler".into()), Some("cron"));
            observer.record_correlated_event(
                &ObserverEvent::SchedulerJobStart {
                    job_id: job.id.clone(),
                    job_type: job.job_type.as_str().to_string(),
                },
                correlation.clone(),
            );
            let (success, output) = execute_job(config, &job).await;
            let finished = Utc::now();
            let duration_ms = (finished - started).num_milliseconds();

            let _ = record_run(
                &config.workspace_dir,
                &job.id,
                started,
                finished,
                if success { "ok" } else { "error" },
                Some(&output),
                duration_ms,
                max_history,
            );

            if let Err(e) = reschedule_after_run(&config.workspace_dir, &job, success, &output) {
                tracing::warn!("Failed to reschedule job {}: {e}", job.id);
            }

            if success {
                observer.record_correlated_event(
                    &ObserverEvent::SchedulerJobComplete {
                        job_id: job.id.clone(),
                        job_type: job.job_type.as_str().to_string(),
                        duration_ms,
                        success: true,
                    },
                    correlation.clone(),
                );
            } else {
                crate::health::mark_component_error("scheduler", format!("job {} failed", job.id));
                observer.record_correlated_event(
                    &ObserverEvent::SchedulerJobError {
                        job_id: job.id.clone(),
                        job_type: job.job_type.as_str().to_string(),
                        error: output.clone(),
                    },
                    correlation.clone(),
                );
            }

            let prompt_chars = match job.job_type {
                JobType::Shell => job.command.chars().count(),
                JobType::Agent => job.prompt.as_deref().unwrap_or("").chars().count(),
            };
            let response_chars = output.chars().count();
            let _ = cost_tracker.record(CostRecord {
                timestamp: Utc::now(),
                tool: config.primary_tool.clone(),
                provider: "cli".into(),
                model: None,
                duration_ms: started_instant.elapsed().as_millis() as i64,
                prompt_chars,
                response_chars,
                estimated_cost_usd: CostEstimator::estimate_cost(
                    &config.primary_tool,
                    prompt_chars,
                    response_chars,
                    None,
                ),
                success,
                error_message: (!success).then(|| output.clone()),
                invocation_type: InvocationType::CronJob,
                workspace_dir: Some(config.workspace_dir.to_string_lossy().to_string()),
                session_id: correlation.session_id.clone(),
                component: correlation.component.clone(),
                correlation: Some(correlation),
            });
        }
    }
}

async fn execute_job(config: &Config, job: &CronJob) -> (bool, String) {
    match job.job_type {
        JobType::Shell => run_shell_job(config, job).await,
        JobType::Agent => run_agent_job(config, job).await,
    }
}

async fn run_shell_job(config: &Config, job: &CronJob) -> (bool, String) {
    let registry = crate::agent::build_default_tool_registry(config);
    let Some(shell) = registry.get("shell") else {
        return (false, "shell tool missing from runtime registry".into());
    };

    let output = shell.execute(json!({ "command": job.command })).await;
    if output.is_error {
        (false, output.content)
    } else {
        (true, output.content)
    }
}

async fn run_agent_job(config: &Config, job: &CronJob) -> (bool, String) {
    let prompt = job.prompt.as_deref().unwrap_or("");
    let prefixed = format!("[cron:{}] {prompt}", job.id);

    match crate::agent::run_prompt(config, prefixed, 600).await {
        Ok(result) => {
            let content = result.content().trim().to_string();
            (
                true,
                if content.is_empty() {
                    "agent job executed".into()
                } else {
                    content
                },
            )
        }
        Err(error) => (false, format!("agent job failed: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_type_shell_default() {
        let job = CronJob {
            id: "test".into(),
            expression: String::new(),
            schedule: crate::cron::Schedule::Every { every_ms: 1000 },
            command: "echo ok".into(),
            prompt: None,
            name: None,
            job_type: JobType::Shell,
            enabled: true,
            created_at: Utc::now(),
            next_run: Utc::now(),
            last_run: None,
            last_status: None,
            last_output: None,
        };
        assert_eq!(job.job_type.as_str(), "shell");
    }
}
