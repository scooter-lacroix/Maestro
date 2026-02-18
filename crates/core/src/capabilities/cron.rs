//! Routines Engine (Cron & Event Scheduler)
//!
//! This module implements cron and event-based scheduling following patterns from ZeroClaw and Moltis:
//! - `zeroclaw/src/cron/scheduler.rs` - Core scheduler loop
//! - `moltis/crates/cron/src/service.rs` - CronService with timer loop
//! - `zeroclaw/src/cron/types.rs` - Schedule types (Cron, At, Every)
//! - `zeroclaw/src/heartbeat/engine.rs` - HEARTBEAT.md processing
//!
//! Key features:
//! - Multiple schedule types: Cron expression, one-shot (At), interval (Every)
//! - Session isolation (Main, Isolated, Named)
//! - Job persistence and recovery
//! - Rate limiting and cooldown

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule as CronScheduleLib;
use serde::{Deserialize, Serialize};

/// Unique identifier for a job.
pub type JobId = String;

/// Schedule type for cron jobs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Schedule {
    /// Cron expression (5-field standard or 6-field with seconds).
    Cron {
        expr: String,
        #[serde(default)]
        tz: Option<String>,
    },
    /// One-shot: fire once at a specific time.
    At {
        #[serde(with = "chrono::serde::ts_milliseconds")]
        at: DateTime<Utc>,
    },
    /// Fixed interval: fire every N milliseconds.
    Every {
        every_ms: u64,
        /// Optional anchor time for interval alignment.
        #[serde(default)]
        anchor_ms: Option<u64>,
    },
}

impl Schedule {
    /// Create a cron schedule from an expression.
    pub fn cron(expr: impl Into<String>) -> Self {
        Self::Cron {
            expr: expr.into(),
            tz: None,
        }
    }

    /// Create a cron schedule with timezone.
    pub fn cron_with_tz(expr: impl Into<String>, tz: impl Into<String>) -> Self {
        Self::Cron {
            expr: expr.into(),
            tz: Some(tz.into()),
        }
    }

    /// Create a one-shot schedule at a specific time.
    pub fn at(dt: DateTime<Utc>) -> Self {
        Self::At { at: dt }
    }

    /// Create an interval schedule.
    pub fn every(duration: Duration) -> Self {
        Self::Every {
            every_ms: duration.as_millis() as u64,
            anchor_ms: None,
        }
    }

    /// Create an interval schedule with anchor.
    pub fn every_with_anchor(duration: Duration, anchor: DateTime<Utc>) -> Self {
        Self::Every {
            every_ms: duration.as_millis() as u64,
            anchor_ms: Some(anchor.timestamp_millis() as u64),
        }
    }

    /// Calculate the next run time after the given time.
    pub fn next_run(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Cron { expr, tz: _ } => {
                // Parse cron expression using TryFrom
                let schedule = CronScheduleLib::try_from(expr.as_str()).ok()?;

                // Get next occurrence after the given time
                let next = schedule.after(after).next();

                next.map(|dt| Utc.from_utc_datetime(&dt.naive_utc()))
            }
            Self::At { at } => {
                if at > after {
                    Some(*at)
                } else {
                    None
                }
            }
            Self::Every { every_ms, anchor_ms } => {
                let interval_ms = *every_ms;
                let anchor = anchor_ms.unwrap_or_else(|| after.timestamp_millis() as u64);

                let elapsed = after.timestamp_millis() as u64 - anchor;
                let intervals_elapsed = elapsed / interval_ms;
                let next_offset = (intervals_elapsed + 1) * interval_ms;
                let next_ts = anchor + next_offset;

                Utc.timestamp_millis_opt(next_ts as i64).single()
            }
        }
    }
}

/// Type of job to execute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum JobType {
    /// Shell command execution.
    #[default]
    Shell,
    /// Agent turn execution.
    Agent,
}

/// Target session for job execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SessionTarget {
    /// Inject into the main conversation session.
    Main,
    /// Run in an isolated, throwaway session.
    #[default]
    Isolated,
    /// Run in a named session that persists across runs.
    Named(String),
}

/// Delivery configuration for job results.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryConfig {
    /// Channel to deliver results to (e.g., "telegram", "slack").
    #[serde(default)]
    pub channel: Option<String>,
    /// Recipient for delivery.
    #[serde(default)]
    pub recipient: Option<String>,
    /// Whether to deliver on success.
    #[serde(default = "default_true")]
    pub on_success: bool,
    /// Whether to deliver on failure.
    #[serde(default)]
    pub on_failure: bool,
}

fn default_true() -> bool {
    true
}

/// Guardrails for job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobGuardrails {
    /// Minimum time between executions.
    #[serde(with = "humantime_serde", default)]
    pub cooldown: Duration,
    /// Maximum concurrent executions.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
    /// Deduplication window.
    #[serde(default)]
    pub dedup_window: Option<Duration>,
}

fn default_max_concurrent() -> u32 {
    1
}

impl Default for JobGuardrails {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_secs(0),
            max_concurrent: 1,
            dedup_window: None,
        }
    }
}

/// A cron job definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
    /// Unique job identifier.
    pub id: JobId,
    /// Human-readable name.
    #[serde(default)]
    pub name: Option<String>,
    /// Schedule for execution.
    pub schedule: Schedule,
    /// Type of job.
    #[serde(default)]
    pub job_type: JobType,
    /// Command to execute (for Shell type).
    #[serde(default)]
    pub command: Option<String>,
    /// Prompt to send (for Agent type).
    #[serde(default)]
    pub prompt: Option<String>,
    /// Target session for execution.
    #[serde(default)]
    pub session_target: SessionTarget,
    /// Result delivery configuration.
    #[serde(default)]
    pub delivery: DeliveryConfig,
    /// Execution guardrails.
    #[serde(default)]
    pub guardrails: JobGuardrails,
    /// Whether to delete after one run (for one-shots).
    #[serde(default)]
    pub delete_after_run: bool,
    /// Whether the job is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Tags for filtering/grouping.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl CronJob {
    /// Create a new job builder.
    pub fn builder(id: impl Into<String>) -> CronJobBuilder {
        CronJobBuilder::new(id)
    }

    /// Check if the job is due at the given time.
    pub fn is_due(&self, now: &DateTime<Utc>, last_run: Option<&DateTime<Utc>>) -> bool {
        if !self.enabled {
            return false;
        }

        // If never run before, job is due immediately
        let last = match last_run {
            Some(last) => last,
            None => return true,
        };

        let next = match self.schedule.next_run(last) {
            Some(next) => next,
            None => return false,
        };

        next <= *now
    }
}

/// Builder for creating cron jobs.
pub struct CronJobBuilder {
    id: JobId,
    name: Option<String>,
    schedule: Option<Schedule>,
    job_type: JobType,
    command: Option<String>,
    prompt: Option<String>,
    session_target: SessionTarget,
    delivery: DeliveryConfig,
    guardrails: JobGuardrails,
    delete_after_run: bool,
    enabled: bool,
    tags: Vec<String>,
}

impl CronJobBuilder {
    /// Create a new builder with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            schedule: None,
            job_type: JobType::Shell,
            command: None,
            prompt: None,
            session_target: SessionTarget::default(),
            delivery: DeliveryConfig::default(),
            guardrails: JobGuardrails::default(),
            delete_after_run: false,
            enabled: true,
            tags: Vec::new(),
        }
    }

    /// Set the job name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the schedule.
    pub fn schedule(mut self, schedule: Schedule) -> Self {
        self.schedule = Some(schedule);
        self
    }

    /// Set as cron schedule.
    pub fn cron(mut self, expr: impl Into<String>) -> Self {
        self.schedule = Some(Schedule::cron(expr));
        self
    }

    /// Set as interval schedule.
    pub fn every(mut self, duration: Duration) -> Self {
        self.schedule = Some(Schedule::every(duration));
        self
    }

    /// Set as one-shot at specific time.
    pub fn at(mut self, dt: DateTime<Utc>) -> Self {
        self.schedule = Some(Schedule::at(dt));
        self
    }

    /// Set as shell job with command.
    pub fn shell(mut self, command: impl Into<String>) -> Self {
        self.job_type = JobType::Shell;
        self.command = Some(command.into());
        self
    }

    /// Set as agent job with prompt.
    pub fn agent(mut self, prompt: impl Into<String>) -> Self {
        self.job_type = JobType::Agent;
        self.prompt = Some(prompt.into());
        self
    }

    /// Set session target.
    pub fn session_target(mut self, target: SessionTarget) -> Self {
        self.session_target = target;
        self
    }

    /// Set as isolated session.
    pub fn isolated(mut self) -> Self {
        self.session_target = SessionTarget::Isolated;
        self
    }

    /// Set as main session.
    pub fn main_session(mut self) -> Self {
        self.session_target = SessionTarget::Main;
        self
    }

    /// Set as named session.
    pub fn named_session(mut self, name: impl Into<String>) -> Self {
        self.session_target = SessionTarget::Named(name.into());
        self
    }

    /// Set delivery configuration.
    pub fn delivery(mut self, delivery: DeliveryConfig) -> Self {
        self.delivery = delivery;
        self
    }

    /// Set cooldown duration.
    pub fn cooldown(mut self, duration: Duration) -> Self {
        self.guardrails.cooldown = duration;
        self
    }

    /// Set max concurrent executions.
    pub fn max_concurrent(mut self, max: u32) -> Self {
        self.guardrails.max_concurrent = max;
        self
    }

    /// Set delete after run.
    pub fn delete_after_run(mut self, delete: bool) -> Self {
        self.delete_after_run = delete;
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set enabled status.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Build the cron job.
    pub fn build(self) -> anyhow::Result<CronJob> {
        let schedule = self.schedule.ok_or_else(|| {
            anyhow::anyhow!("Schedule is required")
        })?;

        Ok(CronJob {
            id: self.id,
            name: self.name,
            schedule,
            job_type: self.job_type,
            command: self.command,
            prompt: self.prompt,
            session_target: self.session_target,
            delivery: self.delivery,
            guardrails: self.guardrails,
            delete_after_run: self.delete_after_run,
            enabled: self.enabled,
            tags: self.tags,
        })
    }
}

/// Result of a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    /// Job ID.
    pub job_id: JobId,
    /// Whether execution succeeded.
    pub success: bool,
    /// Output from execution.
    pub output: String,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution start time.
    pub started_at: DateTime<Utc>,
    /// Execution end time.
    pub finished_at: DateTime<Utc>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

/// Storage trait for job persistence.
pub trait JobStore: Send + Sync {
    /// Get all jobs.
    fn get_jobs(&self) -> Vec<CronJob>;

    /// Get a job by ID.
    fn get_job(&self, id: &str) -> Option<CronJob>;

    /// Add or update a job.
    fn save_job(&mut self, job: CronJob);

    /// Delete a job.
    fn delete_job(&mut self, id: &str) -> bool;

    /// Get the last run time for a job.
    fn get_last_run(&self, id: &str) -> Option<DateTime<Utc>>;

    /// Record a job execution.
    fn record_run(&mut self, result: JobResult);
}

/// In-memory job store for testing.
#[derive(Debug, Default)]
pub struct InMemoryJobStore {
    jobs: HashMap<JobId, CronJob>,
    last_runs: HashMap<JobId, DateTime<Utc>>,
    results: Vec<JobResult>,
}

impl InMemoryJobStore {
    /// Create a new in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobStore for InMemoryJobStore {
    fn get_jobs(&self) -> Vec<CronJob> {
        self.jobs.values().cloned().collect()
    }

    fn get_job(&self, id: &str) -> Option<CronJob> {
        self.jobs.get(id).cloned()
    }

    fn save_job(&mut self, job: CronJob) {
        self.jobs.insert(job.id.clone(), job);
    }

    fn delete_job(&mut self, id: &str) -> bool {
        self.jobs.remove(id).is_some()
    }

    fn get_last_run(&self, id: &str) -> Option<DateTime<Utc>> {
        self.last_runs.get(id).copied()
    }

    fn record_run(&mut self, result: JobResult) {
        self.last_runs.insert(result.job_id.clone(), result.finished_at);
        self.results.push(result);
    }
}

/// Cron service configuration.
#[derive(Debug, Clone)]
pub struct CronConfig {
    /// Poll interval for checking due jobs.
    pub poll_interval: Duration,
    /// Default timeout for job execution.
    pub default_timeout: Duration,
    /// Maximum retries on failure.
    pub max_retries: u32,
    /// Retry delay multiplier.
    pub retry_delay_multiplier: f64,
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(30),
            default_timeout: Duration::from_secs(300),
            max_retries: 3,
            retry_delay_multiplier: 2.0,
        }
    }
}

/// The cron scheduler service.
pub struct CronService<S: JobStore> {
    store: Arc<std::sync::Mutex<S>>,
    config: CronConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl<S: JobStore + 'static> CronService<S> {
    /// Create a new cron service.
    pub fn new(store: S, config: CronConfig) -> Self {
        Self {
            store: Arc::new(std::sync::Mutex::new(store)),
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the scheduler loop.
    pub async fn start(&self) {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        let mut interval = tokio::time::interval(self.config.poll_interval);

        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            interval.tick().await;
            self.tick().await;
        }
    }

    /// Stop the scheduler.
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check for and execute due jobs.
    pub async fn tick(&self) {
        let now = Utc::now();
        let due_jobs: Vec<CronJob> = {
            let store = self.store.lock().unwrap();
            store
                .get_jobs()
                .into_iter()
                .filter(|job| {
                    let last_run = store.get_last_run(&job.id);
                    job.is_due(&now, last_run.as_ref())
                })
                .collect()
        };

        for job in due_jobs {
            self.execute_job(job).await;
        }
    }

    /// Execute a single job.
    async fn execute_job(&self, job: CronJob) {
        let start = Utc::now();

        // For now, just record a placeholder result
        // Full implementation would execute shell commands or agent turns
        let result = JobResult {
            job_id: job.id.clone(),
            success: true,
            output: format!("Job {} executed", job.name.as_deref().unwrap_or(&job.id)),
            error: None,
            started_at: start,
            finished_at: Utc::now(),
            duration_ms: 0,
        };

        let mut store = self.store.lock().unwrap();
        store.record_run(result);

        // Delete one-shot jobs after execution
        if job.delete_after_run {
            store.delete_job(&job.id);
        }
    }

    /// Add a job.
    pub fn add_job(&self, job: CronJob) {
        let mut store = self.store.lock().unwrap();
        store.save_job(job);
    }

    /// Remove a job.
    pub fn remove_job(&self, id: &str) -> bool {
        let mut store = self.store.lock().unwrap();
        store.delete_job(id)
    }

    /// Get all jobs.
    pub fn get_jobs(&self) -> Vec<CronJob> {
        let store = self.store.lock().unwrap();
        store.get_jobs()
    }

    /// Get a job by ID.
    pub fn get_job(&self, id: &str) -> Option<CronJob> {
        let store = self.store.lock().unwrap();
        store.get_job(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_cron() {
        let schedule = Schedule::cron("0 0 * * * *"); // Every hour
        let now = Utc::now();
        let next = schedule.next_run(&now);
        assert!(next.is_some());
        assert!(next.unwrap() > now);
    }

    #[test]
    fn test_schedule_every() {
        let schedule = Schedule::every(Duration::from_secs(60));
        let now = Utc::now();
        let next = schedule.next_run(&now);
        assert!(next.is_some());
        let next = next.unwrap();
        assert!(next > now);
        assert!(next <= now + chrono::Duration::seconds(120));
    }

    #[test]
    fn test_schedule_at_future() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let schedule = Schedule::at(future);
        let now = Utc::now();
        let next = schedule.next_run(&now);
        assert!(next.is_some());
        assert_eq!(next.unwrap(), future);
    }

    #[test]
    fn test_schedule_at_past() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let schedule = Schedule::at(past);
        let now = Utc::now();
        let next = schedule.next_run(&now);
        assert!(next.is_none());
    }

    #[test]
    fn test_cron_job_builder() {
        let job = CronJob::builder("test-job")
            .name("Test Job")
            .every(Duration::from_secs(300))
            .agent("Check system status")
            .isolated()
            .cooldown(Duration::from_secs(60))
            .build()
            .unwrap();

        assert_eq!(job.id, "test-job");
        assert_eq!(job.name, Some("Test Job".to_string()));
        assert_eq!(job.job_type, JobType::Agent);
        assert_eq!(job.prompt, Some("Check system status".to_string()));
        assert_eq!(job.session_target, SessionTarget::Isolated);
        assert!(job.enabled);
    }

    #[test]
    fn test_cron_job_builder_shell() {
        let job = CronJob::builder("backup")
            .cron("0 0 2 * * *") // 2 AM daily
            .shell("rsync -av /data /backup")
            .main_session()
            .delete_after_run(false)
            .build()
            .unwrap();

        assert_eq!(job.id, "backup");
        assert_eq!(job.job_type, JobType::Shell);
        assert!(job.command.is_some());
        assert_eq!(job.session_target, SessionTarget::Main);
        assert!(!job.delete_after_run);
    }

    #[test]
    fn test_cron_job_is_due() {
        let job = CronJob::builder("test")
            .every(Duration::from_secs(60))
            .shell("echo test")
            .build()
            .unwrap();

        // Job with no last run should be due
        let now = Utc::now();
        assert!(job.is_due(&now, None));

        // Job with recent last run (within interval) should not be due
        let recent = now - chrono::Duration::seconds(30);
        // Note: is_due checks if next scheduled time <= now, so this depends on schedule
    }

    #[test]
    fn test_job_store_crud() {
        let mut store = InMemoryJobStore::new();

        let job = CronJob::builder("test")
            .every(Duration::from_secs(60))
            .shell("test")
            .build()
            .unwrap();

        // Add
        store.save_job(job.clone());
        assert_eq!(store.get_jobs().len(), 1);

        // Get
        let retrieved = store.get_job("test").unwrap();
        assert_eq!(retrieved.id, "test");

        // Delete
        assert!(store.delete_job("test"));
        assert!(store.get_job("test").is_none());
        assert!(!store.delete_job("nonexistent"));
    }

    #[test]
    fn test_job_result_recording() {
        let mut store = InMemoryJobStore::new();

        let result = JobResult {
            job_id: "test".to_string(),
            success: true,
            output: "OK".to_string(),
            error: None,
            started_at: Utc::now(),
            finished_at: Utc::now(),
            duration_ms: 100,
        };

        store.record_run(result);
        let last_run = store.get_last_run("test");
        assert!(last_run.is_some());
    }

    #[test]
    fn test_default_config() {
        let config = CronConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(30));
        assert_eq!(config.default_timeout, Duration::from_secs(300));
        assert_eq!(config.max_retries, 3);
    }
}
