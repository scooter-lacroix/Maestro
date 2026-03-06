//! Schedule parsing and next-run calculation

use crate::cron::Schedule;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cron::Schedule as CronExprSchedule;
use std::str::FromStr;

pub fn next_run_for_schedule(schedule: &Schedule, from: DateTime<Utc>) -> Result<DateTime<Utc>> {
    match schedule {
        Schedule::Cron { expr, tz } => {
            let normalized = normalize_expression(expr)?;
            let cron_schedule = CronExprSchedule::from_str(&normalized)
                .with_context(|| format!("Invalid cron expression: {expr}"))?;

            if let Some(tz_name) = tz {
                let timezone = chrono_tz::Tz::from_str(tz_name)
                    .with_context(|| format!("Invalid IANA timezone: {tz_name}"))?;
                let localized = from.with_timezone(&timezone);
                let next = cron_schedule
                    .after(&localized)
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No future occurrence for: {expr}"))?;
                Ok(next.with_timezone(&Utc))
            } else {
                cron_schedule
                    .after(&from)
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No future occurrence for: {expr}"))
            }
        }
        Schedule::At { at } => Ok(*at),
        Schedule::Every { every_ms } => {
            if *every_ms == 0 {
                anyhow::bail!("every_ms must be > 0");
            }
            let ms = i64::try_from(*every_ms).context("every_ms too large")?;
            from.checked_add_signed(ChronoDuration::milliseconds(ms))
                .ok_or_else(|| anyhow::anyhow!("every_ms overflowed DateTime"))
        }
    }
}

pub fn validate_schedule(schedule: &Schedule, now: DateTime<Utc>) -> Result<()> {
    match schedule {
        Schedule::Cron { expr, .. } => {
            let _ = normalize_expression(expr)?;
            let _ = next_run_for_schedule(schedule, now)?;
            Ok(())
        }
        Schedule::At { at } => {
            if *at <= now {
                anyhow::bail!("'at' must be in the future");
            }
            Ok(())
        }
        Schedule::Every { every_ms } => {
            if *every_ms == 0 {
                anyhow::bail!("every_ms must be > 0");
            }
            Ok(())
        }
    }
}

pub fn schedule_expression(schedule: &Schedule) -> Option<String> {
    match schedule {
        Schedule::Cron { expr, .. } => Some(expr.clone()),
        _ => None,
    }
}

pub fn normalize_expression(expr: &str) -> Result<String> {
    let expr = expr.trim();
    let fields = expr.split_whitespace().count();
    match fields {
        5 => Ok(format!("0 {expr}")),
        6 | 7 => Ok(expr.to_string()),
        _ => anyhow::bail!("Invalid cron expression: {expr} (expected 5-7 fields, got {fields})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_five_field_expression() {
        let result = normalize_expression("*/5 * * * *").unwrap();
        assert_eq!(result, "0 */5 * * * *");
    }

    #[test]
    fn next_run_every() {
        let now = Utc::now();
        let schedule = Schedule::Every { every_ms: 60_000 };
        let next = next_run_for_schedule(&schedule, now).unwrap();
        assert!(next > now);
    }
}
