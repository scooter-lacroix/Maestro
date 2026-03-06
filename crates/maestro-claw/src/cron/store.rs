//! File-based cron job store for MaestroClaw
//!
//! Uses JSON files instead of SQLite for simplicity and no extra dependencies.

use crate::cron::{
    next_run_for_schedule, schedule_expression, validate_schedule, CronJob, CronJobPatch, CronRun,
    JobType, Schedule,
};
use anyhow::{Context, Result};
use chrono::Utc;

fn jobs_file(workspace: &std::path::Path) -> std::path::PathBuf {
    workspace.join("cron").join("jobs.json")
}

fn runs_file(workspace: &std::path::Path) -> std::path::PathBuf {
    workspace.join("cron").join("runs.json")
}

fn ensure_dir(workspace: &std::path::Path) -> Result<()> {
    let dir = workspace.join("cron");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create cron dir: {}", dir.display()))
}

fn load_jobs(workspace: &std::path::Path) -> Result<Vec<CronJob>> {
    let path = jobs_file(workspace);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let jobs: Vec<CronJob> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(jobs)
}

fn save_jobs(workspace: &std::path::Path, jobs: &[CronJob]) -> Result<()> {
    ensure_dir(workspace)?;
    let content = serde_json::to_string_pretty(jobs)?;
    std::fs::write(jobs_file(workspace), content)?;
    Ok(())
}

fn load_runs(workspace: &std::path::Path) -> Result<Vec<CronRun>> {
    let path = runs_file(workspace);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let runs: Vec<CronRun> = serde_json::from_str(&content)?;
    Ok(runs)
}

fn save_runs(workspace: &std::path::Path, runs: &[CronRun]) -> Result<()> {
    ensure_dir(workspace)?;
    let content = serde_json::to_string_pretty(runs)?;
    std::fs::write(runs_file(workspace), content)?;
    Ok(())
}

pub fn add_shell_job(
    workspace: &std::path::Path,
    name: Option<String>,
    schedule: Schedule,
    command: &str,
) -> Result<CronJob> {
    let now = Utc::now();
    validate_schedule(&schedule, now)?;
    let next_run = next_run_for_schedule(&schedule, now)?;
    let id = uuid::Uuid::new_v4().to_string();
    let expression = schedule_expression(&schedule).unwrap_or_default();

    let job = CronJob {
        id: id.clone(),
        expression,
        schedule,
        command: command.to_string(),
        prompt: None,
        name,
        job_type: JobType::Shell,
        enabled: true,
        created_at: now,
        next_run,
        last_run: None,
        last_status: None,
        last_output: None,
    };

    let mut jobs = load_jobs(workspace)?;
    jobs.push(job);
    save_jobs(workspace, &jobs)?;

    get_job(workspace, &id)
}

pub fn add_agent_job(
    workspace: &std::path::Path,
    name: Option<String>,
    schedule: Schedule,
    prompt: &str,
) -> Result<CronJob> {
    let now = Utc::now();
    validate_schedule(&schedule, now)?;
    let next_run = next_run_for_schedule(&schedule, now)?;
    let id = uuid::Uuid::new_v4().to_string();
    let expression = schedule_expression(&schedule).unwrap_or_default();

    let job = CronJob {
        id: id.clone(),
        expression,
        schedule,
        command: String::new(),
        prompt: Some(prompt.to_string()),
        name,
        job_type: JobType::Agent,
        enabled: true,
        created_at: now,
        next_run,
        last_run: None,
        last_status: None,
        last_output: None,
    };

    let mut jobs = load_jobs(workspace)?;
    jobs.push(job);
    save_jobs(workspace, &jobs)?;

    get_job(workspace, &id)
}

pub fn list_jobs(workspace: &std::path::Path) -> Result<Vec<CronJob>> {
    let mut jobs = load_jobs(workspace)?;
    jobs.sort_by(|a, b| a.next_run.cmp(&b.next_run));
    Ok(jobs)
}

pub fn get_job(workspace: &std::path::Path, job_id: &str) -> Result<CronJob> {
    let jobs = load_jobs(workspace)?;
    jobs.into_iter()
        .find(|j| j.id == job_id)
        .ok_or_else(|| anyhow::anyhow!("Cron job '{job_id}' not found"))
}

pub fn remove_job(workspace: &std::path::Path, id: &str) -> Result<()> {
    let mut jobs = load_jobs(workspace)?;
    let before = jobs.len();
    jobs.retain(|j| j.id != id);
    if jobs.len() == before {
        anyhow::bail!("Cron job '{id}' not found");
    }
    save_jobs(workspace, &jobs)?;

    // Also remove runs
    let mut runs = load_runs(workspace)?;
    runs.retain(|r| r.job_id != id);
    save_runs(workspace, &runs)?;

    println!("✅ Removed cron job {id}");
    Ok(())
}

pub fn due_jobs(
    workspace: &std::path::Path,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<CronJob>> {
    let jobs = load_jobs(workspace)?;
    Ok(jobs
        .into_iter()
        .filter(|j| j.enabled && j.next_run <= now)
        .collect())
}

pub fn update_job(
    workspace: &std::path::Path,
    job_id: &str,
    patch: CronJobPatch,
) -> Result<CronJob> {
    let mut jobs = load_jobs(workspace)?;
    let job = jobs
        .iter_mut()
        .find(|j| j.id == job_id)
        .ok_or_else(|| anyhow::anyhow!("Cron job '{job_id}' not found"))?;

    if let Some(schedule) = patch.schedule {
        validate_schedule(&schedule, Utc::now())?;
        job.expression = schedule_expression(&schedule).unwrap_or_default();
        job.next_run = next_run_for_schedule(&schedule, Utc::now())?;
        job.schedule = schedule;
    }
    if let Some(cmd) = patch.command {
        job.command = cmd;
    }
    if let Some(prompt) = patch.prompt {
        job.prompt = Some(prompt);
    }
    if let Some(name) = patch.name {
        job.name = Some(name);
    }
    if let Some(enabled) = patch.enabled {
        job.enabled = enabled;
    }

    save_jobs(workspace, &jobs)?;
    get_job(workspace, job_id)
}

pub fn reschedule_after_run(
    workspace: &std::path::Path,
    job: &CronJob,
    success: bool,
    output: &str,
) -> Result<()> {
    let mut jobs = load_jobs(workspace)?;
    if let Some(j) = jobs.iter_mut().find(|j| j.id == job.id) {
        j.last_run = Some(Utc::now());
        j.last_status = Some(if success { "ok" } else { "error" }.to_string());
        j.last_output = Some(output.to_string());
        if let Ok(next) = next_run_for_schedule(&j.schedule, Utc::now()) {
            j.next_run = next;
        }
    }
    save_jobs(workspace, &jobs)
}

pub fn record_run(
    workspace: &std::path::Path,
    job_id: &str,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: chrono::DateTime<chrono::Utc>,
    status: &str,
    output: Option<&str>,
    duration_ms: i64,
    max_history: usize,
) -> Result<()> {
    let mut runs = load_runs(workspace)?;
    let next_id = runs.iter().map(|r| r.id).max().unwrap_or(0) + 1;

    runs.push(CronRun {
        id: next_id,
        job_id: job_id.to_string(),
        started_at,
        finished_at,
        status: status.to_string(),
        output: output.map(|s| s.to_string()),
        duration_ms: Some(duration_ms),
    });

    // Prune old runs for this job
    let job_runs: Vec<&CronRun> = runs.iter().filter(|r| r.job_id == job_id).collect();
    if job_runs.len() > max_history {
        let cutoff_id = job_runs[job_runs.len() - max_history].id;
        runs.retain(|r| r.job_id != job_id || r.id >= cutoff_id);
    }

    save_runs(workspace, &runs)
}

pub fn list_runs(workspace: &std::path::Path, job_id: &str, limit: usize) -> Result<Vec<CronRun>> {
    let runs = load_runs(workspace)?;
    let mut job_runs: Vec<CronRun> = runs.into_iter().filter(|r| r.job_id == job_id).collect();
    job_runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    job_runs.truncate(limit);
    Ok(job_runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn workspace(tmp: &TempDir) -> std::path::PathBuf {
        let ws = tmp.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        ws
    }

    #[test]
    fn add_list_remove_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        let schedule = Schedule::Cron {
            expr: "*/5 * * * *".into(),
            tz: None,
        };
        let job = add_shell_job(&ws, None, schedule, "echo ok").unwrap();
        assert_eq!(list_jobs(&ws).unwrap().len(), 1);
        remove_job(&ws, &job.id).unwrap();
        assert!(list_jobs(&ws).unwrap().is_empty());
    }

    #[test]
    fn due_jobs_filters_by_time() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        let schedule = Schedule::Cron {
            expr: "* * * * *".into(),
            tz: None,
        };
        let _job = add_shell_job(&ws, None, schedule, "echo due").unwrap();
        let due_now = due_jobs(&ws, Utc::now()).unwrap();
        assert!(due_now.is_empty());
        let far_future = Utc::now() + chrono::Duration::days(365);
        let due_future = due_jobs(&ws, far_future).unwrap();
        assert_eq!(due_future.len(), 1);
    }
}
