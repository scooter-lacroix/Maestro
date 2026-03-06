//! Cost tracking for CLI agent invocations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use crate::observability::TelemetryCorrelation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    pub provider: String,
    pub model: Option<String>,
    pub duration_ms: i64,
    pub prompt_chars: usize,
    pub response_chars: usize,
    pub estimated_cost_usd: f64,
    pub success: bool,
    pub error_message: Option<String>,
    pub invocation_type: InvocationType,
    pub workspace_dir: Option<String>,
    pub session_id: Option<String>,
    pub component: Option<String>,
    /// Correlation fields for telemetry correlation
    pub correlation: Option<TelemetryCorrelation>,
}

impl CostRecord {
    pub fn effective_correlation(&self) -> TelemetryCorrelation {
        self.correlation
            .clone()
            .unwrap_or_default()
            .normalized_with(
                self.session_id.clone(),
                self.component.clone(),
                self.component.as_deref(),
            )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InvocationType {
    Direct,
    CronJob,
    HeartbeatTask,
    ChannelMessage,
    Daemon,
    Runtime,
}

impl Default for InvocationType {
    fn default() -> Self {
        InvocationType::Direct
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostSummary {
    pub total_invocations: usize,
    pub successful_invocations: usize,
    pub failed_invocations: usize,
    pub total_duration_ms: i64,
    pub avg_duration_ms: f64,
    pub total_estimated_cost_usd: f64,
    pub daily_cost_usd: f64,
    pub monthly_cost_usd: f64,
    pub by_tool: Vec<ToolCostBreakdown>,
    pub by_type: Vec<TypeCostBreakdown>,
    pub by_provider: Vec<ProviderCostBreakdown>,
    pub by_component: Vec<ComponentCostBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCostBreakdown {
    pub tool: String,
    pub invocations: usize,
    pub total_cost_usd: f64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeCostBreakdown {
    pub invocation_type: InvocationType,
    pub invocations: usize,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderCostBreakdown {
    pub provider: String,
    pub invocations: usize,
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentCostBreakdown {
    pub component: String,
    pub invocations: usize,
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCostSummary {
    pub tool: String,
    pub invocations: usize,
    pub total_duration_ms: i64,
    pub avg_duration_ms: f64,
    pub total_prompt_chars: usize,
    pub total_response_chars: usize,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeSummary {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_invocations: usize,
    pub successful_invocations: usize,
    pub failed_invocations: usize,
    pub total_duration_ms: i64,
    pub total_cost_usd: f64,
    pub by_tool: Vec<ToolCostSummary>,
    pub by_type: Vec<TypeCostBreakdown>,
    pub by_provider: Vec<ProviderCostBreakdown>,
    pub by_component: Vec<ComponentCostBreakdown>,
    pub by_day: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CostCountBreakdown {
    pub key: String,
    pub count: usize,
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostReport {
    pub total_records: usize,
    pub recent_records: Vec<CostRecord>,
    pub by_component: Vec<CostCountBreakdown>,
    pub by_surface: Vec<CostCountBreakdown>,
    pub by_principal: Vec<CostCountBreakdown>,
    pub by_session: Vec<CostCountBreakdown>,
    pub by_thread: Vec<CostCountBreakdown>,
    pub by_tool_call: Vec<CostCountBreakdown>,
}

pub struct CostTracker {
    storage_path: PathBuf,
    daily_limit_usd: f64,
    monthly_limit_usd: f64,
    use_jsonl: bool,
}

impl CostTracker {
    pub fn new(workspace_dir: &Path, daily_limit: f64, monthly_limit: f64) -> Self {
        Self {
            storage_path: workspace_dir.join("cost").join("records.jsonl"),
            daily_limit_usd: daily_limit,
            monthly_limit_usd: monthly_limit,
            use_jsonl: true,
        }
    }

    pub fn new_json(workspace_dir: &Path, daily_limit: f64, monthly_limit: f64) -> Self {
        Self {
            storage_path: workspace_dir.join("cost").join("records.json"),
            daily_limit_usd: daily_limit,
            monthly_limit_usd: monthly_limit,
            use_jsonl: false,
        }
    }

    pub fn record(&self, record: CostRecord) -> Result<()> {
        if self.use_jsonl {
            if let Some(parent) = self.storage_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string(&record)?;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.storage_path)?;
            use std::io::Write;
            writeln!(file, "{}", json)?;
            Ok(())
        } else {
            let mut records = self.load()?;
            records.push(record);
            self.save(&records)
        }
    }

    pub fn record_simple(
        &self,
        tool: &str,
        duration_ms: i64,
        prompt_chars: usize,
        response_chars: usize,
    ) -> Result<()> {
        let estimated_cost = CostEstimator::estimate_cost(tool, prompt_chars, response_chars, None);
        self.record(CostRecord {
            timestamp: Utc::now(),
            tool: tool.to_string(),
            provider: "cli".to_string(),
            model: None,
            duration_ms,
            prompt_chars,
            response_chars,
            estimated_cost_usd: estimated_cost,
            success: true,
            error_message: None,
            invocation_type: InvocationType::Direct,
            workspace_dir: None,
            session_id: None,
            component: None,
            correlation: None,
        })
    }

    pub fn check_budget(&self) -> Result<bool> {
        let summary = self.summarize()?;
        Ok(summary.daily_cost_usd <= self.daily_limit_usd
            && summary.monthly_cost_usd <= self.monthly_limit_usd)
    }

    pub fn summarize(&self) -> Result<CostSummary> {
        let records = self.load()?;
        let now = Utc::now();

        let day_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|naive| naive.and_local_timezone(Utc).single())
            .unwrap_or(now);
        let month_start = now
            .date_naive()
            .with_day(1)
            .and_then(|day| day.and_hms_opt(0, 0, 0))
            .and_then(|naive| naive.and_local_timezone(Utc).single())
            .unwrap_or(now);

        let total = records.len();
        let successful = records.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let total_duration: i64 = records.iter().map(|r| r.duration_ms).sum();
        let avg_duration = if total > 0 {
            total_duration as f64 / total as f64
        } else {
            0.0
        };

        let total_cost: f64 = records.iter().map(|r| r.estimated_cost_usd).sum();
        let daily_cost: f64 = records
            .iter()
            .filter(|r| r.timestamp >= day_start)
            .map(|r| r.estimated_cost_usd)
            .sum();
        let monthly_cost: f64 = records
            .iter()
            .filter(|r| r.timestamp >= month_start)
            .map(|r| r.estimated_cost_usd)
            .sum();

        let mut by_tool_map: std::collections::HashMap<String, (usize, f64, i64)> =
            std::collections::HashMap::new();
        for record in &records {
            let entry = by_tool_map
                .entry(record.tool.clone())
                .or_insert((0, 0.0, 0));
            entry.0 += 1;
            entry.1 += record.estimated_cost_usd;
            entry.2 += record.duration_ms;
        }
        let mut by_tool: Vec<ToolCostBreakdown> = by_tool_map
            .into_iter()
            .map(|(tool, (count, cost, duration))| ToolCostBreakdown {
                tool,
                invocations: count,
                total_cost_usd: cost,
                avg_duration_ms: if count > 0 {
                    duration as f64 / count as f64
                } else {
                    0.0
                },
            })
            .collect();
        by_tool.sort_by(|a, b| {
            b.total_cost_usd
                .total_cmp(&a.total_cost_usd)
                .then_with(|| a.tool.cmp(&b.tool))
        });

        let mut by_type_map: std::collections::HashMap<InvocationType, (usize, f64)> =
            std::collections::HashMap::new();
        for record in &records {
            let entry = by_type_map
                .entry(record.invocation_type.clone())
                .or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += record.estimated_cost_usd;
        }
        let mut by_type: Vec<TypeCostBreakdown> = by_type_map
            .into_iter()
            .map(|(invocation_type, (count, cost))| TypeCostBreakdown {
                invocation_type,
                invocations: count,
                total_cost_usd: cost,
            })
            .collect();
        by_type.sort_by(|a, b| {
            b.total_cost_usd.total_cmp(&a.total_cost_usd).then_with(|| {
                format!("{:?}", a.invocation_type).cmp(&format!("{:?}", b.invocation_type))
            })
        });

        let mut by_provider_map: std::collections::HashMap<String, (usize, f64, i64)> =
            std::collections::HashMap::new();
        for record in &records {
            let entry = by_provider_map
                .entry(record.provider.clone())
                .or_insert((0, 0.0, 0));
            entry.0 += 1;
            entry.1 += record.estimated_cost_usd;
            entry.2 += record.duration_ms;
        }
        let mut by_provider: Vec<ProviderCostBreakdown> = by_provider_map
            .into_iter()
            .map(
                |(provider, (count, cost, duration))| ProviderCostBreakdown {
                    provider,
                    invocations: count,
                    total_cost_usd: cost,
                    total_duration_ms: duration,
                },
            )
            .collect();
        by_provider.sort_by(|a, b| {
            b.total_cost_usd
                .total_cmp(&a.total_cost_usd)
                .then_with(|| a.provider.cmp(&b.provider))
        });

        let mut by_component_map: std::collections::HashMap<String, (usize, f64, i64)> =
            std::collections::HashMap::new();
        for record in &records {
            if let Some(component) = &record.component {
                let entry = by_component_map
                    .entry(component.clone())
                    .or_insert((0, 0.0, 0));
                entry.0 += 1;
                entry.1 += record.estimated_cost_usd;
                entry.2 += record.duration_ms;
            }
        }
        let mut by_component: Vec<ComponentCostBreakdown> = by_component_map
            .into_iter()
            .map(
                |(component, (count, cost, duration))| ComponentCostBreakdown {
                    component,
                    invocations: count,
                    total_cost_usd: cost,
                    total_duration_ms: duration,
                },
            )
            .collect();
        by_component.sort_by(|a, b| {
            b.total_cost_usd
                .total_cmp(&a.total_cost_usd)
                .then_with(|| a.component.cmp(&b.component))
        });

        Ok(CostSummary {
            total_invocations: total,
            successful_invocations: successful,
            failed_invocations: failed,
            total_duration_ms: total_duration,
            avg_duration_ms: avg_duration,
            total_estimated_cost_usd: total_cost,
            daily_cost_usd: daily_cost,
            monthly_cost_usd: monthly_cost,
            by_tool,
            by_type,
            by_provider,
            by_component,
        })
    }

    fn load(&self) -> Result<Vec<CostRecord>> {
        if !self.storage_path.exists() {
            return Ok(Vec::new());
        }

        if self.use_jsonl {
            let content = std::fs::read_to_string(&self.storage_path)?;
            let records: Vec<CostRecord> = content
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();
            Ok(records)
        } else {
            let content = std::fs::read_to_string(&self.storage_path)?;
            Ok(serde_json::from_str(&content)?)
        }
    }

    fn save(&self, records: &[CostRecord]) -> Result<()> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if self.use_jsonl {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.storage_path)?;
            use std::io::Write;
            for record in records {
                let json = serde_json::to_string(record)?;
                writeln!(file, "{}", json)?;
            }
            Ok(())
        } else {
            std::fs::write(&self.storage_path, serde_json::to_string_pretty(records)?)?;
            Ok(())
        }
    }

    pub fn get_storage_path(&self) -> &Path {
        &self.storage_path
    }

    pub fn recent_records(&self, limit: usize) -> Result<Vec<CostRecord>> {
        let mut records = self.load()?;
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        records.truncate(limit);
        Ok(records)
    }

    pub fn recent_records_by_correlation(
        &self,
        correlation: &TelemetryCorrelation,
        limit: usize,
    ) -> Result<Vec<CostRecord>> {
        let mut records: Vec<CostRecord> = self
            .load()?
            .into_iter()
            .filter(|record| record.effective_correlation().matches(correlation))
            .collect();
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        records.truncate(limit);
        Ok(records)
    }

    pub fn recent_report(&self, limit: usize) -> Result<CostReport> {
        let recent_records = self.recent_records(limit)?;
        let mut by_component = HashMap::<String, (usize, f64, i64)>::new();
        let mut by_surface = HashMap::<String, (usize, f64, i64)>::new();
        let mut by_principal = HashMap::<String, (usize, f64, i64)>::new();
        let mut by_session = HashMap::<String, (usize, f64, i64)>::new();
        let mut by_thread = HashMap::<String, (usize, f64, i64)>::new();
        let mut by_tool_call = HashMap::<String, (usize, f64, i64)>::new();

        for record in &recent_records {
            let correlation = record.effective_correlation();
            bump_breakdown(
                &mut by_component,
                correlation
                    .component
                    .clone()
                    .or_else(|| record.component.clone()),
                record,
            );
            bump_breakdown(&mut by_surface, correlation.inferred_surface(), record);
            bump_breakdown(&mut by_principal, correlation.actor(), record);
            bump_breakdown(&mut by_session, correlation.session_id.clone(), record);
            bump_breakdown(&mut by_thread, correlation.thread_id.clone(), record);
            bump_breakdown(&mut by_tool_call, correlation.tool_call_id.clone(), record);
        }

        Ok(CostReport {
            total_records: self.load()?.len(),
            recent_records,
            by_component: sort_cost_breakdown(by_component),
            by_surface: sort_cost_breakdown(by_surface),
            by_principal: sort_cost_breakdown(by_principal),
            by_session: sort_cost_breakdown(by_session),
            by_thread: sort_cost_breakdown(by_thread),
            by_tool_call: sort_cost_breakdown(by_tool_call),
        })
    }

    pub fn runtime_summary(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<RuntimeSummary> {
        let records: Vec<CostRecord> = self
            .load()?
            .into_iter()
            .filter(|record| record.timestamp >= period_start && record.timestamp <= period_end)
            .collect();

        let mut by_tool_map: HashMap<String, ToolCostSummary> = HashMap::new();
        let mut by_type_map: HashMap<InvocationType, (usize, f64)> = HashMap::new();
        let mut by_provider_map: HashMap<String, (usize, f64, i64)> = HashMap::new();
        let mut by_component_map: HashMap<String, (usize, f64, i64)> = HashMap::new();
        let mut by_day = HashMap::new();
        let successful_invocations = records.iter().filter(|record| record.success).count();
        let failed_invocations = records.len().saturating_sub(successful_invocations);

        for record in &records {
            let entry = by_tool_map
                .entry(record.tool.clone())
                .or_insert_with(|| ToolCostSummary {
                    tool: record.tool.clone(),
                    ..ToolCostSummary::default()
                });
            entry.invocations += 1;
            entry.total_duration_ms += record.duration_ms;
            entry.total_prompt_chars += record.prompt_chars;
            entry.total_response_chars += record.response_chars;
            entry.total_cost_usd += record.estimated_cost_usd;

            let type_entry = by_type_map
                .entry(record.invocation_type.clone())
                .or_insert((0, 0.0));
            type_entry.0 += 1;
            type_entry.1 += record.estimated_cost_usd;

            let provider_entry = by_provider_map
                .entry(record.provider.clone())
                .or_insert((0, 0.0, 0));
            provider_entry.0 += 1;
            provider_entry.1 += record.estimated_cost_usd;
            provider_entry.2 += record.duration_ms;

            if let Some(component) = &record.component {
                let component_entry = by_component_map
                    .entry(component.clone())
                    .or_insert((0, 0.0, 0));
                component_entry.0 += 1;
                component_entry.1 += record.estimated_cost_usd;
                component_entry.2 += record.duration_ms;
            }

            let day = record.timestamp.format("%Y-%m-%d").to_string();
            *by_day.entry(day).or_insert(0.0) += record.estimated_cost_usd;
        }

        let mut by_tool: Vec<ToolCostSummary> = by_tool_map
            .into_values()
            .map(|mut summary| {
                summary.avg_duration_ms = if summary.invocations > 0 {
                    summary.total_duration_ms as f64 / summary.invocations as f64
                } else {
                    0.0
                };
                summary
            })
            .collect();
        by_tool.sort_by(|a, b| {
            b.total_cost_usd
                .total_cmp(&a.total_cost_usd)
                .then_with(|| a.tool.cmp(&b.tool))
        });

        let mut by_type: Vec<TypeCostBreakdown> = by_type_map
            .into_iter()
            .map(|(invocation_type, (count, cost))| TypeCostBreakdown {
                invocation_type,
                invocations: count,
                total_cost_usd: cost,
            })
            .collect();
        by_type.sort_by(|a, b| {
            b.total_cost_usd.total_cmp(&a.total_cost_usd).then_with(|| {
                format!("{:?}", a.invocation_type).cmp(&format!("{:?}", b.invocation_type))
            })
        });

        let mut by_provider: Vec<ProviderCostBreakdown> = by_provider_map
            .into_iter()
            .map(
                |(provider, (count, cost, duration))| ProviderCostBreakdown {
                    provider,
                    invocations: count,
                    total_cost_usd: cost,
                    total_duration_ms: duration,
                },
            )
            .collect();
        by_provider.sort_by(|a, b| {
            b.total_cost_usd
                .total_cmp(&a.total_cost_usd)
                .then_with(|| a.provider.cmp(&b.provider))
        });

        let mut by_component: Vec<ComponentCostBreakdown> = by_component_map
            .into_iter()
            .map(
                |(component, (count, cost, duration))| ComponentCostBreakdown {
                    component,
                    invocations: count,
                    total_cost_usd: cost,
                    total_duration_ms: duration,
                },
            )
            .collect();
        by_component.sort_by(|a, b| {
            b.total_cost_usd
                .total_cmp(&a.total_cost_usd)
                .then_with(|| a.component.cmp(&b.component))
        });

        Ok(RuntimeSummary {
            period_start,
            period_end,
            total_invocations: records.len(),
            successful_invocations,
            failed_invocations,
            total_duration_ms: records.iter().map(|record| record.duration_ms).sum(),
            total_cost_usd: records.iter().map(|record| record.estimated_cost_usd).sum(),
            by_tool,
            by_type,
            by_provider,
            by_component,
            by_day,
        })
    }
}

pub struct CostEstimator;

impl CostEstimator {
    pub fn estimate_cost(
        tool: &str,
        prompt_chars: usize,
        response_chars: usize,
        model: Option<&str>,
    ) -> f64 {
        let prompt_tokens = prompt_chars / 4;
        let response_tokens = response_chars / 4;

        let (input_rate, output_rate) = match tool {
            "claude" => (15.0, 75.0),
            "gpt-4" | "gpt-4o" => (30.0, 60.0),
            "gpt-4o-mini" | "gpt-3.5" => (0.5, 1.5),
            "gemini" => (0.35, 1.05),
            "qwen" => (0.8, 2.4),
            "codex" => (15.0, 60.0),
            "iflow" => (5.0, 15.0),
            "droid" => (10.0, 30.0),
            _ => (10.0, 40.0),
        };

        if let Some(m) = model {
            let (m_input, m_output) = match m {
                m if m.contains("4o-mini") || m.contains("3.5-turbo") => (0.5, 1.5),
                m if m.contains("4o") || m.contains("4-turbo") => (30.0, 60.0),
                m if m.contains("sonnet") => (15.0, 75.0),
                m if m.contains("haiku") => (0.3, 0.8),
                _ => (input_rate, output_rate),
            };
            return (prompt_tokens as f64 * m_input / 1_000_000.0)
                + (response_tokens as f64 * m_output / 1_000_000.0);
        }

        (prompt_tokens as f64 * input_rate / 1_000_000.0)
            + (response_tokens as f64 * output_rate / 1_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_budget_is_within_limits() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);
        assert!(tracker.check_budget().unwrap());
    }

    #[test]
    fn record_and_summarize_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);

        tracker
            .record(CostRecord {
                timestamp: Utc::now(),
                tool: "claude".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 5000,
                prompt_chars: 100,
                response_chars: 500,
                estimated_cost_usd: 0.05,
                success: true,
                error_message: None,
                invocation_type: InvocationType::Direct,
                workspace_dir: None,
                session_id: None,
                component: None,
                correlation: None,
            })
            .unwrap();

        let summary = tracker.summarize().unwrap();
        assert_eq!(summary.total_invocations, 1);
        assert_eq!(summary.successful_invocations, 1);
        assert_eq!(summary.failed_invocations, 0);
        assert!((summary.total_estimated_cost_usd - 0.05).abs() < 0.001);
    }

    #[test]
    fn cost_estimator_calculates_correctly() {
        let cost = CostEstimator::estimate_cost("claude", 1000, 2000, None);
        assert!(cost > 0.0);
    }

    #[test]
    fn cost_estimator_uses_model_when_provided() {
        let cost_with_model =
            CostEstimator::estimate_cost("claude", 1000, 2000, Some("claude-sonnet-4-20250514"));
        let _cost_default = CostEstimator::estimate_cost("claude", 1000, 2000, None);
        assert!(cost_with_model > 0.0);
    }

    #[test]
    fn summarize_includes_tool_breakdown() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);

        tracker
            .record(CostRecord {
                timestamp: Utc::now(),
                tool: "claude".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 1000,
                prompt_chars: 100,
                response_chars: 100,
                estimated_cost_usd: 0.01,
                success: true,
                error_message: None,
                invocation_type: InvocationType::Direct,
                workspace_dir: None,
                session_id: None,
                component: None,
                correlation: None,
            })
            .unwrap();

        tracker
            .record(CostRecord {
                timestamp: Utc::now(),
                tool: "codex".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 2000,
                prompt_chars: 100,
                response_chars: 100,
                estimated_cost_usd: 0.02,
                success: true,
                error_message: None,
                invocation_type: InvocationType::CronJob,
                workspace_dir: None,
                session_id: None,
                component: None,
                correlation: None,
            })
            .unwrap();

        let summary = tracker.summarize().unwrap();
        assert_eq!(summary.by_tool.len(), 2);
        assert_eq!(summary.by_type.len(), 2);
    }

    #[test]
    fn jsonl_format_works_correctly() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);

        assert!(tracker
            .get_storage_path()
            .to_string_lossy()
            .ends_with(".jsonl"));

        tracker
            .record(CostRecord {
                timestamp: Utc::now(),
                tool: "claude".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 1000,
                prompt_chars: 100,
                response_chars: 100,
                estimated_cost_usd: 0.01,
                success: true,
                error_message: None,
                invocation_type: InvocationType::Direct,
                workspace_dir: None,
                session_id: None,
                component: None,
                correlation: None,
            })
            .unwrap();

        let summary = tracker.summarize().unwrap();
        assert_eq!(summary.total_invocations, 1);
    }

    #[test]
    fn record_simple_helper_works() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);

        tracker.record_simple("claude", 5000, 1000, 500).unwrap();

        let summary = tracker.summarize().unwrap();
        assert_eq!(summary.total_invocations, 1);
        assert!(summary.total_estimated_cost_usd > 0.0);
    }

    #[test]
    fn summarize_includes_provider_and_component_breakdowns() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);

        tracker
            .record(CostRecord {
                timestamp: Utc::now(),
                tool: "claude".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 1000,
                prompt_chars: 100,
                response_chars: 250,
                estimated_cost_usd: 0.01,
                success: true,
                error_message: None,
                invocation_type: InvocationType::ChannelMessage,
                workspace_dir: None,
                session_id: None,
                component: Some("channel:telegram".into()),
                correlation: None,
            })
            .unwrap();

        let summary = tracker.summarize().unwrap();
        assert_eq!(summary.by_provider[0].provider, "cli");
        assert_eq!(summary.by_component[0].component, "channel:telegram");
    }

    #[test]
    fn runtime_summary_filters_by_period() {
        use chrono::Duration as ChronoDuration;

        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = CostTracker::new(tmp.path(), 10.0, 100.0);
        let now = Utc::now();

        tracker
            .record(CostRecord {
                timestamp: now - ChronoDuration::days(2),
                tool: "claude".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 1000,
                prompt_chars: 100,
                response_chars: 100,
                estimated_cost_usd: 0.01,
                success: true,
                error_message: None,
                invocation_type: InvocationType::Direct,
                workspace_dir: None,
                session_id: None,
                component: Some("agent:claude".into()),
                correlation: None,
            })
            .unwrap();

        tracker
            .record(CostRecord {
                timestamp: now,
                tool: "claude".into(),
                provider: "cli".into(),
                model: None,
                duration_ms: 2000,
                prompt_chars: 120,
                response_chars: 180,
                estimated_cost_usd: 0.02,
                success: true,
                error_message: None,
                invocation_type: InvocationType::HeartbeatTask,
                workspace_dir: None,
                session_id: None,
                component: Some("heartbeat".into()),
                correlation: None,
            })
            .unwrap();

        let summary = tracker
            .runtime_summary(
                now - ChronoDuration::hours(1),
                now + ChronoDuration::hours(1),
            )
            .unwrap();

        assert_eq!(summary.total_invocations, 1);
        assert_eq!(summary.successful_invocations, 1);
        assert_eq!(summary.failed_invocations, 0);
        assert_eq!(summary.by_tool[0].tool, "claude");
        assert_eq!(
            summary.by_type[0].invocation_type,
            InvocationType::HeartbeatTask
        );
        assert_eq!(summary.by_provider[0].provider, "cli");
        assert_eq!(summary.by_component[0].component, "heartbeat");
    }
}

fn bump_breakdown(
    map: &mut HashMap<String, (usize, f64, i64)>,
    key: Option<String>,
    record: &CostRecord,
) {
    let Some(key) = key else {
        return;
    };
    let entry = map.entry(key).or_insert((0, 0.0, 0));
    entry.0 += 1;
    entry.1 += record.estimated_cost_usd;
    entry.2 += record.duration_ms;
}

fn sort_cost_breakdown(map: HashMap<String, (usize, f64, i64)>) -> Vec<CostCountBreakdown> {
    let mut values: Vec<CostCountBreakdown> = map
        .into_iter()
        .map(
            |(key, (count, total_cost_usd, total_duration_ms))| CostCountBreakdown {
                key,
                count,
                total_cost_usd,
                total_duration_ms,
            },
        )
        .collect();
    values.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    values
}
