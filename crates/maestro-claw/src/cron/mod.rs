//! MaestroClaw cron scheduler system

pub mod schedule;
pub mod scheduler;
pub mod store;
pub mod types;

pub use schedule::{
    next_run_for_schedule, normalize_expression, schedule_expression, validate_schedule,
};
pub use store::{
    add_agent_job, add_shell_job, due_jobs, get_job, list_jobs, list_runs, record_run, remove_job,
    reschedule_after_run, update_job,
};
pub use types::{CronJob, CronJobPatch, CronRun, JobType, Schedule};

use anyhow::Result;
use std::path::Path;

pub fn handle_command(command: &str, args: &[&str], workspace: &Path) -> Result<()> {
    match command {
        "list" => {
            let jobs = list_jobs(workspace)?;
            if jobs.is_empty() {
                println!("No scheduled tasks.");
                return Ok(());
            }
            println!("🕒 Scheduled jobs ({}):", jobs.len());
            for job in jobs {
                let last = job.last_run.map_or("never".into(), |d| d.to_rfc3339());
                let status = job.last_status.unwrap_or_else(|| "n/a".into());
                println!(
                    "- {} | {:?} | next={} | last={} ({})",
                    job.id,
                    job.schedule,
                    job.next_run.to_rfc3339(),
                    last,
                    status
                );
                if !job.command.is_empty() {
                    println!("    cmd: {}", job.command);
                }
                if let Some(p) = &job.prompt {
                    println!("    prompt: {p}");
                }
            }
            Ok(())
        }
        "add" if args.len() >= 2 => {
            let schedule = Schedule::Cron {
                expr: args[0].to_string(),
                tz: None,
            };
            let command_str = args[1..].join(" ");
            let job = add_shell_job(workspace, None, schedule, &command_str)?;
            println!("✅ Added cron job {}", job.id);
            Ok(())
        }
        "remove" if !args.is_empty() => remove_job(workspace, args[0]),
        _ => {
            println!("Usage: maestro claw cron [list|add|remove]");
            Ok(())
        }
    }
}
