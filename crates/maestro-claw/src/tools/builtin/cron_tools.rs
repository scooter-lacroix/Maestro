//! Cron management tools for scheduling tasks from the tool interface.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use super::shell::{CommandRiskLevel, ShellTool};
use crate::tools::{Tool, ToolOutput};

pub struct CronAddTool {
    workspace_dir: PathBuf,
}

impl CronAddTool {
    pub fn new(workspace_dir: &Path) -> Self {
        Self {
            workspace_dir: workspace_dir.to_path_buf(),
        }
    }
}

#[async_trait]
impl Tool for CronAddTool {
    fn name(&self) -> &str {
        "cron_add"
    }

    fn description(&self) -> &str {
        "Schedule a recurring task. Provide a cron expression and a shell command."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "required": ["expression", "command"],
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Cron expression such as '*/5 * * * *'"
                },
                "command": {
                    "type": "string",
                    "description": "Shell command to execute on schedule"
                },
                "name": {
                    "type": "string",
                    "description": "Optional human-readable job name"
                }
            }
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        let expression = arguments["expression"].as_str().unwrap_or("");
        let command = arguments["command"].as_str().unwrap_or("");
        let name = arguments["name"].as_str().map(ToOwned::to_owned);

        if expression.is_empty() || command.is_empty() {
            return ToolOutput::error("Both 'expression' and 'command' are required".into());
        }

        match ShellTool::classify_command(command) {
            CommandRiskLevel::Blocked => {
                return ToolOutput::error("Blocked shell commands cannot be scheduled".into());
            }
            CommandRiskLevel::Dangerous => {
                return ToolOutput::error(
                    "Dangerous shell commands cannot be scheduled without explicit approval".into(),
                );
            }
            CommandRiskLevel::Safe | CommandRiskLevel::Moderate => {}
        }

        let schedule = crate::cron::Schedule::Cron {
            expr: expression.to_string(),
            tz: None,
        };

        match crate::cron::add_shell_job(&self.workspace_dir, name, schedule, command) {
            Ok(job) => ToolOutput::success(format!(
                "Scheduled cron job {}. Next run: {}",
                job.id,
                job.next_run.to_rfc3339()
            )),
            Err(error) => ToolOutput::error(format!("Failed to schedule job: {error}")),
        }
    }
}

pub struct CronListTool {
    workspace_dir: PathBuf,
}

impl CronListTool {
    pub fn new(workspace_dir: &Path) -> Self {
        Self {
            workspace_dir: workspace_dir.to_path_buf(),
        }
    }
}

#[async_trait]
impl Tool for CronListTool {
    fn name(&self) -> &str {
        "cron_list"
    }

    fn description(&self) -> &str {
        "List all scheduled cron jobs with status and next run time."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _arguments: JsonValue) -> ToolOutput {
        match crate::cron::list_jobs(&self.workspace_dir) {
            Ok(jobs) => {
                if jobs.is_empty() {
                    return ToolOutput::success("No scheduled jobs.".into());
                }

                let mut lines = vec![format!("{} scheduled job(s):", jobs.len())];
                for job in &jobs {
                    let status = job.last_status.as_deref().unwrap_or("pending");
                    let summary = if job.command.is_empty() {
                        job.prompt.as_deref().unwrap_or("(agent)")
                    } else {
                        &job.command
                    };
                    lines.push(format!(
                        "- {} | {} | next={} | status={}",
                        job.id,
                        summary,
                        job.next_run.to_rfc3339(),
                        status
                    ));
                }

                ToolOutput::success(lines.join("\n"))
            }
            Err(error) => ToolOutput::error(format!("Failed to list jobs: {error}")),
        }
    }
}

pub struct CronRemoveTool {
    workspace_dir: PathBuf,
}

impl CronRemoveTool {
    pub fn new(workspace_dir: &Path) -> Self {
        Self {
            workspace_dir: workspace_dir.to_path_buf(),
        }
    }
}

#[async_trait]
impl Tool for CronRemoveTool {
    fn name(&self) -> &str {
        "cron_remove"
    }

    fn description(&self) -> &str {
        "Remove a scheduled cron job by ID."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Job ID to remove"
                }
            }
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        let id = arguments["id"].as_str().unwrap_or("");
        if id.is_empty() {
            return ToolOutput::error("'id' is required".into());
        }

        match crate::cron::remove_job(&self.workspace_dir, id) {
            Ok(()) => ToolOutput::success(format!("Removed cron job {id}")),
            Err(error) => ToolOutput::error(format!("Failed to remove job: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn cron_add_rejects_missing_expression() {
        let tmp = TempDir::new().unwrap();
        let tool = CronAddTool::new(tmp.path());

        let result = tool
            .execute(serde_json::json!({
                "expression": "",
                "command": "echo hi"
            }))
            .await;

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn cron_list_reports_empty_state() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let tool = CronListTool::new(&workspace);

        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.is_error);
        assert_eq!(result.content, "No scheduled jobs.");
    }

    #[tokio::test]
    async fn cron_add_rejects_dangerous_commands() {
        let tmp = TempDir::new().unwrap();
        let tool = CronAddTool::new(tmp.path());

        let result = tool
            .execute(serde_json::json!({
                "expression": "*/5 * * * *",
                "command": "rm file.txt"
            }))
            .await;

        assert!(result.is_error);
        assert!(result.content.contains("Dangerous"));
    }
}
